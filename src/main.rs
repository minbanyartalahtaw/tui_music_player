mod app;
mod eq;
mod player;
mod ui;
mod visualizer;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> Result<()> {
    // Set panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal()?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut app = app::App::new()?;
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key.code, key.modifiers);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.check_track_end();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_key(app: &mut app::App, code: KeyCode, modifiers: KeyModifiers) {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);

    // Ctrl+E (or Ctrl+Meta+E where Meta is Alt): toggle Equalizer popup
    if ctrl && code == KeyCode::Char('e') {
        app.eq_popup_toggle();
        return;
    }

    // When EQ popup is open, handle popup-specific keys first
    if app.eq_popup_open() {
        match code {
            KeyCode::Esc => {
                app.eq_popup_toggle();
                return;
            }
            KeyCode::Left => {
                app.eq_select_prev_band();
                return;
            }
            KeyCode::Right => {
                app.eq_select_next_band();
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.eq_band_up();
                return;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.eq_band_down();
                return;
            }
            _ => {}
        }
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if ctrl => app.should_quit = true,
        KeyCode::Char(' ') => app.toggle_pause(),
        KeyCode::Enter => app.play_selected(),
        KeyCode::Char('n') => app.next_track(),
        KeyCode::Char('p') => app.prev_track(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Left => app.seek_backward(),
        KeyCode::Right => app.seek_forward(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.volume_up(),
        KeyCode::Char('-') => app.volume_down(),
        KeyCode::Char('r') => app.toggle_repeat(),
        _ => {}
    }
}
