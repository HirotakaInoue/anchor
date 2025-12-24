mod app;
mod port;
mod tunnel;
mod ui;

use anyhow::Result;
use app::{App, AppTab};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new()?;

    // Initial port scan
    app.refresh_ports()?;

    // Main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for events with timeout for auto-refresh
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Global quit
                if key.code == KeyCode::Char('q') && !app.show_input && !app.show_filter {
                    return Ok(());
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }

                // Handle input mode
                if app.show_input {
                    match key.code {
                        KeyCode::Enter => app.submit_input()?,
                        KeyCode::Esc => app.cancel_input(),
                        KeyCode::Char(c) => app.input_buffer.push(c),
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle filter mode
                if app.show_filter {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.show_filter = false;
                        }
                        KeyCode::Char(c) => {
                            app.filter_text.push(c);
                            app.apply_filter();
                        }
                        KeyCode::Backspace => {
                            app.filter_text.pop();
                            app.apply_filter();
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle confirmation dialog
                if app.show_confirm {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_action()?;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.show_confirm = false;
                            app.confirm_message.clear();
                        }
                        _ => {}
                    }
                    continue;
                }

                // Normal mode key handling
                match key.code {
                    // Tab navigation
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.prev_tab(),
                    KeyCode::Char('1') => app.current_tab = AppTab::Ports,
                    KeyCode::Char('2') => app.current_tab = AppTab::Tunnels,

                    // List navigation
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Home | KeyCode::Char('g') => app.select_first(),
                    KeyCode::End | KeyCode::Char('G') => app.select_last(),

                    // Actions
                    KeyCode::Char('r') | KeyCode::F(5) => app.refresh_ports()?,
                    KeyCode::Char('/') => {
                        app.show_filter = true;
                        app.filter_text.clear();
                    }
                    KeyCode::Char('K') => app.request_kill()?,
                    KeyCode::Char('a') => {
                        if matches!(app.current_tab, AppTab::Tunnels) {
                            app.start_add_tunnel();
                        }
                    }
                    KeyCode::Char('c') => {
                        if matches!(app.current_tab, AppTab::Tunnels) {
                            app.connect_tunnel()?;
                        }
                    }
                    KeyCode::Char('d') => {
                        if matches!(app.current_tab, AppTab::Tunnels) {
                            app.disconnect_tunnel()?;
                        }
                    }
                    KeyCode::Char('x') => {
                        if matches!(app.current_tab, AppTab::Tunnels) {
                            app.request_delete_tunnel()?;
                        }
                    }
                    KeyCode::Esc => {
                        app.filter_text.clear();
                        app.apply_filter();
                    }
                    _ => {}
                }
            }
        }
    }
}
