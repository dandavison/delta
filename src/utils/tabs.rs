use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ansi::ansi_strings_iterator;

pub fn has_tab(line: &str) -> bool {
    line.as_bytes().iter().any(|c| *c == b'\t')
}

#[derive(Debug, Clone)]
pub struct TabCfg {
    replacement: String,
}

impl TabCfg {
    pub fn new(width: usize) -> Self {
        TabCfg {
            replacement: " ".repeat(width),
        }
    }
    pub fn width(&self) -> usize {
        self.replacement.len()
    }
    pub fn replace(&self) -> bool {
        !self.replacement.is_empty()
    }
    fn replacement_str(&self, upto: usize) -> &str {
        &self.replacement[..upto]
    }
}

/// Expand tabs as spaces, always using a fixed number of replacement chars.
pub fn expand_fixed(line: &str, tabs: &TabCfg) -> String {
    if tabs.replace() && line.as_bytes().iter().any(|c| *c == b'\t') {
        itertools::join(line.split('\t'), &tabs.replacement)
    } else {
        line.to_string()
    }
}

/// Expand tabs as spaces, taking tabstops into account.
pub fn expand(line: &str, tabs: &TabCfg) -> String {
    expand_impl(line, tabs, expand_text)
}

/// Expand tabs as spaces, but don't count ansi escape codes as visible.
pub fn expand_raw(line: &str, tabs: &TabCfg) -> String {
    expand_impl(line, tabs, expand_ansi)
}

/// Remove `prefix` chars from `line`, then call `tabs::expand()`.
pub fn remove_prefix_and_expand(prefix: usize, line: &str, tabs: &TabCfg) -> String {
    let line_bytes = line.as_bytes();
    // The to-be-removed prefixes are almost always ascii +/- (or ++/ +/.. for merges) for
    // which grapheme clusters are not required.
    if line_bytes.len() >= prefix && line_bytes[..prefix].is_ascii() {
        // Safety: slicing into the utf-8 line-str is ok, upto `prefix` only ascii was present.
        expand(&line[prefix..], tabs)
    } else {
        let cut_line = line.graphemes(true).skip(prefix).collect::<String>();
        expand(&cut_line, tabs)
    }
}

#[inline]
fn expand_text(position: &mut usize, expanded: &mut String, line: &str, tabs: &TabCfg) {
    for c in line.graphemes(true) {
        if c == "\t" {
            let upto = tabs.width() - (*position % tabs.width());
            expanded.push_str(tabs.replacement_str(upto));
            *position = 0;
        } else {
            expanded.push_str(c);
            *position += c.width(); // see 54e1ee79c7cefe - some chars take up more than one cell
        }
    }
}

#[inline]
fn expand_ansi(position: &mut usize, expanded: &mut String, line: &str, tabs: &TabCfg) {
    for (element, is_ansi) in ansi_strings_iterator(line) {
        if is_ansi {
            // do not increment `position` counter
            expanded.push_str(element);
        } else {
            expand_text(position, expanded, element, tabs);
        }
    }
}

#[inline]
fn expand_impl<F>(line: &str, tabs: &TabCfg, tab_expander: F) -> String
where
    F: Fn(&mut usize, &mut String, &str, &TabCfg),
{
    if tabs.replace() && has_tab(line) {
        let mut expanded = String::new();
        let mut position = 0;
        tab_expander(&mut position, &mut expanded, line, tabs);
        expanded
    } else {
        line.to_string()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::ansi::strip_ansi_codes;
    use crate::tests::integration_test_utils::*;

    pub const TABSTOP_DIFF: &str = "\
--- a/a
+++ b/b
@@ -1 +1 @@
-1	1.	1..	1..4	1..4.	1..4..	1..4...	1..4...8	x
+1	1.	1..	1..4	1..4.	1..4..	1..4...	1..4...8	y
";

    #[test]
    fn test_remove_prefix_and_expand() {
        let line = "+-foo\tbar";
        let result = remove_prefix_and_expand(2, line, &TabCfg::new(3));
        assert_eq!(result, "foo   bar");
        let result = remove_prefix_and_expand(2, line, &TabCfg::new(0));
        assert_eq!(result, "foo\tbar");

        let utf8_prefix = "-│-foo\tbar";
        let n = 3;
        let result = remove_prefix_and_expand(n, utf8_prefix, &TabCfg::new(1));
        assert_eq!(result, "foo bar");
        // ensure non-ascii chars were removed:
        assert!(utf8_prefix.len() - result.len() > n);
    }

    #[test]
    fn test_tabstops() {
        let line = "1234\t1\t12\t123\tZ";
        let result = expand(line, &TabCfg::new(4));
        assert_eq!(result, "1234    1   12  123 Z");
    }

    #[test]
    fn test_expand_raw() {
        let raw_line = "\x1b[32m+\x1b[m\x1b[32mpub\tfn\tfoo() -> bool {\x1b[m";
        let expected = "+pub   fn     foo() -> bool {";
        let text_line = strip_ansi_codes(raw_line);
        let raw_result = expand_raw(raw_line, &TabCfg::new(7));
        let raw_result_noansi = strip_ansi_codes(&raw_result);
        let text_result = expand(&text_line, &TabCfg::new(7));
        let text_via_ansi = expand_raw(&text_line, &TabCfg::new(7));
        let raw_no_expansion = expand_raw(raw_line, &TabCfg::new(0));
        assert_eq!(expected, raw_result_noansi);
        assert_eq!(expected, text_result);
        assert_eq!(expected, text_via_ansi);
        assert_eq!(raw_line, raw_no_expansion);
    }

    #[test]
    fn test_tabs_expansion() {
        let config = make_config_from_args(&["--tabs", "8"]);
        let output = run_delta(TABSTOP_DIFF, &config);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(
            "1       1.      1..     1..4    1..4.   1..4..  1..4... 1..4...8        x",
            strip_ansi_codes(line_1)
        );
        assert_eq!(
            "1       1.      1..     1..4    1..4.   1..4..  1..4... 1..4...8        y",
            strip_ansi_codes(line_2)
        );

        // the +/- shifts everything, but tab counting remains identical
        let config = make_config_from_args(&["--tabs", "4", "--keep-plus-minus-markers"]);
        let output = run_delta(TABSTOP_DIFF, &config);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(
            "-1   1.  1.. 1..4    1..4.   1..4..  1..4... 1..4...8    x",
            strip_ansi_codes(line_1)
        );
        assert_eq!(
            "+1   1.  1.. 1..4    1..4.   1..4..  1..4... 1..4...8    y",
            strip_ansi_codes(line_2)
        );
    }
}
