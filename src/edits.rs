use std::cmp::{max, min};

use syntect::highlighting::StyleModifier;

use crate::edits::string_pair::StringPair;

/// Create background style sections for a region of removed/added lines.
/*
  This function is called iff a region of n minus lines followed
  by n plus lines is encountered, e.g. n successive lines have
  been partially changed.

  Consider the i-th such line and let m, p be the i-th minus and
  i-th plus line, respectively.  The following cases exist:

  1. Whitespace deleted at line beginning.
     => The deleted section is highlighted in m; p is unstyled.

  2. Whitespace inserted at line beginning.
     => The inserted section is highlighted in p; m is unstyled.

  3. An internal section of the line containing a non-whitespace character has been deleted.
     => The deleted section is highlighted in m; p is unstyled.

  4. An internal section of the line containing a non-whitespace character has been changed.
     => The original section is highlighted in m; the replacement is highlighted in p.

  5. An internal section of the line containing a non-whitespace character has been inserted.
     => The inserted section is highlighted in p; m is unstyled.

  Note that whitespace can be neither deleted nor inserted at the
  end of the line: the line by definition has no trailing
  whitespace.
*/
pub fn get_diff_style_sections(
    minus_lines: &Vec<String>,
    plus_lines: &Vec<String>,
    minus_style_modifier: StyleModifier,
    minus_emph_style_modifier: StyleModifier,
    plus_style_modifier: StyleModifier,
    plus_emph_style_modifier: StyleModifier,
    similarity_threshold: f64,
) -> (
    Vec<Vec<(StyleModifier, String)>>,
    Vec<Vec<(StyleModifier, String)>>,
) {
    let mut minus_line_sections = Vec::new();
    let mut plus_line_sections = Vec::new();

    for (minus, plus) in minus_lines.iter().zip(plus_lines.iter()) {
        let string_pair = StringPair::new(minus, plus);

        // We require that (right-trimmed length) >= (common prefix length). Consider:
        // minus = "a    "
        // plus  = "a b  "
        // Here, the right-trimmed length of minus is 1, yet the common prefix length is
        // 2. We resolve this by taking the following maxima:
        let minus_length = max(string_pair.lengths[0], string_pair.common_prefix_length);
        let plus_length = max(string_pair.lengths[1], string_pair.common_prefix_length);

        // Work backwards from the end of the strings. The end of the
        // change region is equal to the start of their common
        // suffix. To find the start of the change region, start with
        // the end of their common prefix, and then move leftwards
        // until it is before the start of the common suffix in both
        // strings.
        let minus_change_end = minus_length - string_pair.common_suffix_length;
        let plus_change_end = plus_length - string_pair.common_suffix_length;
        let change_begin = min(
            string_pair.common_prefix_length,
            min(minus_change_end, plus_change_end),
        );

        let minus_edit = Edit {
            change_begin,
            change_end: minus_change_end,
            string_length: minus_length,
        };
        let plus_edit = Edit {
            change_begin,
            change_end: plus_change_end,
            string_length: plus_length,
        };

        if minus_edit.appears_genuine(similarity_threshold)
            && plus_edit.appears_genuine(similarity_threshold)
        {
            minus_line_sections.push(vec![
                (minus_style_modifier, minus[0..change_begin].to_string()),
                (
                    minus_emph_style_modifier,
                    minus[change_begin..minus_change_end].to_string(),
                ),
                (minus_style_modifier, minus[minus_change_end..].to_string()),
            ]);
            plus_line_sections.push(vec![
                (plus_style_modifier, plus[0..change_begin].to_string()),
                (
                    plus_emph_style_modifier,
                    plus[change_begin..plus_change_end].to_string(),
                ),
                (plus_style_modifier, plus[plus_change_end..].to_string()),
            ]);
        } else {
            minus_line_sections.push(vec![(minus_style_modifier, minus.to_string())]);
            plus_line_sections.push(vec![(plus_style_modifier, plus.to_string())]);
        }
    }
    (minus_line_sections, plus_line_sections)
}

struct Edit {
    change_begin: usize,
    change_end: usize,
    string_length: usize,
}

impl Edit {
    // TODO: exclude leading whitespace in this calculation
    fn appears_genuine(&self, similarity_threshold: f64) -> bool {
        ((self.change_end - self.change_begin) as f64 / self.string_length as f64)
            < similarity_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntect::highlighting::{Color, FontStyle};

    #[test]
    fn test_get_diff_style_sections_1() {
        let actual_edits = get_diff_style_sections(
            &vec!["aaa\n".to_string()],
            &vec!["aba\n".to_string()],
            MINUS,
            MINUS_EMPH,
            PLUS,
            PLUS_EMPH,
            1.0,
        );
        let expected_edits = (
            vec![as_strings(vec![
                (MINUS, "a"),
                (MINUS_EMPH, "a"),
                (MINUS, "a\n"),
            ])],
            vec![as_strings(vec![
                (PLUS, "a"),
                (PLUS_EMPH, "b"),
                (PLUS, "a\n"),
            ])],
        );

        assert_consistent(&expected_edits);
        assert_consistent(&actual_edits);
        assert_eq!(actual_edits, expected_edits);
    }

    #[test]
    fn test_get_diff_style_sections_2() {
        let actual_edits = get_diff_style_sections(
            &vec!["d.iteritems()\n".to_string()],
            &vec!["d.items()\n".to_string()],
            MINUS,
            MINUS_EMPH,
            PLUS,
            PLUS_EMPH,
            1.0,
        );
        let expected_edits = (
            vec![as_strings(vec![
                (MINUS, "d."),
                (MINUS_EMPH, "iter"),
                (MINUS, "items()\n"),
            ])],
            vec![as_strings(vec![
                (PLUS, "d."),
                (PLUS_EMPH, ""),
                (PLUS, "items()\n"),
            ])],
        );
        assert_consistent(&expected_edits);
        assert_consistent(&actual_edits);
        assert_eq!(actual_edits, expected_edits);
    }

    type StyleSection = (StyleModifier, String);
    type StyleSections = Vec<StyleSection>;
    type LineStyleSections = Vec<StyleSections>;
    type Edits = (LineStyleSections, LineStyleSections);

    fn assert_consistent(edits: &Edits) {
        let (minus_line_style_sections, plus_line_style_sections) = edits;
        for (minus_style_sections, plus_style_sections) in minus_line_style_sections
            .iter()
            .zip(plus_line_style_sections)
        {
            let (minus_total, minus_delta) = summarize_style_sections(minus_style_sections);
            let (plus_total, plus_delta) = summarize_style_sections(plus_style_sections);
            assert_eq!(minus_total - minus_delta, plus_total - plus_delta);
        }
    }

    fn summarize_style_sections(sections: &StyleSections) -> (usize, usize) {
        let mut total = 0;
        let mut delta = 0;
        for (style, s) in sections {
            total += s.len();
            if is_emph(style) {
                delta += s.len();
            }
        }
        (total, delta)
    }

    const RED: Color = Color::BLACK;
    const GREEN: Color = Color::WHITE;

    const MINUS: StyleModifier = StyleModifier {
        foreground: None,
        background: Some(RED),
        font_style: None,
    };

    const MINUS_EMPH: StyleModifier = StyleModifier {
        foreground: None,
        background: Some(RED),
        font_style: Some(FontStyle::BOLD),
    };

    const PLUS: StyleModifier = StyleModifier {
        foreground: None,
        background: Some(GREEN),
        font_style: None,
    };

    const PLUS_EMPH: StyleModifier = StyleModifier {
        foreground: None,
        background: Some(GREEN),
        font_style: Some(FontStyle::BOLD),
    };

    fn as_strings(sections: Vec<(StyleModifier, &str)>) -> StyleSections {
        let mut new_sections = Vec::new();
        for (style, s) in sections {
            new_sections.push((style, s.to_string()));
        }
        new_sections
    }

    // For debugging test failures:

    #[allow(dead_code)]
    fn compare_style_sections(actual: Edits, expected: Edits) {
        let (minus, plus) = actual;
        println!("actual minus:");
        print_line_style_sections(minus);
        println!("actual plus:");
        print_line_style_sections(plus);

        let (minus, plus) = expected;
        println!("expected minus:");
        print_line_style_sections(minus);
        println!("expected plus:");
        print_line_style_sections(plus);
    }

    #[allow(dead_code)]
    fn print_line_style_sections(line_style_sections: LineStyleSections) {
        for style_sections in line_style_sections {
            print_style_sections(style_sections);
        }
    }

    #[allow(dead_code)]
    fn print_style_sections(style_sections: StyleSections) {
        for (style, s) in style_sections {
            print!("({} {}), ", fmt_style(style), s);
        }
        print!("\n");
    }

    #[allow(dead_code)]
    fn fmt_style(style: StyleModifier) -> &'static str {
        match (style.background.unwrap(), style.font_style) {
            (RED, None) => "MINUS",
            (RED, _) => "MINUS_EMPH",
            (GREEN, None) => "PLUS",
            (GREEN, _) => "PLUS_EMPH",
            _ => panic!(),
        }
    }

    fn is_emph(style: &StyleModifier) -> bool {
        style.font_style.is_some()
    }

}

mod string_pair {
    use std::iter::Peekable;

    /// A pair of right-trimmed strings.
    pub struct StringPair {
        pub common_prefix_length: usize,
        pub common_suffix_length: usize,
        pub lengths: [usize; 2],
    }

    impl StringPair {
        pub fn new(s0: &str, s1: &str) -> StringPair {
            let common_prefix_length = StringPair::common_prefix_length(s0.chars(), s1.chars());
            let (common_suffix_length, trailing_whitespace) =
                StringPair::suffix_data(s0.chars(), s1.chars());
            StringPair {
                common_prefix_length,
                common_suffix_length,
                lengths: [
                    s0.len() - trailing_whitespace[0],
                    s1.len() - trailing_whitespace[1],
                ],
            }
        }

        fn common_prefix_length(
            s0: impl Iterator<Item = char>,
            s1: impl Iterator<Item = char>,
        ) -> usize {
            let mut i = 0;
            for (c0, c1) in s0.zip(s1) {
                if c0 != c1 {
                    break;
                } else {
                    i += 1;
                }
            }
            i
        }

        /// Return common suffix length and number of trailing whitespace characters on each string.
        fn suffix_data(
            s0: impl DoubleEndedIterator<Item = char>,
            s1: impl DoubleEndedIterator<Item = char>,
        ) -> (usize, [usize; 2]) {
            let mut s0 = s0.rev().peekable();
            let mut s1 = s1.rev().peekable();
            let n0 = StringPair::consume_whitespace(&mut s0);
            let n1 = StringPair::consume_whitespace(&mut s1);

            (StringPair::common_prefix_length(s0, s1), [n0, n1])
        }

        /// Consume leading whitespace; return number of characters consumed.
        fn consume_whitespace(s: &mut Peekable<impl Iterator<Item = char>>) -> usize {
            let mut i = 0;
            loop {
                match s.peek() {
                    Some('\n') | Some(' ') => {
                        s.next();
                        i += 1;
                    }
                    _ => break,
                }
            }
            i
        }
    }

    #[cfg(test)]
    mod tests {
        fn common_prefix_length(s1: &str, s2: &str) -> usize {
            super::StringPair::new(s1, s2).common_prefix_length
        }

        fn common_suffix_length(s1: &str, s2: &str) -> usize {
            super::StringPair::new(s1, s2).common_suffix_length
        }

        #[test]
        fn test_common_prefix_length() {
            assert_eq!(common_prefix_length("", ""), 0);
            assert_eq!(common_prefix_length("", "a"), 0);
            assert_eq!(common_prefix_length("a", ""), 0);
            assert_eq!(common_prefix_length("a", "b"), 0);
            assert_eq!(common_prefix_length("a", "a"), 1);
            assert_eq!(common_prefix_length("a", "ab"), 1);
            assert_eq!(common_prefix_length("ab", "a"), 1);
            assert_eq!(common_prefix_length("ab", "aba"), 2);
            assert_eq!(common_prefix_length("aba", "ab"), 2);
        }

        #[test]
        fn test_common_prefix_length_with_leading_whitespace() {
            assert_eq!(common_prefix_length(" ", ""), 0);
            assert_eq!(common_prefix_length(" ", " "), 1);
            assert_eq!(common_prefix_length(" a", " a"), 2);
            assert_eq!(common_prefix_length(" a", "a"), 0);
        }

        #[test]
        fn test_common_suffix_length() {
            assert_eq!(common_suffix_length("", ""), 0);
            assert_eq!(common_suffix_length("", "a"), 0);
            assert_eq!(common_suffix_length("a", ""), 0);
            assert_eq!(common_suffix_length("a", "b"), 0);
            assert_eq!(common_suffix_length("a", "a"), 1);
            assert_eq!(common_suffix_length("a", "ab"), 0);
            assert_eq!(common_suffix_length("ab", "a"), 0);
            assert_eq!(common_suffix_length("ab", "b"), 1);
            assert_eq!(common_suffix_length("ab", "aab"), 2);
            assert_eq!(common_suffix_length("aba", "ba"), 2);
        }

        #[test]
        fn test_common_suffix_length_with_trailing_whitespace() {
            assert_eq!(common_suffix_length("", "  "), 0);
            assert_eq!(common_suffix_length("  ", "a"), 0);
            assert_eq!(common_suffix_length("a  ", ""), 0);
            assert_eq!(common_suffix_length("a", "b  "), 0);
            assert_eq!(common_suffix_length("a", "a  "), 1);
            assert_eq!(common_suffix_length("a  ", "ab  "), 0);
            assert_eq!(common_suffix_length("ab", "a  "), 0);
            assert_eq!(common_suffix_length("ab  ", "b "), 1);
            assert_eq!(common_suffix_length("ab ", "aab  "), 2);
            assert_eq!(common_suffix_length("aba ", "ba"), 2);
        }
    }
}
