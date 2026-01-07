use crate::tui::app_state::{AppState, SortMode};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_project_tree(f: &mut Frame, area: Rect, state: &AppState) {
    let projects = state.visible_projects();

    let items: Vec<ListItem> = projects
        .iter()
        .enumerate()
        .map(|(idx, project)| {
            let emoji = match project.strategy_name.as_str() {
                "Rust" => "🦀",
                "Node.js" => "📦",
                "Flutter" => "💙",
                "Android" => "🤖",
                _ => "📁",
            };

            let size = format_size(project.total_size);
            let path = project
                .root_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            let checkbox = if state.is_selected(idx) { "[✓]" } else { "[ ]" };

            let text = format!("{} {} {} - {}", checkbox, emoji, path, size);

            let style = if idx == state.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if state.is_selected(idx) {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let sort_label = match state.sort_mode {
        SortMode::SizeDesc => "Size ↓",
        SortMode::SizeAsc => "Size ↑",
        SortMode::NameAsc => "Name ↑",
        SortMode::NameDesc => "Name ↓",
    };

    let title = if state.scanning {
        format!(
            " Projects (Scanning...) | Sort: {} | Filter: {} ",
            sort_label,
            state.filter_mode.label()
        )
    } else {
        format!(
            " Projects ({}/{}) | Sort: {} | Filter: {} ",
            state.visible_count(),
            state.total_projects(),
            sort_label,
            state.filter_mode.label()
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    f.render_widget(list, area);
}

pub fn render_details_pane(f: &mut Frame, area: Rect, state: &AppState) {
    let text = if let Some(project) = state.current_project() {
        let path_str = project.root_path.display().to_string();

        vec![
            Line::from(vec![
                Span::styled("Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(path_str),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(project.strategy_name.clone()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Size: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format_size(project.total_size),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Rebuild Cost: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(match project.strategy_name.as_str() {
                    "Rust" => "~2-5 mins (cargo build)",
                    "Node.js" => "~1-2 mins (npm install)",
                    "Flutter" => "~1-3 mins (flutter pub get)",
                    "Android" => "~3-10 mins (gradle build)",
                    _ => "~1-3 mins",
                }),
            ]),
        ]
    } else {
        vec![Line::from("No project selected")]
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

pub fn render_action_pane(f: &mut Frame, area: Rect, state: &AppState) {
    let total_size = state.total_selected_size();
    let selected_count = state.selected_count();

    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Total Reclaimable:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            format_size(total_size),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("Selected: {} projects", selected_count),
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Controls:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  ↑/↓ or j/k: Navigate"),
        Line::from("  Space: Toggle selection"),
        Line::from("  Enter: Clean selected"),
        Line::from("  s: Toggle sort"),
        Line::from("  f: Cycle filter"),
        Line::from("  q/Esc: Quit"),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Actions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

pub fn render_confirmation_modal(f: &mut Frame, state: &AppState) {
    let selected_count = state.selected_count();
    let total_size = state.total_selected_size();

    if selected_count == 0 {
        let area = centered_rect(50, 30, f.area());

        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "⚠️  No Projects Selected",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Please select at least one project"),
            Line::from("using the spacebar."),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press any key to continue...",
                Style::default().fg(Color::Gray),
            )]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Warning ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    } else {
        let area = centered_rect(60, 40, f.area());

        let text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "⚠️  Confirm Deletion",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Delete "),
                Span::styled(
                    format!("{} projects", selected_count),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" totaling "),
                Span::styled(
                    format_size(total_size),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("?"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "This action cannot be undone!",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("y", Style::default().fg(Color::Green)),
                Span::styled(" to confirm, ", Style::default().fg(Color::Gray)),
                Span::styled("n", Style::default().fg(Color::Red)),
                Span::styled(" to cancel", Style::default().fg(Color::Gray)),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Confirmation ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
