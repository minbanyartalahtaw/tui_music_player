use std::time::Duration;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, BorderType, Borders, Gauge, List, ListItem, Padding, Paragraph},
};

use crate::app::{App, RepeatMode};

const CYAN: Color = Color::Cyan;
const WHITE: Color = Color::White;
const GRAY: Color = Color::Gray;
const DARK_GRAY: Color = Color::DarkGray;
const GREEN: Color = Color::Green;
const YELLOW: Color = Color::Yellow;
const HIGHLIGHT_BG: Color = Color::Rgb(35, 35, 55);

fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

fn truncate_name(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(6), // Visualizer
        Constraint::Length(8), // Now playing
    ])
    .split(frame.area());

    draw_song_list(frame, app, chunks[0]);
    draw_visualizer(frame, app, chunks[1]);
    draw_now_playing(frame, app, chunks[2]);

    if app.eq_state.popup_open {
        draw_eq_popup(frame, app);
    }
}

fn draw_song_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ♫ ", Style::default().fg(CYAN)),
            Span::styled(
                "Music Player ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DARK_GRAY))
        .padding(Padding::horizontal(1));

    if app.songs.is_empty() {
        let msg = Paragraph::new(Line::from(vec![
            Span::styled("No music files found in ", Style::default().fg(DARK_GRAY)),
            Span::styled("./music/", Style::default().fg(WHITE)),
        ]))
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

    let inner_width = block.inner(area).width as usize;

    let items: Vec<ListItem> = app
        .songs
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let is_selected = i == app.selected;
            let is_playing = app.now_playing == Some(i);

            let indicator = if is_playing { "▸ " } else { "  " };
            let indicator_display_w: usize = 2;
            let dur_str = song
                .duration
                .map(|d| format_duration(d))
                .unwrap_or_else(|| "─:──".to_string());
            let dur_display_w = dur_str.len();

            let max_name_chars =
                inner_width.saturating_sub(indicator_display_w + dur_display_w + 2);
            let name = truncate_name(&song.name, max_name_chars);
            let name_display_w = name.chars().count();

            let total_used = indicator_display_w + name_display_w + dur_display_w;
            let pad_len = inner_width.saturating_sub(total_used);

            let indicator_style = if is_playing {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(DARK_GRAY)
            };

            let name_style = match (is_selected, is_playing) {
                (true, true) => Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                (true, false) => Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
                (false, true) => Style::default().fg(CYAN),
                (false, false) => Style::default().fg(GRAY),
            };

            let line = Line::from(vec![
                Span::styled(indicator, indicator_style),
                Span::styled(name, name_style),
                Span::raw(" ".repeat(pad_len)),
                Span::styled(dur_str, Style::default().fg(DARK_GRAY)),
            ]);

            let mut item = ListItem::new(line);
            if is_selected {
                item = item.style(Style::default().bg(HIGHLIGHT_BG));
            }
            item
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default());

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_visualizer(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DARK_GRAY));

    let spectrum = app.spectrum();
    let inner = block.inner(area);

    // Calculate how many bars fit in the available width.
    let bar_w: u16 = 2;
    let gap: u16 = 1;
    let max_bars = if inner.width > 0 {
        ((inner.width + gap) / (bar_w + gap)) as usize
    } else {
        0
    };

    let display = resample_spectrum(&spectrum, max_bars);

    let bars: Vec<Bar> = display
        .iter()
        .map(|&v| {
            Bar::default()
                .value(v)
                .style(Style::default().fg(CYAN))
                .text_value(String::new())
        })
        .collect();

    let chart = BarChart::default()
        .block(block)
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_w)
        .bar_gap(gap)
        .max(100);

    frame.render_widget(chart, area);
}

/// Resample `data` (fixed-size spectrum from the analyser) into `target_len`
/// bars by averaging adjacent bins, so the chart adapts to any terminal width.
fn resample_spectrum(data: &[u64], target_len: usize) -> Vec<u64> {
    if data.is_empty() || target_len == 0 {
        return vec![0; target_len];
    }
    if target_len >= data.len() {
        // More space than bins -- just return data padded or as-is.
        let mut out = data.to_vec();
        out.resize(target_len, 0);
        return out;
    }
    (0..target_len)
        .map(|i| {
            let lo = i * data.len() / target_len;
            let hi = ((i + 1) * data.len() / target_len).max(lo + 1).min(data.len());
            let sum: u64 = data[lo..hi].iter().sum();
            sum / (hi - lo) as u64
        })
        .collect()
}

/// Equalizer popup: centered overlay with 3 band gauges; selected band highlighted.
fn draw_eq_popup(frame: &mut Frame, app: &App) {
    const POPUP_W: u16 = 44;
    const POPUP_H: u16 = 14;
    let area = frame.area();
    let popup_x = area.width.saturating_sub(POPUP_W) / 2;
    let popup_y = area.height.saturating_sub(POPUP_H) / 2;
    let popup_rect = Rect::new(area.x + popup_x, area.y + popup_y, POPUP_W, POPUP_H);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" Equalizer ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(" Ctrl+E close ", Style::default().fg(DARK_GRAY)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN));

    let inner = block.inner(popup_rect);
    frame.render_widget(block, popup_rect);

    let selected = app.eq_selected_band();
    let band_labels = ["Bass  ", "Mid   ", "Treble"];

    let label_w = 8u16;
    let db_w = 8u16;
    let gauge_w = inner.width.saturating_sub(label_w + db_w + 2).max(4);

    for (i, &label) in band_labels.iter().enumerate() {
        let row_y = inner.y + 2 + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let db = app.eq_band_gain_db(i);
        let ratio = ((db + 12.0) / 24.0).clamp(0.0, 1.0) as f64;
        let is_selected = i == selected;

        let style = if is_selected {
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(GRAY)
        };
        let gauge_style = if is_selected { Style::default().fg(CYAN) } else { Style::default().fg(DARK_GRAY) };

        let label_rect = Rect::new(inner.x + 1, row_y, label_w, 1);
        let gauge_rect = Rect::new(inner.x + 1 + label_w, row_y, gauge_w, 1);
        let db_rect = Rect::new(inner.x + 1 + label_w + gauge_w, row_y, db_w, 1);

        frame.render_widget(Paragraph::new(Line::from(Span::styled(label, style))), label_rect);

        let gauge = Gauge::default()
            .gauge_style(gauge_style)
            .ratio(ratio)
            .label(Span::raw(""));
        frame.render_widget(gauge, gauge_rect);

        let db_str = format!("{:+.0} dB", db);
        frame.render_widget(Paragraph::new(Line::from(Span::styled(db_str, style))), db_rect);
    }

    let hint = Line::from(vec![
        Span::styled("← → band  ", Style::default().fg(DARK_GRAY)),
        Span::styled("↑ ↓ gain  ", Style::default().fg(DARK_GRAY)),
        Span::styled("Esc/Ctrl+E close", Style::default().fg(DARK_GRAY)),
    ]);
    let hint_rect = Rect::new(inner.x, inner.y + inner.height.saturating_sub(2), inner.width, 1);
    frame.render_widget(Paragraph::new(hint), hint_rect);
}

fn draw_now_playing(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Now Playing ",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DARK_GRAY))
        .padding(Padding::new(2, 2, 1, 0));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 5 || inner.width < 20 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1), // Song title
        Constraint::Length(1), // Progress bar
        Constraint::Length(1), // Volume + Repeat
        Constraint::Length(1), // Spacer
        Constraint::Length(1), // Controls
    ])
    .split(inner);

    // ── Now playing title ──
    let icon = if app.is_playing() {
        Span::styled("▶  ", Style::default().fg(GREEN))
    } else if app.now_playing.is_some() {
        Span::styled("⏸  ", Style::default().fg(YELLOW))
    } else {
        Span::styled("■  ", Style::default().fg(DARK_GRAY))
    };

    let title = Line::from(vec![
        icon,
        Span::styled(
            app.now_playing_name(),
            Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(title), chunks[0]);

    // ── Progress bar ──
    let pos = app.current_position();
    let dur = app.current_duration();
    let pos_str = format_duration(pos);
    let dur_str = dur
        .map(|d| format_duration(d))
        .unwrap_or_else(|| "─:──".to_string());

    let bar_width =
        (chunks[1].width as usize).saturating_sub(pos_str.len() + dur_str.len() + 2);
    let ratio = match dur {
        Some(d) if d.as_secs() > 0 => (pos.as_secs_f64() / d.as_secs_f64()).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let filled = (ratio * bar_width as f64) as usize;
    let empty = bar_width.saturating_sub(filled);

    let progress = Line::from(vec![
        Span::styled(pos_str, Style::default().fg(WHITE)),
        Span::raw(" "),
        Span::styled("━".repeat(filled), Style::default().fg(CYAN)),
        Span::styled("─".repeat(empty), Style::default().fg(DARK_GRAY)),
        Span::raw(" "),
        Span::styled(dur_str, Style::default().fg(DARK_GRAY)),
    ]);
    frame.render_widget(Paragraph::new(progress), chunks[1]);

    // ── Volume + Repeat ──
    let vol = app.volume_percent();
    let repeat_mode = app.repeat;
    let repeat_style = if repeat_mode != RepeatMode::Off {
        Style::default().fg(CYAN)
    } else {
        Style::default().fg(DARK_GRAY)
    };

    let vol_repeat = Line::from(vec![
        Span::styled("Vol ", Style::default().fg(DARK_GRAY)),
        Span::styled(format!("{vol}%"), Style::default().fg(WHITE)),
        Span::raw("    "),
        Span::styled("⟳ Repeat: ", repeat_style),
        Span::styled(
            repeat_mode.label(),
            repeat_style.add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(vol_repeat), chunks[2]);

    // ── Controls ──
    let controls = Line::from(vec![
        Span::styled("␣", Style::default().fg(CYAN)),
        Span::styled(" Pause  ", Style::default().fg(DARK_GRAY)),
        Span::styled("↑↓", Style::default().fg(CYAN)),
        Span::styled(" Nav  ", Style::default().fg(DARK_GRAY)),
        Span::styled("⏎", Style::default().fg(CYAN)),
        Span::styled(" Play  ", Style::default().fg(DARK_GRAY)),
        Span::styled("n/p", Style::default().fg(CYAN)),
        Span::styled(" Next/Prev  ", Style::default().fg(DARK_GRAY)),
        Span::styled("←→", Style::default().fg(CYAN)),
        Span::styled(" Seek  ", Style::default().fg(DARK_GRAY)),
        Span::styled("±", Style::default().fg(CYAN)),
        Span::styled(" Vol  ", Style::default().fg(DARK_GRAY)),
        Span::styled("r", Style::default().fg(CYAN)),
        Span::styled(" Repeat  ", Style::default().fg(DARK_GRAY)),
        Span::styled("q", Style::default().fg(CYAN)),
        Span::styled(" Quit", Style::default().fg(DARK_GRAY)),
    ]);
    frame.render_widget(Paragraph::new(controls), chunks[4]);
}
