//! Terminal UI for OISP Sensor

mod app;
mod ui;
mod event_handler;

pub use app::App;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Run the TUI application
pub async fn run(
    event_rx: broadcast::Receiver<Arc<oisp_core::events::OispEvent>>,
) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app
    let mut app = App::new(event_rx);
    
    // Run the app
    let res = run_app(&mut terminal, &mut app).await;
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    res
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        
        // Poll for events with timeout
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('t') => app.set_view(app::View::Timeline),
                        KeyCode::Char('i') => app.set_view(app::View::Inventory),
                        KeyCode::Char('p') => app.set_view(app::View::ProcessTree),
                        KeyCode::Char('r') => app.set_view(app::View::Traces),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                        KeyCode::PageUp => app.page_up(),
                        KeyCode::PageDown => app.page_down(),
                        _ => {}
                    }
                }
            }
        }
        
        // Process incoming events
        app.process_events();
    }
}

