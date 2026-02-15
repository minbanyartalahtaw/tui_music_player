# TUI Music Player — Usage Guide

This document describes how to use the TUI music player: layout, keybindings, and behavior in detail.

---

## 1. Starting the player

1. Ensure you have a `music/` directory in the project root.
2. Put supported audio files (e.g. MP3, WAV, OGG, FLAC) in `music/`.
3. Run:
   ```bash
   cargo run
   ```
4. The terminal switches to the alternate screen and shows the player UI. When you quit, the previous terminal content is restored.

If `music/` is missing or empty, the playlist area shows: *No music files found in ./music/*.

---

## 2. Screen layout

The interface is split into three main areas (top to bottom).

### 2.1 Playlist (top)

- **Title:** “♫ Music Player”
- **Content:** One line per track: optional playing indicator (▸), track name, and duration (e.g. `3:45`).
- **Selection:** One row is highlighted (dark background). The currently playing track is marked with a green ▸ and cyan name when applicable.
- **Scrolling:** The list scrolls so the selected (and playing) item stays visible when there are many tracks.

### 2.2 Spectrum visualizer (middle)

- **Content:** A row of vertical bars showing real-time frequency levels (from FFT of the current audio).
- **Behavior:** Updates while audio is playing; bars reflect bass to treble. When nothing is playing, bars can fall to zero.
- **Resizing:** Bar count adapts to terminal width.

### 2.3 Now playing (bottom)

- **Title:** “Now Playing”
- **Line 1 — Track and state:**  
  ▶ (green) = playing, ⏸ (yellow) = paused, ■ (gray) = stopped. Then the current track name or “Nothing playing”.
- **Line 2 — Progress:**  
  Current time (e.g. `1:23`), a progress bar (filled = elapsed), total time (e.g. `4:56`).
- **Line 3 — Volume and repeat:**  
  “Vol 100%” and “⟳ Repeat: Off | All | One”.
- **Line 4 — Controls hint:**  
  Short list of main keys (Pause, Nav, Play, Next/Prev, Seek, Vol, Repeat, Quit).

### 2.4 Equalizer popup (overlay)

- **When:** Shown only when the EQ popup is open (see **Ctrl+E** below).
- **Where:** Centered overlay with a bordered “Equalizer” box.
- **Content:**
  - Three rows: **Bass**, **Mid**, **Treble**, each with a horizontal gauge and gain in dB (e.g. `+2 dB`).
  - One band is “active” (highlighted in cyan).
  - At the bottom: “← → band   ↑ ↓ gain   Esc/Ctrl+E close”.

---

## 3. Keybindings reference

### 3.1 Global (main screen)

| Key | Action |
|-----|--------|
| **q** | Quit |
| **Ctrl+C** | Quit |
| **Space** | Pause / Resume |
| **Enter** | Play selected track |
| **n** | Next track |
| **p** | Previous track (or restart current if &gt; 3 s in) |
| **↑** or **k** | Move selection up in playlist |
| **↓** or **j** | Move selection down in playlist |
| **←** | Seek backward 5 seconds |
| **→** | Seek forward 5 seconds |
| **+** or **=** | Volume up |
| **-** | Volume down |
| **r** | Cycle repeat mode: Off → All → One → Off |
| **Ctrl+E** | Open or close Equalizer popup |

### 3.2 When Equalizer popup is open

| Key | Action |
|-----|--------|
| **Ctrl+E** | Close popup |
| **Esc** | Close popup |
| **←** | Select previous band (Bass ← Mid ← Treble) |
| **→** | Select next band |
| **↑** or **k** | Increase gain of selected band (+1 dB, max +12 dB) |
| **↓** or **j** | Decrease gain of selected band (−1 dB, min −12 dB) |

All other keys are ignored while the popup is open (e.g. no seek/volume/playlist).

---

## 4. Playback behavior

- **Play:** Enter or **n**/**p** when nothing is playing starts the selected or next/previous track.
- **Pause:** Space toggles pause; the progress bar and time stop advancing.
- **Next:** **n** goes to the next track; at the end of the list, behavior depends on repeat (see below).
- **Previous:** **p** goes to the previous track, or restarts the current one if already more than 3 seconds in. At the first track with repeat Off, previous does nothing.
- **Seek:** **←** and **→** move playback by 5 seconds (implemented by restarting from the new position). Forward seek is clamped to the end of the track.
- **End of track:**  
  - **Repeat Off:** Stops (no auto-advance).  
  - **Repeat All:** Plays the next track; after the last, goes to the first.  
  - **Repeat One:** Replays the current track.

---

## 5. Equalizer

- **Bands:** Bass (120 Hz), Mid (1 kHz), Treble (8 kHz). Each is a peaking biquad filter.
- **Range:** ±12 dB per band. 0 dB = flat (no change).
- **Persistence:** Gains are kept for the session; they apply to all playback (same EQ for every track).
- **Popup:** Open with **Ctrl+E**. Use **←**/**→** to choose the band, **↑**/**↓** to change its gain. Close with **Esc** or **Ctrl+E**. Changes take effect in real time.

---

## 6. Volume

- **Range:** 0%–150% (relative to decoded level).
- **Keys:** **+** / **=** increase, **-** decrease, in 5% steps.
- **Display:** Shown in the “Now playing” block as “Vol XX%”.

---

## 7. Tips and notes

- **Music folder:** Only files in `./music/` are listed. Supported extensions: mp3, wav, ogg, flac, m4a, aac (case-insensitive).
- **Duration:** Shown next to each track and in the progress line. For some formats or corrupt files, duration may be unknown (shown as “─:──”).
- **Resize:** The UI redraws on terminal resize; the spectrum bar count and layout adjust.
- **Quit:** Use **q** or **Ctrl+C** so the terminal is restored correctly (raw mode and alternate screen are cleared).

---

## 8. Troubleshooting

| Issue | What to check |
|-------|----------------|
| No sound | System volume, default audio device, and that the file format is supported. |
| “No music files found” | Ensure `music/` exists and contains files with supported extensions. |
| Seek seems to “restart” | Seek is implemented by restarting playback from the new position; a short gap is normal. |
| EQ has no effect | Confirm the EQ popup is closed and you adjusted the band with **↑**/**↓** (not only **←**/**→**). |
| Keys do nothing in popup | Only EQ keys (← → ↑ ↓ Esc Ctrl+E) work when the Equalizer popup is open. |

For build or run errors, ensure Rust is up to date (`rustup update`) and that the project builds with `cargo build`.
