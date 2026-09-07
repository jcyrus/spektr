use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug)]
pub enum AppEvent {
    Quit,
    MoveUp,
    MoveDown,
    ToggleSelection,
    ConfirmAction,
    ToggleSort,
    CycleFilter,
    CloseModal,
    ToggleViewMode,
    MoveRight,
    MoveLeft,
}

pub fn poll_event(timeout: Duration) -> Result<Option<AppEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }

    if let Event::Key(key) = event::read()? {
        // Windows reports both press and release; only act on press, or a
        // single physical key press fires its bound action twice -- e.g. one
        // tap of -> both entering a drill-down and immediately descending
        // again inside it.
        if key.kind != KeyEventKind::Press {
            return Ok(None);
        }
        return Ok(handle_key(key));
    }

    Ok(None)
}

fn handle_key(key: KeyEvent) -> Option<AppEvent> {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => Some(AppEvent::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(AppEvent::Quit),

        // Navigation
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => Some(AppEvent::MoveUp),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => Some(AppEvent::MoveDown),

        // Selection
        (KeyCode::Char(' '), _) => Some(AppEvent::ToggleSelection),

        // Actions
        (KeyCode::Enter, _) | (KeyCode::Char('y'), _) => Some(AppEvent::ConfirmAction),

        // Filters & Sorts
        (KeyCode::Char('s'), _) => Some(AppEvent::ToggleSort),
        (KeyCode::Char('f'), _) => Some(AppEvent::CycleFilter),

        // Modal close
        (KeyCode::Char('n'), _) => Some(AppEvent::CloseModal),

        // Horizontal navigation: open a project / expand a tree node, and back out.
        (KeyCode::Tab, _) => Some(AppEvent::ToggleViewMode),
        (KeyCode::Right, _) | (KeyCode::Char('l'), _) => Some(AppEvent::MoveRight),
        (KeyCode::Left, _) | (KeyCode::Char('h'), _) => Some(AppEvent::MoveLeft),

        _ => None,
    }
}
