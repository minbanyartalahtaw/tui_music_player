use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rodio::Source;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

/// FFT window size -- must be a power of two.
const FFT_SIZE: usize = 2048;

/// Number of output frequency bars.
pub const NUM_BARS: usize = 32;

/// Ring-buffer capacity (keeps ~4 FFT frames of samples).
const BUFFER_CAP: usize = FFT_SIZE * 4;

/// Smoothing factor for the decay animation (0.0 = instant, 1.0 = frozen).
const DECAY: f64 = 0.55;

pub type SampleBuffer = Arc<Mutex<VecDeque<f32>>>;

// ─── Source wrapper ──────────────────────────────────────────────────────────

/// Transparent wrapper around any `Source<Item = f32>` that copies every
/// sample into a shared ring-buffer so the FFT thread can read it.
pub struct VisualizerSource<S> {
    inner: S,
    buffer: SampleBuffer,
}

impl<S> VisualizerSource<S> {
    pub fn new(inner: S, buffer: SampleBuffer) -> Self {
        Self { inner, buffer }
    }
}

impl<S: Source<Item = f32>> Iterator for VisualizerSource<S> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        // try_lock so we never block the audio thread
        if let Ok(mut buf) = self.buffer.try_lock() {
            buf.push_back(sample);
            if buf.len() > BUFFER_CAP {
                let excess = buf.len() - BUFFER_CAP;
                buf.drain(..excess);
            }
        }
        Some(sample)
    }
}

impl<S: Source<Item = f32>> Source for VisualizerSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        // Clear captured samples on seek -- position just changed.
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        self.inner.try_seek(pos)
    }
}

// ─── Background spectrum analyser ────────────────────────────────────────────

/// Runs a dedicated thread that periodically grabs samples from the shared
/// ring-buffer, applies a Hann window, runs an FFT, and writes the resulting
/// spectrum bars (normalised 0-100) into shared state that the UI can read
/// without blocking.
pub struct SpectrumAnalyzer {
    sample_buffer: SampleBuffer,
    spectrum: Arc<Mutex<Vec<f64>>>,
    channels: Arc<AtomicU16>,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let sample_buffer: SampleBuffer =
            Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_CAP)));
        let spectrum = Arc::new(Mutex::new(vec![0.0f64; NUM_BARS]));
        let channels = Arc::new(AtomicU16::new(2));
        let running = Arc::new(AtomicBool::new(true));

        let buf = sample_buffer.clone();
        let spec = spectrum.clone();
        let ch = channels.clone();
        let run = running.clone();

        let thread = std::thread::spawn(move || {
            Self::fft_loop(buf, spec, ch, run);
        });

        Self {
            sample_buffer,
            spectrum,
            channels,
            running,
            thread: Some(thread),
        }
    }

    // ── public helpers ───────────────────────────────────────────────────

    /// Returns a clone of the sample ring-buffer handle so `VisualizerSource`
    /// can push samples into it.
    pub fn buffer(&self) -> SampleBuffer {
        self.sample_buffer.clone()
    }

    /// Tell the analyser how many interleaved channels the current source has.
    pub fn set_channels(&self, ch: u16) {
        self.channels.store(ch, Ordering::Relaxed);
    }

    /// Read the latest spectrum bars (each value 0..=100).
    pub fn spectrum(&self) -> Vec<u64> {
        self.spectrum
            .lock()
            .map(|s| s.iter().map(|&v| v.round() as u64).collect())
            .unwrap_or_else(|_| vec![0; NUM_BARS])
    }

    /// Clear both the sample buffer and the spectrum (e.g. on track change).
    pub fn clear(&self) {
        if let Ok(mut buf) = self.sample_buffer.lock() {
            buf.clear();
        }
        if let Ok(mut spec) = self.spectrum.lock() {
            spec.iter_mut().for_each(|v| *v = 0.0);
        }
    }

    // ── background thread ────────────────────────────────────────────────

    fn fft_loop(
        buf: SampleBuffer,
        spec: Arc<Mutex<Vec<f64>>>,
        ch: Arc<AtomicU16>,
        run: Arc<AtomicBool>,
    ) {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Pre-compute Hann window coefficients once.
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();

        let mut prev = vec![0.0f64; NUM_BARS];

        while run.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(30));

            let channels = ch.load(Ordering::Relaxed).max(1) as usize;

            // ── grab the most recent FFT_SIZE * channels samples ─────────
            let raw: Vec<f32> = {
                let Ok(guard) = buf.lock() else {
                    continue;
                };
                let needed = FFT_SIZE * channels;
                if guard.len() < needed {
                    continue;
                }
                let start = guard.len() - needed;
                guard.range(start..).copied().collect()
            };

            // ── mix interleaved channels down to mono ────────────────────
            let mono: Vec<f32> = raw
                .chunks(channels)
                .map(|c| c.iter().sum::<f32>() / c.len() as f32)
                .collect();

            if mono.len() < FFT_SIZE {
                continue;
            }

            // ── apply Hann window → complex buffer ───────────────────────
            let mut fft_buf: Vec<Complex<f32>> = mono[..FFT_SIZE]
                .iter()
                .zip(window.iter())
                .map(|(&s, &w)| Complex::new(s * w, 0.0))
                .collect();

            // ── run FFT in-place ─────────────────────────────────────────
            fft.process(&mut fft_buf);

            // ── magnitudes of positive frequencies ───────────────────────
            let half = FFT_SIZE / 2;
            let magnitudes: Vec<f32> = fft_buf[..half].iter().map(|c| c.norm()).collect();

            // ── map to bars with logarithmic frequency spacing ───────────
            let new_spec: Vec<f64> = (0..NUM_BARS)
                .map(|i| {
                    // Logarithmic bin edges: half^(i/NUM_BARS) .. half^((i+1)/NUM_BARS)
                    let lo =
                        ((half as f64).powf(i as f64 / NUM_BARS as f64)) as usize;
                    let hi =
                        ((half as f64).powf((i + 1) as f64 / NUM_BARS as f64)) as usize;
                    let lo = lo.max(1).min(half - 1);
                    let hi = hi.max(lo + 1).min(half);

                    let sum: f32 = magnitudes[lo..hi].iter().sum();
                    let avg = sum / (hi - lo) as f32;

                    // Convert to dB then normalise into 0..100
                    let db = 20.0 * (avg.max(1e-10)).log10() as f64;
                    let normalized = ((db + 20.0) / 55.0 * 100.0).clamp(0.0, 100.0);

                    // Asymmetric smoothing: rise fast, decay slowly
                    if normalized > prev[i] {
                        prev[i] * 0.2 + normalized * 0.8
                    } else {
                        prev[i] * DECAY + normalized * (1.0 - DECAY)
                    }
                })
                .collect();

            prev.clone_from(&new_spec);

            if let Ok(mut guard) = spec.lock() {
                *guard = new_spec;
            }
        }
    }
}

impl Drop for SpectrumAnalyzer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}
