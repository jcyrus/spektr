//! Shared byte formatting.
//!
//! Sizes are reported in decimal units (1 kB = 1000 B), matching Finder,
//! `df`, and every other macOS surface a user might compare against. Using
//! binary divisors behind decimal labels made SPEKTR under-report by ~7% at
//! MB and ~10% at GB against the number the user could see in Finder.

use unicode_width::UnicodeWidthChar;

const KB: u64 = 1_000;
const MB: u64 = KB * 1_000;
const GB: u64 = MB * 1_000;
const TB: u64 = GB * 1_000;

/// Space a file occupies on disk, which is its block allocation rather than
/// its logical length. A 5,214-byte file occupies two 4 KB blocks, and it is
/// those 8,192 bytes that deleting it gives back -- for trees of many small
/// files (`node_modules` being the obvious case) the two figures diverge
/// sharply, and the logical one understates what a cleanup would reclaim.
/// Windows has no cheap equivalent in `std`, so the logical size stands in
/// there.
#[cfg(unix)]
pub fn allocated_size(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // `blocks()` is always in 512-byte units, whatever the filesystem uses.
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
pub fn allocated_size(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

/// Formats a byte count using decimal SI units.
pub fn format_size(bytes: u64) -> String {
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Display width of one character, in terminal cells. Zero-width marks
/// (combining accents) and wide glyphs (CJK, most emoji) both diverge from a
/// raw `char` count, which is what made the previous implementation of
/// `truncate` overflow or misalign on exactly that input.
fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

/// Display width of a string, in terminal cells.
pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

/// Truncates from the left with an ellipsis, so the distinguishing tail of a
/// long name stays visible. The result never exceeds `width` terminal cells
/// -- measured by display width, not character count -- because a row that
/// overflows its column wraps and breaks the alignment of everything to its
/// right.
pub fn truncate(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    const ELLIPSIS: char = '\u{2026}';
    let budget = width - char_width(ELLIPSIS);

    // Keep chars from the end while they still fit the budget. A character
    // that would overflow it is dropped even if a sliver of budget remains --
    // guaranteeing the result never exceeds `width` matters more than using
    // every last cell.
    let mut tail = String::new();
    let mut used = 0;
    for ch in text.chars().rev() {
        let w = char_width(ch);
        if used + w > budget {
            break;
        }
        tail.push(ch);
        used += w;
    }
    let tail: String = tail.chars().rev().collect();
    format!("{ELLIPSIS}{tail}")
}

/// Pads `text` with trailing spaces to exactly `width` terminal cells. Rust's
/// built-in `{:<width$}` formatting pads by character count, which misaligns
/// columns as soon as a string contains a wide character -- this pads by the
/// same display-width measure `truncate` guarantees its output fits within.
pub fn pad_to_width(text: &str, width: usize) -> String {
    let used = display_width(text);
    if used >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - used))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unit labels must mean what they say. A gibibyte of data is 1.07 GB,
    /// not 1.00 GB -- the previous binary-divisor implementation printed the
    /// latter and disagreed with Finder on every non-trivial directory.
    #[test]
    fn uses_decimal_units_not_binary() {
        assert_eq!(format_size(1_000), "1 KB");
        assert_eq!(format_size(1_000_000), "1.0 MB");
        assert_eq!(format_size(1_000_000_000), "1.00 GB");
        // One gibibyte, the size a binary implementation would call "1.00 GB".
        assert_eq!(format_size(1_073_741_824), "1.07 GB");
    }

    #[test]
    fn small_values_stay_in_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(999), "999 B");
    }

    /// Long names keep their tail, and always fit the column exactly.
    #[test]
    fn truncation_preserves_the_tail() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "\u{2026}ghij");
        assert_eq!(display_width(&truncate("abcdefghij", 5)), 5);
        assert_eq!(truncate("abcdefghij", 1), "\u{2026}");
        assert_eq!(truncate("abcdefghij", 0), "");
    }

    /// Wide glyphs (CJK, most emoji) occupy two terminal cells each. Counting
    /// `chars()` instead of display width let a string like this overflow its
    /// column by as much as its own length in wide characters.
    #[test]
    fn truncation_respects_wide_characters() {
        // Five wide chars = 10 cells; must not overflow a 6-cell budget.
        let wide = "文文文文文";
        let truncated = truncate(wide, 6);
        assert!(
            display_width(&truncated) <= 6,
            "{truncated:?} is {} cells wide, wanted <= 6",
            display_width(&truncated)
        );
    }

    #[test]
    fn padding_accounts_for_display_width() {
        assert_eq!(pad_to_width("ab", 5), "ab   ");
        // "文" is 2 cells wide, so only 3 spaces are needed to reach 5.
        assert_eq!(pad_to_width("文", 5), "文   ");
        assert_eq!(display_width(&pad_to_width("文", 5)), 5);
    }

    #[test]
    fn scales_to_terabytes() {
        assert_eq!(format_size(2_500_000_000_000), "2.50 TB");
    }
}
