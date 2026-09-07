//! Terminal UI for the storage explorer.

use super::{scan, Browser, Progress, Row};
use crate::format::{format_size, truncate};
use crate::theme;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Runs the explorer against `root`, returning once the user quits.
pub fn run(root: &Path) -> Result<()> {
    let root = root
        .canonicalize()
        .with_context(|| format!("Cannot open {}", root.display()))?;
    anyhow::ensure!(root.is_dir(), "{} is not a directory", root.display());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = browse(&mut terminal, root);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn browse<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, root: PathBuf) -> Result<()> {
    let progress = Arc::new(Progress::default());
    let scan_progress = Arc::clone(&progress);
    let scan_root = root.clone();
    let handle = thread::spawn(move || scan(&scan_root, &scan_progress));

    // Scanning screen, until the walk completes or the user bails out.
    let mut tick = 0usize;
    let mut cancelled = false;
    while !progress.done.load(Ordering::Acquire) {
        terminal.draw(|f| render_scanning(f, &root, &progress, tick))?;
        tick = tick.wrapping_add(1);
        if let Some(key) = poll_key(Duration::from_millis(80))? {
            if matches!(key, Key::Quit) {
                cancelled = true;
                break;
            }
        }
    }

    let sizes = handle
        .join()
        .map_err(|_| anyhow::anyhow!("Scanner thread panicked"))?;
    if cancelled {
        return Ok(());
    }

    let mut browser = Browser::new(root, sizes);
    loop {
        // The viewport decides how many rows fit before the tail is collapsed.
        let height = terminal.size()?.height;
        browser.visible_limit = usize::from(height).saturating_sub(6).max(1);

        terminal.draw(|f| render_browser(f, &browser))?;

        let Some(key) = poll_key(Duration::from_millis(120))? else {
            continue;
        };
        match key {
            Key::Quit => break,
            Key::Up => browser.move_up(),
            Key::Down => browser.move_down(),
            Key::Descend => browser.descend(),
            Key::Ascend => browser.ascend(),
        }
    }
    Ok(())
}

enum Key {
    Quit,
    Up,
    Down,
    Descend,
    Ascend,
}

fn poll_key(timeout: Duration) -> Result<Option<Key>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let Event::Key(key) = event::read()? else {
        return Ok(None);
    };
    // Windows reports both press and release; only act on press.
    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }
    Ok(match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Key::Quit),
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => Some(Key::Quit),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => Some(Key::Up),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => Some(Key::Down),
        (KeyCode::Right, _) | (KeyCode::Char('l'), _) | (KeyCode::Enter, _) => Some(Key::Descend),
        (KeyCode::Left, _) | (KeyCode::Char('h'), _) => Some(Key::Ascend),
        _ => None,
    })
}

fn frame(f: &mut Frame, title_key: &str) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::CHROME));
    let area = f.area();
    let inner = block.inner(area);
    f.render_widget(block, area);
    let _ = title_key;
    inner
}

fn render_scanning(f: &mut Frame, root: &Path, progress: &Progress, tick: usize) {
    let inner = frame(f, "scanning");
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(brand_line(root)), rows[0]);

    let spinner = SPINNER[(tick / 2) % SPINNER.len()];
    let dirs = progress.dirs.load(Ordering::Relaxed);
    let bytes = progress.bytes.load(Ordering::Relaxed);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{spinner} "), Style::default().fg(theme::ACCENT)),
            Span::styled("scanning  ", Style::default().fg(theme::BODY)),
            Span::styled(format_size(bytes), Style::default().fg(theme::OK)),
            Span::styled(
                format!("  ·  {dirs} directories"),
                Style::default().fg(theme::DIM),
            ),
        ])),
        rows[1],
    );

    let current = progress
        .current
        .lock()
        .map(|c| c.clone())
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            truncate(&contract_home(Path::new(&current)), inner.width as usize),
            Style::default().fg(theme::CHROME),
        ))),
        rows[2],
    );

    f.render_widget(Paragraph::new(keys_line(&[("q", "cancel")])), rows[4]);
}

fn render_browser(f: &mut Frame, browser: &Browser) {
    let inner = frame(f, "analyze");
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(brand_line(&browser.cwd)), chunks[0]);

    let total = browser.total();
    let mut summary = vec![
        Span::styled(format_size(total), Style::default().fg(theme::OK)),
        Span::styled(
            format!("  ·  {} items", browser.entries.len()),
            Style::default().fg(theme::DIM),
        ),
    ];
    if !browser.at_root() {
        summary.push(Span::styled(
            "  ·  ← parent",
            Style::default().fg(theme::CHROME),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(summary)), chunks[1]);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(theme::CHROME),
        ))),
        chunks[2],
    );

    render_rows(f, chunks[3], browser, total);

    let mut keys = vec![("↑↓", "navigate"), ("→", "open"), ("←", "back")];
    if matches!(browser.rows().get(browser.cursor), Some(Row::Other { .. })) {
        keys[1] = ("→", "expand");
    }
    keys.push(("q", "quit"));
    f.render_widget(Paragraph::new(keys_line(&keys)), chunks[4]);
}

fn render_rows(f: &mut Frame, area: Rect, browser: &Browser, total: u64) {
    let rows = browser.rows();
    let height = area.height as usize;
    if height == 0 || rows.is_empty() {
        return;
    }

    // Only needed once the tail has been expanded past the viewport.
    let offset = browser.cursor.saturating_sub(height.saturating_sub(1));
    let width = area.width as usize;
    let bar_width = (width / 3).clamp(8, 28);
    let name_width = width
        .saturating_sub(2 + 10 + 3 + 6 + 1 + bar_width + 2)
        .max(6);

    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .map(|(index, row)| {
            let focused = index == browser.cursor;
            let (name, size, is_dir, aggregate) = match row {
                Row::Item(entry) => (entry.name.clone(), entry.size, entry.is_dir, false),
                Row::Other { count, size } => (format!("Other ({count} items)"), *size, true, true),
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

            let bar = theme::bar_spans(share, bar_width);
            Line::from(vec![
                Span::styled(
                    if focused { "▸ " } else { "  " },
                    Style::default().fg(theme::ACCENT),
                ),
                Span::styled(
                    format!("{:<w$}", truncate(&name, name_width), w = name_width),
                    name_style,
                ),
                Span::styled(
                    format!("{:>10}", format_size(size)),
                    Style::default().fg(theme::size_color(size)),
                ),
                Span::styled(" │ ", Style::default().fg(theme::CHROME)),
                Span::styled(format!("{share:>5.1}%"), Style::default().fg(theme::DIM)),
                Span::raw(" "),
                bar.0,
                bar.1,
                Span::styled(
                    if is_dir { " →" } else { "  " },
                    Style::default().fg(theme::CHROME),
                ),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}

fn brand_line(path: &Path) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "SPEKTR",
            Style::default()
                .fg(theme::BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ▸  ", Style::default().fg(theme::CHROME)),
        Span::styled(contract_home(path), Style::default().fg(theme::BRIGHT)),
    ])
}

fn keys_line(keys: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (key, label) in keys {
        if !spans.is_empty() {
            spans.push(Span::styled("  ·  ", Style::default().fg(theme::CHROME)));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(theme::ACCENT),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme::DIM),
        ));
    }
    Line::from(spans)
}

fn contract_home(path: &Path) -> String {
    let display = path.display().to_string();
    let Some(home) = std::env::var_os("HOME") else {
        return display;
    };
    let home = home.to_string_lossy();
    match display.strip_prefix(home.as_ref()) {
        Some("") => "~".to_string(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => display,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_contracted() {
        std::env::set_var("HOME", "/Users/example");
        assert_eq!(contract_home(Path::new("/Users/example")), "~");
        assert_eq!(contract_home(Path::new("/Users/example/code")), "~/code");
        assert_eq!(contract_home(Path::new("/opt/tools")), "/opt/tools");
    }
}
