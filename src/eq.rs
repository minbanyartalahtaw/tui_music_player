//! 3-band equalizer (Bass, Mid, Treble) using peaking biquad filters.
//! Coefficients are recomputed only when the user changes gains.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use biquad::frequency::ToHertz;
use biquad::{Biquad, Coefficients, DirectForm1, Type};
use rodio::Source;

const MIN_CENTI_DB: i32 = -1200;
const MAX_CENTI_DB: i32 = 1200;
const BASS_FREQ: f32 = 120.0;
const MID_FREQ: f32 = 1000.0;
const TREBLE_FREQ: f32 = 8000.0;
const Q: f32 = 1.0;
const COEF_UPDATE_INTERVAL: usize = 256;

/// Gain in dB per band (±12 dB). Stored as centi-dB for lock-free updates.
#[derive(Debug)]
pub struct EqGains {
    bass: AtomicI32,
    mid: AtomicI32,
    treble: AtomicI32,
}

impl Default for EqGains {
    fn default() -> Self {
        Self { bass: AtomicI32::new(0), mid: AtomicI32::new(0), treble: AtomicI32::new(0) }
    }
}

impl EqGains {
    pub fn new() -> Self { Self::default() }

    pub fn bass_db(&self) -> f32 { self.bass.load(Ordering::Relaxed) as f32 * 0.01 }
    pub fn set_bass_db(&self, db: f32) {
        let c = (db.clamp(-12.0, 12.0) * 100.0).round() as i32;
        self.bass.store(c.clamp(MIN_CENTI_DB, MAX_CENTI_DB), Ordering::Relaxed);
    }

    pub fn mid_db(&self) -> f32 { self.mid.load(Ordering::Relaxed) as f32 * 0.01 }
    pub fn set_mid_db(&self, db: f32) {
        let c = (db.clamp(-12.0, 12.0) * 100.0).round() as i32;
        self.mid.store(c.clamp(MIN_CENTI_DB, MAX_CENTI_DB), Ordering::Relaxed);
    }

    pub fn treble_db(&self) -> f32 { self.treble.load(Ordering::Relaxed) as f32 * 0.01 }
    pub fn set_treble_db(&self, db: f32) {
        let c = (db.clamp(-12.0, 12.0) * 100.0).round() as i32;
        self.treble.store(c.clamp(MIN_CENTI_DB, MAX_CENTI_DB), Ordering::Relaxed);
    }

    fn load_centi(&self) -> (i32, i32, i32) {
        (self.bass.load(Ordering::Relaxed), self.mid.load(Ordering::Relaxed), self.treble.load(Ordering::Relaxed))
    }
}

fn make_peaking(sr: f32, freq: f32, gain_db: f32) -> Option<Coefficients<f32>> {
    Coefficients::<f32>::from_params(Type::PeakingEQ(gain_db), (sr as i32).hz(), freq.hz(), Q).ok()
}

fn flat_coeffs(sr: f32, freq: f32) -> Coefficients<f32> {
    make_peaking(sr, freq, 0.0).unwrap_or_else(|| {
        Coefficients::<f32>::from_params(Type::PeakingEQ(0.0), (sr as i32).hz(), freq.hz(), Q).unwrap()
    })
}

fn update_coeffs(sr: f32, b: i32, m: i32, t: i32, bass: &mut DirectForm1<f32>, mid: &mut DirectForm1<f32>, treble: &mut DirectForm1<f32>) {
    if let Some(c) = make_peaking(sr, BASS_FREQ, b as f32 * 0.01) { bass.update_coefficients(c); }
    if let Some(c) = make_peaking(sr, MID_FREQ, m as f32 * 0.01) { mid.update_coefficients(c); }
    if let Some(c) = make_peaking(sr, TREBLE_FREQ, t as f32 * 0.01) { treble.update_coefficients(c); }
}

pub struct EqSource<S> {
    inner: S,
    gains: Arc<EqGains>,
    bass: DirectForm1<f32>,
    mid: DirectForm1<f32>,
    treble: DirectForm1<f32>,
    sample_rate: u32,
    last_gains: (i32, i32, i32),
    n: usize,
}

impl<S: Source<Item = f32>> EqSource<S> {
    pub fn new(inner: S, gains: Arc<EqGains>) -> Self {
        let sr_f = inner.sample_rate() as f32;
        let sr_u = inner.sample_rate();
        let (b, m, t) = gains.load_centi();
        let mut bass = DirectForm1::new(flat_coeffs(sr_f, BASS_FREQ));
        let mut mid = DirectForm1::new(flat_coeffs(sr_f, MID_FREQ));
        let mut treble = DirectForm1::new(flat_coeffs(sr_f, TREBLE_FREQ));
        update_coeffs(sr_f, b, m, t, &mut bass, &mut mid, &mut treble);
        Self {
            inner,
            gains,
            bass,
            mid,
            treble,
            sample_rate: sr_u,
            last_gains: (b, m, t),
            n: 0,
        }
    }

    fn maybe_update(&mut self) {
        self.n += 1;
        if self.n < COEF_UPDATE_INTERVAL { return; }
        self.n = 0;
        let cur = self.gains.load_centi();
        if cur == self.last_gains { return; }
        self.last_gains = cur;
        let sr = self.sample_rate as f32;
        update_coeffs(sr, cur.0, cur.1, cur.2, &mut self.bass, &mut self.mid, &mut self.treble);
    }
}

impl<S: Source<Item = f32>> Iterator for EqSource<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let s = self.inner.next()?;
        self.maybe_update();
        Some(self.treble.run(self.mid.run(self.bass.run(s))))
    }
}

impl<S: Source<Item = f32>> Source for EqSource<S> {
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.bass.reset_state();
        self.mid.reset_state();
        self.treble.reset_state();
        self.inner.try_seek(pos)
    }
}
