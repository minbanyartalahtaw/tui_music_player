# TUI Music Player

A terminal-based music player with a clean, minimalist interface. Play audio from a local `music/` folder with keyboard controls, real-time spectrum visualization, and a 3-band equalizer.

## Features

- **Playlist** — Lists all supported audio files from `./music/`, sorted by name
- **Playback** — Play, pause, next, previous, seek forward/backward (5 s)
- **Progress** — Elapsed time and total duration with a progress bar
- **Volume** — Adjustable volume (0–150%)
- **Repeat** — Off, All (loop playlist), One (loop current track)
- **Spectrum visualizer** — Real-time frequency bars (FFT, Hann window, background thread)
- **3-band equalizer** — Bass, Mid, Treble (peaking biquad filters, ±12 dB)
- **EQ popup** — Interactive overlay to adjust bands with gauges (Ctrl+E)

## Requirements

- **Rust** 1.70+ (2024 edition)
- **Audio** — Working output device (ALSA/PulseAudio on Linux, etc.)
- **Terminal** — Supports UTF-8 and colors (e.g. modern xterm, kitty, Alacritty)

## Quick start

```bash
# Clone or navigate to the project
cd tui_music_player

# Create a music directory and add audio files
mkdir -p music
# Copy your .mp3, .wav, .ogg, .flac, .m4a, .aac files into music/

# Run
cargo run
```

## Project layout

```
tui_music_player/
├── Cargo.toml
├── README.md
├── docs/
│   └── USAGE.md       # Detailed usage and keybindings
├── music/              # Place audio files here (required)
└── src/
    ├── main.rs         # Entry point, terminal & event loop
    ├── app.rs          # App state, playlist, EQ state
    ├── player.rs       # Rodio playback, EQ/visualizer chain
    ├── eq.rs           # 3-band biquad equalizer (Bass/Mid/Treble)
    ├── visualizer.rs   # FFT spectrum analyzer (background thread)
    └── ui.rs           # Ratatui layout and widgets
```

## Supported formats

Decoding is provided by **rodio** (Symphonia): MP3, WAV, OGG (Vorbis), FLAC, M4A, AAC. Put files in the `music/` directory; the player scans it on startup.

## Documentation

- **[docs/USAGE.md](docs/USAGE.md)** — Full keybindings, UI sections, equalizer, and usage details.

## License

Use and modify as you like (no formal license specified).
