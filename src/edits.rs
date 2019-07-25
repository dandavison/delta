use std::cmp::{max, min};

use crate::edits::string_pair::StringPair;

/// Infer the edit operations responsible for the differences between
/// a collection of old and new lines.
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
pub fn infer_edit_sections<'m, 'p, EditOperationTag>(
    minus_lines: &'m Vec<String>,
    plus_lines: &'p Vec<String>,
    minus_line_noop: EditOperationTag,
    delete: EditOperationTag,
    plus_line_noop: EditOperationTag,
    insert: EditOperationTag,
    similarity_threshold: f64,
) -> (
    Vec<Vec<(EditOperationTag, &'m str)>>,
    Vec<Vec<(EditOperationTag, &'p str)>>,
)
where
    EditOperationTag: Copy,
{
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
                (minus_line_noop, &minus[0..change_begin]),
                (delete, &minus[change_begin..minus_change_end]),
                (minus_line_noop, &minus[minus_change_end..]),
            ]);
            plus_line_sections.push(vec![
                (plus_line_noop, &plus[0..change_begin]),
                (insert, &plus[change_begin..plus_change_end]),
                (plus_line_noop, &plus[plus_change_end..]),
            ]);
        } else {
            minus_line_sections.push(vec![(minus_line_noop, minus)]);
            plus_line_sections.push(vec![(plus_line_noop, plus)]);
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

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum EditOperationTag {
        MinusNoop,
        PlusNoop,
        Delete,
        Insert,
    }

    use EditOperationTag::*;

    #[test]
    fn test_infer_edit_sections_1() {
        let minus_lines = vec!["aaa\n".to_string()];
        let plus_lines = vec!["aba\n".to_string()];
        let actual_edits = infer_edit_sections(
            &minus_lines,
            &plus_lines,
            MinusNoop,
            Delete,
            PlusNoop,
            Insert,
            1.0,
        );
        let expected_edits = (
            vec![vec![(MinusNoop, "a"), (Delete, "a"), (MinusNoop, "a\n")]],
            vec![vec![(PlusNoop, "a"), (Insert, "b"), (PlusNoop, "a\n")]],
        );

        assert_consistent(&expected_edits);
        assert_consistent(&actual_edits);
        assert_eq!(actual_edits, expected_edits);
    }

    #[test]
    fn test_infer_edit_sections_1_nonascii() {
        let minus_lines = vec!["áaa\n".to_string()];
        let plus_lines = vec!["ááb\n".to_string()];
        let actual_edits = infer_edit_sections(
            &minus_lines,
            &plus_lines,
            MinusNoop,
            Delete,
            PlusNoop,
            Insert,
            1.0,
        );
        let expected_edits = (
            vec![vec![(MinusNoop, "á"), (Delete, "aa"), (MinusNoop, "\n")]],
            vec![vec![(PlusNoop, "á"), (Insert, "áb"), (PlusNoop, "\n")]],
        );

        assert_consistent(&expected_edits);
        assert_consistent(&actual_edits);
        assert_eq!(actual_edits, expected_edits);
    }

    #[test]
    fn test_infer_edit_sections_2() {
        let minus_lines = vec!["d.iteritems()\n".to_string()];
        let plus_lines = vec!["d.items()\n".to_string()];

        let actual_edits = infer_edit_sections(
            &minus_lines,
            &plus_lines,
            MinusNoop,
            Delete,
            PlusNoop,
            Insert,
            1.0,
        );
        let expected_edits = (
            vec![vec![
                (MinusNoop, "d."),
                (Delete, "iter"),
                (MinusNoop, "items()\n"),
            ]],
            vec![vec![
                (PlusNoop, "d."),
                (Insert, ""),
                (PlusNoop, "items()\n"),
            ]],
        );
        assert_consistent(&expected_edits);
        assert_consistent(&actual_edits);
        assert_eq!(actual_edits, expected_edits);
    }

    type EditSection<'a> = (EditOperationTag, &'a str);
    type EditSections<'a> = Vec<EditSection<'a>>;
    type LineEditSections<'a> = Vec<EditSections<'a>>;
    type Edits<'a> = (LineEditSections<'a>, LineEditSections<'a>);

    fn assert_consistent(edits: &Edits) {
        let (minus_line_edit_sections, plus_line_edit_sections) = edits;
        for (minus_edit_sections, plus_edit_sections) in
            minus_line_edit_sections.iter().zip(plus_line_edit_sections)
        {
            let (minus_total, minus_delta) = summarize_edit_sections(minus_edit_sections);
            let (plus_total, plus_delta) = summarize_edit_sections(plus_edit_sections);
            assert_eq!(minus_total - minus_delta, plus_total - plus_delta);
        }
    }

    fn summarize_edit_sections(sections: &EditSections) -> (usize, usize) {
        let mut total = 0;
        let mut delta = 0;
        for (edit, s) in sections {
            total += s.len();
            if is_edit(edit) {
                delta += s.len();
            }
        }
        (total, delta)
    }

    // For debugging test failures:

    #[allow(dead_code)]
    fn compare_edit_sections(actual: Edits, expected: Edits) {
        let (minus, plus) = actual;
        println!("actual minus:");
        print_line_edit_sections(minus);
        println!("actual plus:");
        print_line_edit_sections(plus);

        let (minus, plus) = expected;
        println!("expected minus:");
        print_line_edit_sections(minus);
        println!("expected plus:");
        print_line_edit_sections(plus);
    }

    #[allow(dead_code)]
    fn print_line_edit_sections(line_edit_sections: LineEditSections) {
        for edit_sections in line_edit_sections {
            print_edit_sections(edit_sections);
        }
    }

    #[allow(dead_code)]
    fn print_edit_sections(edit_sections: EditSections) {
        for (edit, s) in edit_sections {
            print!("({} {}), ", fmt_edit(edit), s);
        }
        print!("\n");
    }

    #[allow(dead_code)]
    fn fmt_edit(edit: EditOperationTag) -> &'static str {
        match edit {
            MinusNoop => "MinusNoop",
            Delete => "Delete",
            PlusNoop => "PlusNoop",
            Insert => "Insert",
        }
    }

    fn is_edit(edit: &EditOperationTag) -> bool {
        *edit == Delete || *edit == Insert
    }

}

mod string_pair {
    use itertools::Itertools;
    use unicode_segmentation::UnicodeSegmentation;

    /// A pair of right-trimmed strings.
    pub struct StringPair {
        pub common_prefix_length: usize,
        pub common_suffix_length: usize,
        pub lengths: [usize; 2],
    }

    impl StringPair {
        pub fn new(s0: &str, s1: &str) -> StringPair {
            let (g0, g1) = (s0.grapheme_indices(true), s1.grapheme_indices(true));
            let common_prefix_length = StringPair::common_prefix_length(g0, g1);
            // Watch out! take_while has consumed one additional
            // grapheme, so that needs dealing with if we're going to
            // re-use the same iterators.
            let (g0, g1) = (s0.grapheme_indices(true), s1.grapheme_indices(true));
            let (common_suffix_length, trailing_whitespace) = StringPair::suffix_data(g0, g1);
            StringPair {
                common_prefix_length,
                common_suffix_length,
                lengths: [
                    s0.len() - trailing_whitespace[0],
                    s1.len() - trailing_whitespace[1],
                ],
            }
        }

        /// Align the two strings at their left ends and consider only
        /// the bytes up to the length of the shorter string. Return
        /// the byte offset of the first differing grapheme cluster,
        /// or the byte length of shorter string if they do not
        /// differ.
        fn common_prefix_length<'a, I>(s0: I, s1: I) -> usize
        where
            I: Iterator<Item = (usize, &'a str)>,
        {
            s0.zip(s1)
                .take_while(|((_, c0), (_, c1))| c0 == c1) // TODO: Don't consume one-past-the-end!
                .fold(0, |offset, ((_, c0), (_, _))| offset + c0.len())
        }

        /// Trim trailing whitespace and align the two strings at
        /// their right ends. Fix the origin at their right ends and,
        /// looking left, consider only the bytes up to the length of
        /// the shorter string. Return the byte offset of the first
        /// differing grapheme cluster, or the byte length of the
        /// shorter string if they do not differ. Also return the
        /// number of bytes of whitespace trimmed from each string.
        fn suffix_data<'a, I>(s0: I, s1: I) -> (usize, [usize; 2])
        where
            I: DoubleEndedIterator<Item = (usize, &'a str)>,
            I: Itertools,
        {
            let mut s0 = s0.rev().peekable();
            let mut s1 = s1.rev().peekable();

            let is_whitespace = |(_, c): &(usize, &str)| *c == " " || *c == "\n";
            let n0 = (&mut s0).peeking_take_while(is_whitespace).count();
            let n1 = (&mut s1).peeking_take_while(is_whitespace).count();

            (StringPair::common_prefix_length(s0, s1), [n0, n1])
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

        #[test]
        fn test_common_suffix_length_with_trailing_whitespace_nonascii() {
            assert_eq!(common_suffix_length("  ", "á"), 0);
            assert_eq!(common_suffix_length("á  ", ""), 0);
            assert_eq!(common_suffix_length("á", "b  "), 0);
            assert_eq!(common_suffix_length("á", "á  "), "á".len());
            assert_eq!(common_suffix_length("a  ", "áb  "), 0);
            assert_eq!(common_suffix_length("ab", "á  "), 0);
            assert_eq!(common_suffix_length("áb  ", "b "), 1);
            assert_eq!(common_suffix_length("áb ", "aáb  "), 1 + "á".len());
            assert_eq!(common_suffix_length("abá ", "bá"), 1 + "á".len());
            assert_eq!(
                common_suffix_length("áaáabá ", "ááabá   "),
                2 + 2 * "á".len()
            );
        }
    }
}
