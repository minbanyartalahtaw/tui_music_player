use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use ratatui::widgets::ListState;

use crate::player::Player;

#[derive(Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::All => "All",
            Self::One => "One",
        }
    }
}

pub struct Song {
    pub name: String,
    pub path: PathBuf,
    pub duration: Option<Duration>,
}

/// State for the Equalizer popup: visibility and which band is selected.
#[derive(Debug, Clone)]
pub struct EqState {
    pub popup_open: bool,
    pub selected_band: usize,
}

impl Default for EqState {
    fn default() -> Self {
        Self { popup_open: false, selected_band: 0 }
    }
}

impl EqState {
    pub const BAND_COUNT: usize = 3;
    pub fn band_name(i: usize) -> &'static str {
        match i {
            0 => "Bass",
            1 => "Mid",
            2 => "Treble",
            _ => "?",
        }
    }
}

pub struct App {
    pub songs: Vec<Song>,
    pub selected: usize,
    pub now_playing: Option<usize>,
    pub player: Player,
    pub repeat: RepeatMode,
    pub should_quit: bool,
    pub list_state: ListState,
    pub eq_state: EqState,
}

impl App {
    pub fn new() -> Result<Self> {
        let player = Player::new()?;
        let songs = Self::scan_music();
        let mut list_state = ListState::default();
        if !songs.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            songs,
            selected: 0,
            now_playing: None,
            player,
            repeat: RepeatMode::Off,
            should_quit: false,
            list_state,
            eq_state: EqState::default(),
        })
    }

    fn scan_music() -> Vec<Song> {
        let music_dir = PathBuf::from("music");
        if !music_dir.exists() {
            return Vec::new();
        }

        let extensions = ["mp3", "wav", "ogg", "flac", "m4a", "aac"];
        let Ok(entries) = fs::read_dir(&music_dir) else {
            return Vec::new();
        };

        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| extensions.contains(&ext.to_lowercase().as_str()))
            })
            .collect();

        files.sort_by_key(|e| e.file_name());

        files
            .into_iter()
            .map(|entry| {
                let path = entry.path();
                let name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let duration = Player::get_duration(&path);
                Song {
                    name,
                    path,
                    duration,
                }
            })
            .collect()
    }

    pub fn play_selected(&mut self) {
        if !self.songs.is_empty() {
            self.play_index(self.selected);
        }
    }

    fn play_index(&mut self, idx: usize) {
        if idx < self.songs.len() && self.player.play_file(&self.songs[idx].path).is_ok() {
            self.now_playing = Some(idx);
        }
    }

    pub fn toggle_pause(&self) {
        if self.now_playing.is_some() {
            self.player.toggle_pause();
        }
    }

    pub fn next_track(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        match self.now_playing {
            Some(idx) => {
                let next = if idx + 1 >= self.songs.len() {
                    match self.repeat {
                        RepeatMode::All => 0,
                        _ => return,
                    }
                } else {
                    idx + 1
                };
                self.selected = next;
                self.list_state.select(Some(next));
                self.play_index(next);
            }
            None => self.play_selected(),
        }
    }

    pub fn prev_track(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        match self.now_playing {
            Some(idx) => {
                // If more than 3 seconds in, restart current track
                if self.player.position().as_secs() > 3 {
                    self.play_index(idx);
                    return;
                }
                let prev = if idx == 0 {
                    match self.repeat {
                        RepeatMode::All => self.songs.len() - 1,
                        _ => return,
                    }
                } else {
                    idx - 1
                };
                self.selected = prev;
                self.list_state.select(Some(prev));
                self.play_index(prev);
            }
            None => self.play_selected(),
        }
    }

    pub fn select_next(&mut self) {
        if !self.songs.is_empty() {
            self.selected = (self.selected + 1).min(self.songs.len() - 1);
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn select_prev(&mut self) {
        if !self.songs.is_empty() {
            self.selected = self.selected.saturating_sub(1);
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn volume_up(&self) {
        let vol = self.player.volume();
        self.player.set_volume((vol + 0.05).min(1.5));
    }

    pub fn volume_down(&self) {
        let vol = self.player.volume();
        self.player.set_volume((vol - 0.05).max(0.0));
    }

    pub fn seek_forward(&mut self) {
        let Some(idx) = self.now_playing else { return };
        let pos = self.player.position();
        let new_pos = pos + Duration::from_secs(5);
        let end = self.current_duration().unwrap_or(Duration::MAX);
        let start = new_pos.min(end);
        if self.player.play_file_from(&self.songs[idx].path, start).is_ok() {
            // now_playing unchanged
        }
    }

    pub fn seek_backward(&mut self) {
        let Some(idx) = self.now_playing else { return };
        let pos = self.player.position();
        let start = pos.saturating_sub(Duration::from_secs(5));
        if self.player.play_file_from(&self.songs[idx].path, start).is_ok() {
            // now_playing unchanged
        }
    }

    pub fn toggle_repeat(&mut self) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
    }

    pub fn check_track_end(&mut self) {
        let Some(idx) = self.now_playing else { return };
        if !self.player.is_empty() || self.player.is_paused() {
            return;
        }
        match self.repeat {
            RepeatMode::One => self.play_index(idx),
            RepeatMode::All => {
                let next = (idx + 1) % self.songs.len();
                self.selected = next;
                self.list_state.select(Some(next));
                self.play_index(next);
            }
            RepeatMode::Off => {
                if idx + 1 < self.songs.len() {
                    let next = idx + 1;
                    self.selected = next;
                    self.list_state.select(Some(next));
                    self.play_index(next);
                } else {
                    self.now_playing = None;
                }
            }
        }
    }

    pub fn current_position(&self) -> Duration {
        if self.now_playing.is_some() {
            self.player.position()
        } else {
            Duration::ZERO
        }
    }

    pub fn current_duration(&self) -> Option<Duration> {
        self.now_playing.and_then(|idx| self.songs[idx].duration)
    }

    pub fn is_playing(&self) -> bool {
        self.now_playing.is_some() && !self.player.is_paused()
    }

    pub fn volume_percent(&self) -> u16 {
        (self.player.volume() * 100.0).round() as u16
    }

    pub fn now_playing_name(&self) -> &str {
        self.now_playing
            .map(|idx| self.songs[idx].name.as_str())
            .unwrap_or("Nothing playing")
    }

    pub fn spectrum(&self) -> Vec<u64> {
        self.player.spectrum()
    }

    // ── Equalizer popup and band gains ─────────────────────────────────────

    pub fn eq_popup_toggle(&mut self) {
        self.eq_state.popup_open = !self.eq_state.popup_open;
    }

    pub fn eq_popup_open(&self) -> bool {
        self.eq_state.popup_open
    }

    pub fn eq_selected_band(&self) -> usize {
        self.eq_state.selected_band
    }

    pub fn eq_select_prev_band(&mut self) {
        self.eq_state.selected_band = self.eq_state.selected_band.saturating_sub(1);
    }

    pub fn eq_select_next_band(&mut self) {
        self.eq_state.selected_band = (self.eq_state.selected_band + 1).min(EqState::BAND_COUNT.saturating_sub(1));
    }

    pub fn eq_band_gain_db(&self, band: usize) -> f32 {
        let g = self.player.eq_gains();
        match band {
            0 => g.bass_db(),
            1 => g.mid_db(),
            2 => g.treble_db(),
            _ => 0.0,
        }
    }

    pub fn eq_band_up(&mut self) {
        let band = self.eq_state.selected_band;
        let g = self.player.eq_gains();
        let db = self.eq_band_gain_db(band);
        let new_db = (db + 1.0).min(12.0);
        match band {
            0 => g.set_bass_db(new_db),
            1 => g.set_mid_db(new_db),
            2 => g.set_treble_db(new_db),
            _ => {}
        }
    }

    pub fn eq_band_down(&mut self) {
        let band = self.eq_state.selected_band;
        let g = self.player.eq_gains();
        let db = self.eq_band_gain_db(band);
        let new_db = (db - 1.0).max(-12.0);
        match band {
            0 => g.set_bass_db(new_db),
            1 => g.set_mid_db(new_db),
            2 => g.set_treble_db(new_db),
            _ => {}
        }
    }
}
