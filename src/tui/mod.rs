mod tree;
mod app_state;
mod events;
mod layout;
mod widgets;

pub use app_state::AppState;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{poll_event, AppEvent};
use layout::AppLayout;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::mpsc::Receiver,
    time::Duration,
};
use crate::scanner::ScanEvent;

use std::path::PathBuf;

pub fn run_tui(rx: Receiver<ScanEvent>, scan_path: PathBuf) -> Result<AppState> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new(scan_path);
    let mut should_quit = false;

    // Main event loop
    while !should_quit {
        // Check for scan events (non-blocking) - Drain all pending events to avoid lag
        while let Ok(scan_event) = rx.try_recv() {
            match scan_event {
                ScanEvent::ProjectFound(project) => {
                    state.add_project(project);
                }
                ScanEvent::Scanning(path) => {
                    state.scanning_path = path;
                }
                ScanEvent::Complete => {
                    state.finish_scan();
                }
            }
        }

        // Render UI
        terminal.draw(|f| {
            let app_layout = AppLayout::new(f.area());

            widgets::render_project_tree(f, app_layout.project_tree, &state);
            widgets::render_details_pane(f, app_layout.details_pane, &state);
            widgets::render_action_pane(f, app_layout.action_pane, &state);

            if state.show_confirmation {
                widgets::render_confirmation_modal(f, &state);
            }
        })?;

        // Update spinner (simple ticker)
        state.spinner_index = state.spinner_index.wrapping_add(1);

        // Handle input
        if let Some(app_event) = poll_event(Duration::from_millis(100))? {
            if state.show_confirmation {
                // In confirmation modal
                match app_event {
                    AppEvent::ConfirmAction => {
                        // User pressed 'y' or Enter - confirm deletion
                        if state.selected_count() > 0 {
                            state.confirm_deletion();
                            should_quit = true;
                        } else {
                            state.show_confirmation = false;
                        }
                    }
                    AppEvent::CloseModal | AppEvent::Quit => {
                        state.show_confirmation = false;
                    }
                    _ => {}
                }
            } else {
                // Normal navigation
                match app_event {
                    AppEvent::Quit => should_quit = true,
                    AppEvent::MoveUp => state.move_up(),
                    AppEvent::MoveDown => state.move_down(),
                    AppEvent::ToggleSelection => state.toggle_selection(),
                    AppEvent::ConfirmAction => {
                        state.show_confirmation = true;
                    }
                    AppEvent::ToggleSort => state.toggle_sort(),
                    AppEvent::CycleFilter => state.cycle_filter(),
                    AppEvent::ToggleViewMode => state.toggle_view_mode(),
                    AppEvent::ToggleExpand => state.toggle_expand(),
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(state)
}
