use crate::analyze::Row;
use crate::format::{format_size, pad_to_width, truncate};
use crate::theme;
use crate::tui::app_state::{AppState, Drill, SortMode, ViewMode};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Every pane shares one frame treatment, so the UI reads as a single surface.
fn panel(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme::BRAND)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::CHROME))
}

fn strategy_icon(name: &str) -> &'static str {
    match name {
        "Rust" => "🦀",
        "Node.js" => "📦",
        "Flutter" => "💙",
        "Android" => "🤖",
        _ => "📁",
    }
}

fn dim(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(theme::DIM))
}

pub fn render_project_tree(f: &mut Frame, area: Rect, state: &AppState) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let total = state.visible_total_size();

    let items: Vec<ListItem> = match state.view_mode {
        ViewMode::List => {
            // Columns: checkbox, icon, name, size, share, bar, open affordance.
            let bar_width = (inner_width / 5).clamp(6, 20);
            let name_width = inner_width
                .saturating_sub(2 + 4 + 3 + 10 + 3 + 6 + 1 + bar_width + 2)
                .max(8);

            state
                .visible_projects()
                .iter()
                .enumerate()
                .map(|(idx, project)| {
                    let focused = idx == state.selected_index;
                    let checked = state.is_selected(idx);
                    let share = if total == 0 {
                        0.0
                    } else {
                        project.total_size as f64 / total as f64 * 100.0
                    };
                    let name = project
                        .root_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();

                    let name_style = if focused {
                        Style::default()
                            .fg(theme::BRIGHT)
                            .add_modifier(Modifier::BOLD)
                    } else if checked {
                        Style::default().fg(theme::OK)
                    } else {
                        Style::default().fg(theme::BODY)
                    };

                    let (filled, empty) = theme::bar_spans(share, bar_width);
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            if focused { "▸ " } else { "  " },
                            Style::default().fg(theme::ACCENT),
                        ),
                        Span::styled(
                            if checked { "[▣] " } else { "[ ] " },
                            Style::default().fg(if checked {
                                theme::ACCENT
                            } else {
                                theme::CHROME
                            }),
                        ),
                        Span::raw(format!("{} ", strategy_icon(&project.strategy_name))),
                        Span::styled(
                            pad_to_width(&truncate(&name, name_width), name_width),
                            name_style,
                        ),
                        Span::styled(
                            format!("{:>10}", format_size(project.total_size)),
                            Style::default().fg(theme::size_color(project.total_size)),
                        ),
                        Span::styled(" │ ", Style::default().fg(theme::CHROME)),
                        dim(format!("{share:>5.1}%")),
                        Span::raw(" "),
                        filled,
                        empty,
                        Span::styled(" →", Style::default().fg(theme::CHROME)),
                    ]))
                })
                .collect()
        }
        ViewMode::Tree => state
            .get_flat_tree()
            .iter()
            .enumerate()
            .map(|(idx, flat_node)| {
                let node = flat_node.node;
                let focused = idx == state.selected_index;
                let fold_marker = if node.children.is_empty() {
                    " "
                } else if node.collapsed {
                    "▶"
                } else {
                    "▼"
                };
                let icon = node
                    .project
                    .as_ref()
                    .map(|p| strategy_icon(&p.strategy_name))
                    .unwrap_or("📁");

                let name_style = if focused {
                    Style::default()
                        .fg(theme::BRIGHT)
                        .add_modifier(Modifier::BOLD)
                } else if node.checked {
                    Style::default().fg(theme::OK)
                } else {
                    Style::default().fg(theme::BODY)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        flat_node.guide_prefix.clone(),
                        Style::default().fg(theme::CHROME),
                    ),
                    Span::styled(
                        format!("{fold_marker} "),
                        Style::default().fg(theme::ACCENT),
                    ),
                    Span::styled(
                        if node.checked { "[▣] " } else { "[ ] " },
                        Style::default().fg(if node.checked {
                            theme::ACCENT
                        } else {
                            theme::CHROME
                        }),
                    ),
                    Span::raw(format!("{icon} ")),
                    Span::styled(node.label().to_string(), name_style),
                    dim(" — "),
                    Span::styled(
                        format_size(node.total_size()),
                        Style::default().fg(theme::size_color(node.total_size())),
                    ),
                ]))
            })
            .collect(),
    };

    let sort_label = match state.sort_mode {
        SortMode::SizeDesc => "Size ↓",
        SortMode::SizeAsc => "Size ↑",
        SortMode::NameAsc => "Name ↑",
        SortMode::NameDesc => "Name ↓",
    };
    let view_label = match state.view_mode {
        ViewMode::List => "List",
        ViewMode::Tree => "Tree",
    };
    let count = if state.scanning {
        "scanning…".to_string()
    } else {
        state.visible_count().to_string()
    };

    let mut block = panel("Projects").title(Line::from(vec![
        dim(format!(" {count} · {view_label} · sort ")),
        Span::styled(sort_label, Style::default().fg(theme::ACCENT)),
        dim(" · filter "),
        Span::styled(
            state.filter_mode.label().to_string(),
            Style::default().fg(theme::ACCENT),
        ),
        Span::raw(" "),
    ]));

    if state.scanning {
        const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame = SPINNER[state.spinner_index % SPINNER.len()];
        let max_len = area.width.saturating_sub(20) as usize;
        block = block.title_bottom(
            Line::from(vec![
                Span::styled(format!(" {frame} "), Style::default().fg(theme::ACCENT)),
                dim(truncate(&state.scanning_path, max_len)),
                Span::raw(" "),
            ])
            .alignment(Alignment::Right),
        );
    }

    // A fresh ListState each frame keeps the cursor scrolled into view; without
    // it a list longer than the pane simply never scrolls.
    let mut list_state = ListState::default().with_selected(Some(state.selected_index));
    f.render_stateful_widget(List::new(items).block(block), area, &mut list_state);
}

/// The project drill-down: where one project's bytes actually sit.
pub fn render_drill_pane(f: &mut Frame, area: Rect, state: &AppState) {
    let block = panel("Storage");
    let inner = block.inner(area);
    f.render_widget(block, area);

    match &state.drill {
        Some(Drill::Scanning { name, progress, .. }) => {
            const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = SPINNER[state.spinner_index % SPINNER.len()];
            let dirs = progress.dirs.load(std::sync::atomic::Ordering::Relaxed);
            let bytes = progress.bytes.load(std::sync::atomic::Ordering::Relaxed);
            let lines = vec![
                Line::from(vec![Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(theme::BRIGHT)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(format!("{frame} "), Style::default().fg(theme::ACCENT)),
                    Span::styled("sizing  ", Style::default().fg(theme::BODY)),
                    Span::styled(format_size(bytes), Style::default().fg(theme::OK)),
                    dim(format!("  ·  {dirs} directories")),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        Some(Drill::Ready { name, browser }) => {
            let rows = browser.rows();
            let total = browser.total();
            let width = inner.width as usize;
            let bar_width = (width / 5).clamp(6, 20);
            let name_width = width
                .saturating_sub(2 + 10 + 3 + 6 + 1 + bar_width + 2)
                .max(8);

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(theme::BRAND)
                            .add_modifier(Modifier::BOLD),
                    ),
                    dim("  ▸  "),
                    Span::styled(
                        truncate(&browser.cwd.display().to_string(), width.saturating_sub(4)),
                        Style::default().fg(theme::BRIGHT),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(format_size(total), Style::default().fg(theme::OK)),
                    dim(format!("  ·  {} items", browser.entries.len())),
                    if browser.at_root() {
                        dim("  ·  ← back to projects")
                    } else {
                        dim("  ·  ← parent")
                    },
                ]),
                Line::from(Span::styled(
                    "─".repeat(width),
                    Style::default().fg(theme::CHROME),
                )),
            ];

            let body_height = inner.height.saturating_sub(3) as usize;
            let offset = browser.cursor.saturating_sub(body_height.saturating_sub(1));
            for (index, row) in rows.iter().enumerate().skip(offset).take(body_height) {
                let focused = index == browser.cursor;
                let (label, size, is_dir, aggregate) = match row {
                    Row::Item(entry) => (entry.name.clone(), entry.size, entry.is_dir, false),
                    Row::Other { count, size } => {
                        (format!("Other ({count} items)"), *size, true, true)
                    }
                };
                let share = if total == 0 {
                    0.0
                } else {
                    size as f64 / total as f64 * 100.0
                };
                let name_style = if aggregate {
                    Style::default().fg(theme::DIM)
                } else if focused {
                    Style::default()
                        .fg(theme::BRIGHT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::BODY)
                };
                let (filled, empty) = theme::bar_spans(share, bar_width);
                lines.push(Line::from(vec![
                    Span::styled(
                        if focused { "▸ " } else { "  " },
                        Style::default().fg(theme::ACCENT),
                    ),
                    Span::styled(
                        pad_to_width(&truncate(&label, name_width), name_width),
                        name_style,
                    ),
                    Span::styled(
                        format!("{:>10}", format_size(size)),
                        Style::default().fg(theme::size_color(size)),
                    ),
                    Span::styled(" │ ", Style::default().fg(theme::CHROME)),
                    dim(format!("{share:>5.1}%")),
                    Span::raw(" "),
                    filled,
                    empty,
                    Span::styled(
                        if is_dir { " →" } else { "  " },
                        Style::default().fg(theme::CHROME),
                    ),
                ]));
            }
            f.render_widget(Paragraph::new(lines), inner);
        }
        None => {}
    }
}

pub fn render_details_pane(f: &mut Frame, area: Rect, state: &AppState) {
    let bold = Style::default()
        .fg(theme::BODY)
        .add_modifier(Modifier::BOLD);

    let text = if let Some(Drill::Ready { browser, .. }) = &state.drill {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Folder: ", bold),
                Span::styled(
                    browser.cwd.display().to_string(),
                    Style::default().fg(theme::BRIGHT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Size: ", bold),
                Span::styled(
                    format_size(browser.total()),
                    Style::default().fg(theme::size_color(browser.total())),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Contains: ", bold),
                dim(format!("{} items", browser.entries.len())),
            ]),
            Line::from(""),
        ];
        if let Some(Row::Item(entry)) = browser.rows().get(browser.cursor) {
            lines.push(Line::from(vec![
                Span::styled("Selected: ", bold),
                Span::styled(entry.name.clone(), Style::default().fg(theme::ACCENT)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", bold),
                Span::styled(
                    format_size(entry.size),
                    Style::default().fg(theme::size_color(entry.size)),
                ),
            ]));
        }
        lines
    } else if let Some(project) = state.current_project() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Path: ", bold),
                Span::styled(
                    project.root_path.display().to_string(),
                    Style::default().fg(theme::BRIGHT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Type: ", bold),
                Span::styled(
                    project.strategy_name.clone(),
                    Style::default().fg(theme::ACCENT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Targets: ", bold),
                Span::styled("(will be deleted)", Style::default().fg(theme::DANGER)),
            ]),
        ];

        for target in &project.targets {
            let display_text = target
                .strip_prefix(&project.root_path)
                .unwrap_or(target)
                .display()
                .to_string();
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme::CHROME)),
                Span::styled(display_text, Style::default().fg(theme::DANGER)),
            ]));
        }

        lines.extend([
            Line::from(""),
            Line::from(vec![
                Span::styled("Size: ", bold),
                Span::styled(
                    format_size(project.total_size),
                    Style::default().fg(theme::size_color(project.total_size)),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Rebuild cost: ", bold),
                dim(match project.strategy_name.as_str() {
                    "Rust" => "~2-5 mins (cargo build)",
                    "Node.js" => "~1-2 mins (npm install)",
                    "Flutter" => "~1-3 mins (flutter pub get)",
                    "Android" => "~3-10 mins (gradle build)",
                    _ => "~1-3 mins",
                }),
            ]),
        ]);
        lines
    } else {
        vec![Line::from(dim("No project selected"))]
    };

    f.render_widget(
        Paragraph::new(text)
            .block(panel("Details"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

pub fn render_action_pane(f: &mut Frame, area: Rect, state: &AppState) {
    let key = |k: &str, label: &str| {
        Line::from(vec![
            Span::styled(format!("{k:>10}"), Style::default().fg(theme::ACCENT)),
            dim(format!("  {label}")),
        ])
    };

    let text = if state.drill_active() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Exploring storage",
                Style::default()
                    .fg(theme::BRAND)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            key("↑/↓", "navigate"),
            key("→", "open folder"),
            key("←", "back"),
            key("q/Esc", "back to projects"),
        ]
    } else {
        let total_size = state.total_selected_size();
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Total reclaimable",
                Style::default()
                    .fg(theme::BODY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format_size(total_size),
                Style::default().fg(theme::OK).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(dim(format!("{} projects selected", state.selected_count()))),
            Line::from(""),
            key("↑/↓", "navigate"),
            key("→", "open project"),
            key("Space", "select"),
            key("Enter", "clean selected"),
            key("s", "sort"),
            key("f", "filter"),
            key("Tab", "list / tree"),
            key("q/Esc", "quit"),
        ]
    };

    f.render_widget(
        Paragraph::new(text)
            .block(panel("Actions"))
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_confirmation_modal(f: &mut Frame, state: &AppState) {
    let selected_count = state.selected_count();
    let total_size = state.total_selected_size();

    let (area, title, border, text) = if selected_count == 0 {
        (
            centered_rect(50, 30, f.area()),
            "Nothing selected",
            theme::BRAND,
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No projects selected",
                    Style::default()
                        .fg(theme::BRAND)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(dim("Pick at least one with Space.")),
                Line::from(""),
                Line::from(dim("Press Enter or Esc to continue")),
            ],
        )
    } else {
        (
            centered_rect(60, 40, f.area()),
            "Confirm",
            theme::DANGER,
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Confirm deletion",
                    Style::default()
                        .fg(theme::DANGER)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    dim("Delete "),
                    Span::styled(
                        format!("{selected_count} projects"),
                        Style::default().fg(theme::BRIGHT),
                    ),
                    dim(" totaling "),
                    Span::styled(format_size(total_size), Style::default().fg(theme::OK)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "This cannot be undone.",
                    Style::default()
                        .fg(theme::DANGER)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    dim("Press "),
                    Span::styled("y", Style::default().fg(theme::OK)),
                    dim(" to confirm, "),
                    Span::styled("n", Style::default().fg(theme::DANGER)),
                    dim(" to cancel"),
                ]),
            ],
        )
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(border).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
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
