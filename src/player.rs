use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::eq::{EqGains, EqSource};
use crate::visualizer::{SpectrumAnalyzer, VisualizerSource};

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    analyzer: SpectrumAnalyzer,
    eq_gains: Arc<EqGains>,
    /// Start offset when playback was started with play_file_from (so position display is correct).
    playback_start: Duration,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&handle)?;
        sink.pause();
        let analyzer = SpectrumAnalyzer::new();
        let eq_gains = Arc::new(EqGains::new());
        Ok(Self {
            _stream: stream,
            handle,
            sink,
            analyzer,
            eq_gains,
            playback_start: Duration::ZERO,
        })
    }

    pub fn play_file(&mut self, path: &Path) -> Result<()> {
        self.play_file_from(path, Duration::ZERO)
    }

    /// Start playback from a given position (e.g. after seek). Uses skip_duration
    /// so seeking works even when Sink::try_seek is not applied to the source chain.
    pub fn play_file_from(&mut self, path: &Path, start: Duration) -> Result<()> {
        self.sink.stop();
        self.sink = Sink::try_new(&self.handle)?;
        self.analyzer.clear();
        self.playback_start = start;

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)?;

        let channels = source.channels();
        self.analyzer.set_channels(channels);

        let source = source.skip_duration(start);
        let converted = source.convert_samples::<f32>();
        let eq_source = EqSource::new(converted, Arc::clone(&self.eq_gains));
        let visualized = VisualizerSource::new(eq_source, self.analyzer.buffer());

        self.sink.append(visualized);
        self.sink.play();
        Ok(())
    }

    pub fn eq_gains(&self) -> &EqGains {
        &self.eq_gains
    }

    pub fn toggle_pause(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn position(&self) -> Duration {
        self.playback_start + self.sink.get_pos()
    }

    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    pub fn set_volume(&self, vol: f32) {
        self.sink.set_volume(vol.clamp(0.0, 1.5));
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }


    pub fn spectrum(&self) -> Vec<u64> {
        self.analyzer.spectrum()
    }

    pub fn get_duration(path: &Path) -> Option<Duration> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).ok()?;
        source.total_duration()
    }
}
