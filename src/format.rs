//! Shared byte formatting.
//!
//! Sizes are reported in decimal units (1 kB = 1000 B), matching Finder,
//! `df`, and every other macOS surface a user might compare against. Using
//! binary divisors behind decimal labels made SPEKTR under-report by ~7% at
//! MB and ~10% at GB against the number the user could see in Finder.

const KB: u64 = 1_000;
const MB: u64 = KB * 1_000;
const GB: u64 = MB * 1_000;
const TB: u64 = GB * 1_000;

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

/// Truncates from the left with an ellipsis, so the distinguishing tail of a
/// long name stays visible. The result never exceeds `width` cells, because a
/// row that overflows its column wraps and breaks the alignment of everything
/// to its right.
pub fn truncate(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "\u{2026}".to_string();
    }
    let tail: String = chars[chars.len() - (width - 1)..].iter().collect();
    format!("\u{2026}{tail}")
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
        assert_eq!(truncate("abcdefghij", 5).chars().count(), 5);
        assert_eq!(truncate("abcdefghij", 1), "\u{2026}");
    }

    #[test]
    fn scales_to_terabytes() {
        assert_eq!(format_size(2_500_000_000_000), "2.50 TB");
    }
}
