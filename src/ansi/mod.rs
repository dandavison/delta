pub mod parse;

use std::cmp::min;

use console;
use itertools::Itertools;

pub const ANSI_CSI_CLEAR_TO_EOL: &str = "\x1b[0K";
pub const ANSI_CSI_CLEAR_TO_BOL: &str = "\x1b[1K";
pub const ANSI_SGR_RESET: &str = "\x1b[0m";

pub fn string_starts_with_ansi_escape_sequence(s: &str) -> bool {
    console::AnsiCodeIterator::new(s)
        .nth(0)
        .map(|(_, is_ansi)| is_ansi)
        .unwrap_or(false)
}

/// Return string formed from a byte slice starting at byte position `start`, where the index
/// counts bytes in non-ANSI-escape-sequence content only. All ANSI escape sequences in the
/// original string are preserved.
pub fn ansi_preserving_slice(s: &str, start: usize) -> String {
    console::AnsiCodeIterator::new(s)
        .scan(0, |i, (substring, is_ansi)| {
            // i is the index in non-ANSI-escape-sequence content.
            let substring_slice = if is_ansi || *i > start {
                substring
            } else {
                &substring[min(substring.len(), start - *i)..]
            };
            if !is_ansi {
                *i += substring.len();
            }
            Some(substring_slice)
        })
        .join("")
}

#[cfg(test)]
mod tests {

    use crate::ansi::ansi_preserving_slice;
    use crate::ansi::string_starts_with_ansi_escape_sequence;

    #[test]
    fn test_string_starts_with_ansi_escape_sequence() {
        assert!(!string_starts_with_ansi_escape_sequence(""));
        assert!(!string_starts_with_ansi_escape_sequence("-"));
        assert!(string_starts_with_ansi_escape_sequence(
            "\x1b[31m-XXX\x1b[m\n"
        ));
        assert!(string_starts_with_ansi_escape_sequence("\x1b[32m+XXX"));
    }

    #[test]
    fn test_ansi_preserving_slice() {
        assert_eq!(ansi_preserving_slice("", 0), "");
        assert_eq!(ansi_preserving_slice("a", 0), "a");
        assert_eq!(ansi_preserving_slice("a", 1), "");
        assert_eq!(
            ansi_preserving_slice("\x1b[1;35m-2222.2222.2222.2222\x1b[0m", 1),
            "\x1b[1;35m2222.2222.2222.2222\x1b[0m"
        );
        assert_eq!(
            ansi_preserving_slice("\x1b[1;35m-2222.2222.2222.2222\x1b[0m", 15),
            "\x1b[1;35m.2222\x1b[0m"
        );
        assert_eq!(
            ansi_preserving_slice("\x1b[1;36m-\x1b[m\x1b[1;36m2222·2222·2222·2222\x1b[m\n", 1),
            "\x1b[1;36m\x1b[m\x1b[1;36m2222·2222·2222·2222\x1b[m\n"
        )
    }
}
