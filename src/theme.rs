//! SPEKTR colour palette.
//!
//! "Synthwave": a saturated magenta/violet scheme on an indigo chrome, with
//! amber reserved for emphasis and a single unambiguous danger colour. Kept in
//! one place so the whole TUI can be re-themed without touching widget code.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Frame lines, column rules, and the unfilled portion of bars.
pub const CHROME: Color = Color::Rgb(0x43, 0x28, 0x6B);
/// De-emphasised text: hints, key names, percentages.
pub const DIM: Color = Color::Rgb(0x8A, 0x6F, 0xB0);
/// Default body text.
pub const BODY: Color = Color::Rgb(0xC6, 0xB3, 0xE0);
/// Focused row text and headline values.
pub const BRIGHT: Color = Color::Rgb(0xFF, 0xF0, 0xFB);
/// Cursor, selection marks, active affordances.
pub const ACCENT: Color = Color::Rgb(0xFF, 0x48, 0xC4);
/// Brand / emphasis highlight.
pub const BRAND: Color = Color::Rgb(0xFF, 0xB0, 0x00);
/// Destructive actions. Reserved -- nothing else uses this colour.
pub const DANGER: Color = Color::Rgb(0xFF, 0x38, 0x64);
/// Success and reclaimable totals.
pub const OK: Color = Color::Rgb(0x37, 0xE5, 0xB6);

/// Bar segments, hottest (largest share) to coolest.
pub const BAR_HOT: Color = Color::Rgb(0xFF, 0x48, 0xC4);
pub const BAR_MID: Color = Color::Rgb(0xC4, 0x3B, 0xFF);
pub const BAR_COOL: Color = Color::Rgb(0x7B, 0x4D, 0xFF);

/// Colour for a bar, chosen by the row's share of its parent.
pub fn bar_color(share: f64) -> Color {
    if share >= 50.0 {
        BAR_HOT
    } else if share >= 10.0 {
        BAR_MID
    } else {
        BAR_COOL
    }
}

/// Colour for a size figure, chosen by magnitude, so a 12 GB directory reads
/// as heavy before the number itself is parsed.
pub fn size_color(bytes: u64) -> Color {
    const GB: u64 = 1_000_000_000;
    const MB: u64 = 1_000_000;
    if bytes >= 10 * GB {
        DANGER
    } else if bytes >= GB {
        ACCENT
    } else if bytes >= 100 * MB {
        BRAND
    } else {
        BODY
    }
}

/// Filled cells for a bar of `width`, keeping a one-cell floor so a 0.0% row
/// still reads as a row rather than looking broken.
pub fn bar_cells(share: f64, width: usize) -> usize {
    ((share / 100.0 * width as f64).round() as usize).clamp(1, width.max(1))
}

/// The two spans making up a proportion bar: filled, then unfilled.
pub fn bar_spans(share: f64, width: usize) -> (Span<'static>, Span<'static>) {
    let filled = bar_cells(share, width);
    (
        Span::styled(
            "\u{2588}".repeat(filled),
            Style::default().fg(bar_color(share)),
        ),
        Span::styled(
            "\u{2591}".repeat(width.saturating_sub(filled)),
            Style::default().fg(CHROME),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_stays_within_bounds() {
        assert_eq!(bar_cells(0.0, 28), 1, "zero still shows one block");
        assert_eq!(bar_cells(100.0, 28), 28);
        assert!(bar_cells(150.0, 28) <= 28, "cannot exceed the bar width");
    }
}
