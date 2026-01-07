use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub project_tree: Rect,
    pub details_pane: Rect,
    pub action_pane: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        // Main horizontal split: 60% left (tree), 40% right (details + action)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Right side vertical split: 50% details, 50% action
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[1]);

        Self {
            project_tree: main_chunks[0],
            details_pane: right_chunks[0],
            action_pane: right_chunks[1],
        }
    }
}
