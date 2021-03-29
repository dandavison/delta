mod console_tests;
mod iterator;

use std::borrow::Cow;

use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use iterator::{AnsiElementIterator, Element};

pub const ANSI_CSI_CLEAR_TO_EOL: &str = "\x1b[0K";
pub const ANSI_CSI_CLEAR_TO_BOL: &str = "\x1b[1K";
pub const ANSI_SGR_RESET: &str = "\x1b[0m";

pub fn strip_ansi_codes(s: &str) -> String {
    strip_ansi_codes_from_strings_iterator(ansi_strings_iterator(s))
}

pub fn measure_text_width(s: &str) -> usize {
    // TODO: how should e.g. '\n' be handled?
    strip_ansi_codes(s).width()
}

/// Truncate string such that `tail` is present as a suffix, preceded by as much of `s` as can be
/// displayed in the requested width.
// Return string constructed as follows:
// 1. `display_width` characters are available. If the string fits, return it.
//
// 2. Contribute graphemes and ANSI escape sequences from `tail` until either (1) `tail` is
//    exhausted, or (2) the display width of the result would exceed `display_width`.
//
// 3. If tail was exhausted, then contribute graphemes and ANSI escape sequences from `s` until the
//    display_width of the result would exceed `display_width`.
pub fn truncate_str<'a, 'b>(s: &'a str, display_width: usize, tail: &'b str) -> Cow<'a, str> {
    let items = ansi_strings_iterator(s).collect::<Vec<(&str, bool)>>();
    let width = strip_ansi_codes_from_strings_iterator(items.iter().copied()).width();
    if width <= display_width {
        return Cow::from(s);
    }
    let result_tail = if !tail.is_empty() {
        truncate_str(tail, display_width, "").to_string()
    } else {
        String::new()
    };
    let mut used = measure_text_width(&result_tail);
    let mut result = String::new();
    for (t, is_ansi) in items {
        if !is_ansi {
            for g in t.graphemes(true) {
                let w = g.width();
                if used + w > display_width {
                    break;
                }
                result.push_str(g);
                used += w;
            }
        } else {
            result.push_str(t);
        }
    }

    Cow::from(format!("{}{}", result, result_tail))
}

pub fn parse_first_style(s: &str) -> Option<ansi_term::Style> {
    AnsiElementIterator::new(s).find_map(|el| match el {
        Element::Csi(style, _, _) => Some(style),
        _ => None,
    })
}

pub fn string_starts_with_ansi_style_sequence(s: &str) -> bool {
    AnsiElementIterator::new(s)
        .next()
        .map(|el| matches!(el, Element::Csi(_, _, _)))
        .unwrap_or(false)
}

/// Return string formed from a byte slice starting at byte position `start`, where the index
/// counts bytes in non-ANSI-escape-sequence content only. All ANSI escape sequences in the
/// original string are preserved.
pub fn ansi_preserving_slice(s: &str, start: usize) -> String {
    AnsiElementIterator::new(s)
        .scan(0, |index, element| {
            // `index` is the index in non-ANSI-escape-sequence content.
            Some(match element {
                Element::Csi(_, a, b) => &s[a..b],
                Element::Esc(a, b) => &s[a..b],
                Element::Osc(a, b) => &s[a..b],
                Element::Text(a, b) => {
                    let i = *index;
                    *index += b - a;
                    if *index <= start {
                        // This text segment ends before start, so contributes no bytes.
                        ""
                    } else if i > start {
                        // This section starts after `start`, so contributes all its bytes.
                        &s[a..b]
                    } else {
                        // This section contributes those bytes that are >= start
                        &s[(a + start - i)..b]
                    }
                }
            })
        })
        .join("")
}

fn ansi_strings_iterator(s: &str) -> impl Iterator<Item = (&str, bool)> {
    AnsiElementIterator::new(s).map(move |el| match el {
        Element::Csi(_, i, j) => (&s[i..j], true),
        Element::Esc(i, j) => (&s[i..j], true),
        Element::Osc(i, j) => (&s[i..j], true),
        Element::Text(i, j) => (&s[i..j], false),
    })
}

fn strip_ansi_codes_from_strings_iterator<'a>(
    strings: impl Iterator<Item = (&'a str, bool)>,
) -> String {
    strings
        .filter_map(|(el, is_ansi)| if !is_ansi { Some(el) } else { None })
        .join("")
}

#[cfg(test)]
mod tests {

    use super::{
        ansi_preserving_slice, measure_text_width, parse_first_style,
        string_starts_with_ansi_style_sequence, strip_ansi_codes,
    };

    #[test]
    fn test_strip_ansi_codes() {
        for s in &["src/ansi/mod.rs", "バー", "src/ansi/modバー.rs"] {
            assert_eq!(strip_ansi_codes(s), *s);
        }
        assert_eq!(strip_ansi_codes("\x1b[31mバー\x1b[0m"), "バー");
    }

    #[test]
    fn test_measure_text_width() {
        assert_eq!(measure_text_width("src/ansi/mod.rs"), 15);
        assert_eq!(measure_text_width("バー"), 4);
        assert_eq!(measure_text_width("src/ansi/modバー.rs"), 19);
        assert_eq!(measure_text_width("\x1b[31mバー\x1b[0m"), 4);
        assert_eq!(measure_text_width("a\nb\n"), 2);
    }

    #[test]
    fn test_strip_ansi_codes_osc_hyperlink() {
        assert_eq!(strip_ansi_codes("\x1b[38;5;4m\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b\\src/ansi/mod.rs\x1b]8;;\x1b\\\x1b[0m\n"),
                   "src/ansi/mod.rs\n");
    }

    #[test]
    fn test_measure_text_width_osc_hyperlink() {
        assert_eq!(measure_text_width("\x1b[38;5;4m\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b\\src/ansi/mod.rs\x1b]8;;\x1b\\\x1b[0m"),
                   measure_text_width("src/ansi/mod.rs"));
    }

    #[test]
    fn test_measure_text_width_osc_hyperlink_non_ascii() {
        assert_eq!(measure_text_width("\x1b[38;5;4m\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b\\src/ansi/modバー.rs\x1b]8;;\x1b\\\x1b[0m"),
                   measure_text_width("src/ansi/modバー.rs"));
    }

    #[test]
    fn test_parse_first_style() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let style = parse_first_style(minus_line_from_unconfigured_git);
        let expected_style = ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            ..ansi_term::Style::default()
        };
        assert_eq!(Some(expected_style), style);
    }

    #[test]
    fn test_string_starts_with_ansi_escape_sequence() {
        assert!(!string_starts_with_ansi_style_sequence(""));
        assert!(!string_starts_with_ansi_style_sequence("-"));
        assert!(string_starts_with_ansi_style_sequence(
            "\x1b[31m-XXX\x1b[m\n"
        ));
        assert!(string_starts_with_ansi_style_sequence("\x1b[32m+XXX"));
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
