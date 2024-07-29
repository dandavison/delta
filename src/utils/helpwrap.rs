#![allow(clippy::comparison_to_empty)] // no_indent != "", instead of !no_indent.is_empty()

use crate::ansi::measure_text_width;

/// Wrap `text` at spaces ('` `') to fit into `width`. If `indent_with` is non-empty, indent
/// each line with this string. If a line from `text` starts with `no_indent`, do not indent.
/// If a line starts with `no_wrap`, do not wrap (empty `no_indent`/`no_wrap` have no effect).
/// If both "magic prefix" markers are used, `no_indent` must be first.
/// Takes unicode and ANSI into account when calculating width, but won't wrap ANSI correctly.
/// Removes trailing spaces. Leading spaces or enumerations with '- ' continue the indentation on
/// the wrapped line.
/// Example:
/// ```
/// let wrapped = wrap("ab cd ef\n!NI!123\n|AB CD EF GH\n!NI!|123 456 789", 7, "_", "!NI!", "|");
/// assert_eq!(wrapped, "\
///     _ab cd\n\
///     _ef\n\
///     123\n\
///     _AB CD EF GH\n\
///     123 456 789\n\
///     ");
/// ```
pub fn wrap(text: &str, width: usize, indent_with: &str, no_indent: &str, no_wrap: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let indent_len = measure_text_width(indent_with);

    for line in text.lines() {
        let line = line.trim_end_matches(' ');

        let (line, indent) =
            if let (Some(line), true) = (line.strip_prefix(no_indent), no_indent != "") {
                (line, "")
            } else {
                result.push_str(indent_with);
                (line, indent_with)
            };

        if let (Some(line), true) = (line.strip_prefix(no_wrap), no_wrap != "") {
            result.push_str(line);
        } else {
            // `"foo bar   end".split_inclusive(' ')` => `["foo ", "bar ", " ", " ", "end"]`
            let mut wordit = line.split_inclusive(' ');
            let mut curr_len = indent_len;

            if let Some(word) = wordit.next() {
                result.push_str(word);
                curr_len += measure_text_width(word);
            }

            while let Some(mut word) = wordit.next() {
                let word_len = measure_text_width(word);
                if curr_len + word_len == width + 1 && word.ends_with(' ') {
                    // If just ' ' is over the limit, let the next word trigger the overflow.
                } else if curr_len + word_len > width {
                    // Remove any trailing whitespace:
                    let pos = result.trim_end_matches(' ').len();
                    result.truncate(pos);

                    result.push('\n');

                    // Do not count spaces, skip until next proper word is found.
                    if word == " " {
                        for nextword in wordit.by_ref() {
                            word = nextword;
                            if word != " " {
                                break;
                            }
                        }
                    }

                    // Re-calculates indent for each wrapped line. Could be done only once, maybe
                    // after an early return which just uses .len() (works for fullwidth chars).

                    // If line started with spaces, indent by that much again.
                    let (indent, space_pos) =
                        if let Some(space_prefix_len) = line.find(|c: char| c != ' ') {
                            (
                                format!("{}{}", indent, " ".repeat(space_prefix_len)),
                                space_prefix_len,
                            )
                        } else {
                            debug_assert!(false, "line.trim_end_matches() missing?");
                            (indent.to_string(), 0)
                        };

                    // If line started with '- ', treat it as a bullet point and increase indentation
                    let indent = if line[space_pos..].starts_with("- ") {
                        format!("{}{}", indent, "  ")
                    } else {
                        indent
                    };

                    result.push_str(&indent);
                    curr_len = measure_text_width(&indent);
                }
                curr_len += word_len;
                result.push_str(word);
            }
        }
        let pos = result.trim_end_matches(' ').len();
        result.truncate(pos);
        result.push('\n');
    }

    #[cfg(test)]
    if result.find("no-sanity").is_none() {
        // sanity check
        let stripped_input = text
            .replace(" ", "")
            .replace("\n", "")
            .replace(no_wrap, "")
            .replace(no_indent, "");
        let stripped_output = result
            .replace(" ", "")
            .replace("\n", "")
            .replace(indent_with, "");
        assert_eq!(stripped_input, stripped_output);
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn simple_ascii_can_not_split() {
        let input = "000 123456789 abcdefghijklmnopqrstuvwxyz ok";
        let result = wrap(input, 5, "", "", "");
        assert_snapshot!(result, @r###"
        000
        123456789
        abcdefghijklmnopqrstuvwxyz
        ok
        "###);
    }

    #[test]
    fn simple_ascii_just_whitespace() {
        let input = "               \n   \n           \n  \n \n     \n";
        let result = wrap(input, 3, "__", "", "");
        assert_snapshot!(result, @r###"
        __
        __
        __
        __
        __
        __
        "###);
        let result = wrap(input, 3, "", "", "");
        assert_eq!(result, "\n\n\n\n\n\n");
    }

    #[test]
    fn simple_ascii_can_not_split_plus_whitespace() {
        let input = "000        123456789          abcdefghijklmnopqrstuvwxyz          ok";
        let result = wrap(input, 5, "", "", "");
        assert_snapshot!(result, @r###"
        000
        123456789
        abcdefghijklmnopqrstuvwxyz
        ok
        "###);
    }

    #[test]
    fn simple_ascii_keep_leading_input_indent() {
        let input = "abc\n  Def ghi jkl mno pqr stuv xyz\n    Abc def ghijklm\nok";
        let result = wrap(input, 10, "_", "", "");
        assert_snapshot!(result, @r###"
        _abc
        _  Def ghi
        _  jkl mno
        _  pqr
        _  stuv
        _  xyz
        _    Abc
        _    def
        _    ghijklm
        _ok
        "###);
    }

    #[test]
    fn simple_ascii_indent_and_bullet_points() {
        let input = "- ABC ABC abc\n   def ghi - jkl\n  - 1 22 3 4 55 6 7 8 9\n    - 1 22 3 4 55 6 7 8 9\n!- 0 0 0 0 0 0 0 \n";
        let result = wrap(input, 10, "", "!", "");
        assert_snapshot!(result, @r###"
        - ABC ABC
          abc
           def ghi
           - jkl
          - 1 22 3
            4 55 6
            7 8 9
            - 1 22
              3 4
              55 6
              7 8
              9
        - 0 0 0 0
          0 0 0
        "###);
    }

    #[test]
    fn simple_ascii_all_overlong_after_indent() {
        let input = "0000 1111 2222";
        let result = wrap(input, 5, "__", "", "");
        assert_snapshot!(result, @r###"
        __0000
        __1111
        __2222
        "###);
    }

    #[test]
    fn simple_ascii_one_line() {
        let input = "123 456 789 abc def ghi jkl mno pqr stu vwx yz";
        let result = wrap(input, 10, "__", "", "");
        assert_snapshot!(result, @r###"
        __123 456
        __789 abc
        __def ghi
        __jkl mno
        __pqr stu
        __vwx yz
        "###);
    }

    #[test]
    fn simple_ascii_trailing_space() {
        let input = "123  \n\n   \n  456   \n     a  b \n\n";
        let result = wrap(input, 10, "    ", "", "");
        assert_eq!(result, "    123\n\n\n      456\n         a\n         b\n\n");
    }

    #[test]
    fn simple_ascii_two_lines() {
        let input = "123 456 789 abc def\nghi jkl mno pqr stu vwx yz\n1234 567 89 876 54321\n";
        let result = wrap(input, 10, "__", "", "");
        assert_snapshot!(result, @r###"
        __123 456
        __789 abc
        __def
        __ghi jkl
        __mno pqr
        __stu vwx
        __yz
        __1234 567
        __89 876
        __54321
        "###);
    }

    #[test]
    fn simple_ascii_no_indent() {
        let input = "123 456 789\n!!abc def ghi jkl mno pqr\nstu vwx yz\n\n";
        let result = wrap(input, 10, "__", "!!", "");
        assert_snapshot!(result, @r###"
        __123 456
        __789
        abc def
        ghi jkl
        mno pqr
        __stu vwx
        __yz
        __
        "###);
    }

    #[test]
    fn simple_ascii_no_wrap() {
        let input = "123 456 789\n|abc def ghi jkl mno pqr\nstu vwx yz\n|W\nA B C D E F G H I\n";
        let result = wrap(input, 10, "__", "!!", "|");
        assert_snapshot!(result, @r###"
        __123 456
        __789
        __abc def ghi jkl mno pqr
        __stu vwx
        __yz
        __W
        __A B C D
        __E F G H
        __I
        "###);
    }

    #[test]
    fn simple_ascii_no_both() {
        let input = "123 456 789\n!!|abc def ghi jkl mno pqr\nstu vwx yz\n|W\nA B C D E F G H I\n";
        let result = wrap(input, 10, "__", "!!", "|");
        assert_snapshot!(result, @r###"
        __123 456
        __789
        abc def ghi jkl mno pqr
        __stu vwx
        __yz
        __W
        __A B C D
        __E F G H
        __I
        "###);
    }

    #[test]
    fn simple_ascii_no_both_wrong_order() {
        let input = "!!|abc def ghi jkl\n|!!ABC DEF GHI JKL + no-sanity\n";
        let result = wrap(input, 7, "__", "!!", "|");
        assert_snapshot!(result, @r###"
        abc def ghi jkl
        __!!ABC DEF GHI JKL + no-sanity
        "###);
        let wrapped = wrap(
            "ab cd ef\n!NI!123\n|AB CD EF GH\n!NI!|123 456 789",
            6,
            "_",
            "!NI!",
            "|",
        );
        assert_snapshot!(wrapped, @r###"
        _ab cd
        _ef
        123
        _AB CD EF GH
        123 456 789
        "###);
    }

    #[test]
    fn simple_ascii_much_whitespace() {
        let input = "123       456       789\nabc   def  ghi    jkl   mno  pqr    \nstu   vwx yz";
        let result = wrap(input, 10, "__", "!!", "|");
        assert_snapshot!(result, @r###"
        __123
        __456
        __789
        __abc
        __def  ghi
        __jkl   mno
        __pqr
        __stu
        __vwx yz
        "###);
    }
}
