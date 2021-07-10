use std::cmp::max;

use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::delta::State;
use crate::features::hyperlinks;
use crate::features::side_by_side;
use crate::features::OptionValueFunction;
use crate::style::Style;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "line-numbers",
            bool,
            None,
            _opt => true
        ),
        (
            "line-numbers-left-style",
            String,
            None,
            _opt => "blue"
        ),
        (
            "line-numbers-right-style",
            String,
            None,
            _opt => "blue"
        ),
        (
            "line-numbers-minus-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {
                "red".to_string()
            } else {
                "88".to_string()
            }
        ),
        (
            "line-numbers-zero-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {"#dddddd"} else {"#444444"}
        ),
        (
            "line-numbers-plus-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {
                "green".to_string()
            } else {
                "28".to_string()
            }
        )
    ])
}

/// Return a vec of `ansi_term::ANSIGenericString`s representing the left and right fields of the
/// two-column line number display.
pub fn format_and_paint_line_numbers<'a>(
    line_numbers_data: &'a mut LineNumbersData,
    state: &State,
    side_by_side_panel: Option<side_by_side::PanelSide>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let m_ref = &mut line_numbers_data.hunk_minus_line_number;
    let p_ref = &mut line_numbers_data.hunk_plus_line_number;
    let (minus_style, zero_style, plus_style) = (
        config.line_numbers_minus_style,
        config.line_numbers_zero_style,
        config.line_numbers_plus_style,
    );
    let ((minus_number, plus_number), (minus_style, plus_style)) = match state {
        State::HunkMinus(_) => {
            let m = *m_ref;
            *m_ref += 1;
            ((Some(m), None), (minus_style, plus_style))
        }
        State::HunkZero => {
            let (m, p) = (*m_ref, *p_ref);
            *m_ref += 1;
            *p_ref += 1;
            ((Some(m), Some(p)), (zero_style, zero_style))
        }
        State::HunkPlus(_) => {
            let p = *p_ref;
            *p_ref += 1;
            ((None, Some(p)), (minus_style, plus_style))
        }
        _ => return Vec::new(),
    };

    let mut formatted_numbers = Vec::new();

    let (emit_left, emit_right) = match (config.side_by_side, side_by_side_panel) {
        (false, _) => (true, true),
        (true, Some(side_by_side::PanelSide::Left)) => (true, false),
        (true, Some(side_by_side::PanelSide::Right)) => (false, true),
        (true, None) => unreachable!(),
    };

    if emit_left {
        formatted_numbers.extend(format_and_paint_line_number_field(
            &line_numbers_data.left_format_data,
            &config.line_numbers_left_style,
            minus_number,
            plus_number,
            line_numbers_data.hunk_max_line_number_width,
            &minus_style,
            &plus_style,
            &line_numbers_data.plus_file,
            config,
        ));
    }

    if emit_right {
        formatted_numbers.extend(format_and_paint_line_number_field(
            &line_numbers_data.right_format_data,
            &config.line_numbers_right_style,
            minus_number,
            plus_number,
            line_numbers_data.hunk_max_line_number_width,
            &minus_style,
            &plus_style,
            &line_numbers_data.plus_file,
            config,
        ));
    }
    formatted_numbers
}

lazy_static! {
    static ref LINE_NUMBERS_PLACEHOLDER_REGEX: Regex = Regex::new(
        r"(?x)
\{
(nm|np)         # 1: Literal nm or np
(?:             # Start optional format spec (non-capturing)
  :             #     Literal colon
  (?:           #     Start optional fill/alignment spec (non-capturing)
    ([^<^>])?   #         2: Optional fill character (ignored)
    ([<^>])     #         3: Alignment spec
  )?            #
  (\d+)         #     4: Width
)?              #
\}
"
    )
    .unwrap();
}

#[derive(Default)]
pub struct LineNumbersData<'a> {
    pub left_format_data: LineNumberFormatData<'a>,
    pub right_format_data: LineNumberFormatData<'a>,
    pub hunk_minus_line_number: usize,
    pub hunk_plus_line_number: usize,
    pub hunk_max_line_number_width: usize,
    pub plus_file: String,
}

// Although it's probably unusual, a single format string can contain multiple placeholders. E.g.
// line-numbers-right-format = "{nm} {np}|"
pub type LineNumberFormatData<'a> = Vec<LineNumberPlaceholderData<'a>>;

#[derive(Debug, Default, PartialEq)]
pub struct LineNumberPlaceholderData<'a> {
    pub prefix: &'a str,
    pub placeholder: Option<&'a str>,
    pub alignment_spec: Option<&'a str>,
    pub width: Option<usize>,
    pub suffix: &'a str,
}

impl<'a> LineNumbersData<'a> {
    pub fn from_format_strings(left_format: &'a str, right_format: &'a str) -> LineNumbersData<'a> {
        Self {
            left_format_data: parse_line_number_format(left_format),
            right_format_data: parse_line_number_format(right_format),
            hunk_minus_line_number: 0,
            hunk_plus_line_number: 0,
            hunk_max_line_number_width: 0,
            plus_file: "".to_string(),
        }
    }

    /// Initialize line number data for a hunk.
    pub fn initialize_hunk(&mut self, line_numbers: &[(usize, usize)], plus_file: String) {
        // Typically, line_numbers has length 2: an entry for the minus file, and one for the plus
        // file. In the case of merge commits, it may be longer.
        self.hunk_minus_line_number = line_numbers[0].0;
        self.hunk_plus_line_number = line_numbers[line_numbers.len() - 1].0;
        let hunk_max_line_number = line_numbers.iter().map(|(n, d)| n + d).max().unwrap();
        self.hunk_max_line_number_width =
            1 + (hunk_max_line_number as f64).log10().floor() as usize;
        self.plus_file = plus_file;
    }
}

fn parse_line_number_format(format_string: &str) -> LineNumberFormatData {
    let mut format_data = Vec::new();
    let mut offset = 0;

    for captures in LINE_NUMBERS_PLACEHOLDER_REGEX.captures_iter(format_string) {
        let _match = captures.get(0).unwrap();
        format_data.push(LineNumberPlaceholderData {
            prefix: &format_string[offset.._match.start()],
            placeholder: captures.get(1).map(|m| m.as_str()),
            alignment_spec: captures.get(3).map(|m| m.as_str()),
            width: captures.get(4).map(|m| {
                m.as_str()
                    .parse()
                    .unwrap_or_else(|_| panic!("Invalid width in format string: {}", format_string))
            }),
            suffix: &format_string[_match.end()..],
        });
        offset = _match.end();
    }
    if offset == 0 {
        // No placeholders
        format_data.push(LineNumberPlaceholderData {
            prefix: &format_string[..0],
            placeholder: None,
            alignment_spec: None,
            width: None,
            suffix: &format_string[0..],
        })
    }
    format_data
}

#[allow(clippy::too_many_arguments)]
fn format_and_paint_line_number_field<'a>(
    format_data: &[LineNumberPlaceholderData<'a>],
    style: &Style,
    minus_number: Option<usize>,
    plus_number: Option<usize>,
    min_field_width: usize,
    minus_number_style: &Style,
    plus_number_style: &Style,
    plus_file: &str,
    config: &config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut ansi_strings = Vec::new();
    let mut suffix = "";
    for placeholder in format_data {
        ansi_strings.push(style.paint(placeholder.prefix));

        let alignment_spec = placeholder.alignment_spec.unwrap_or("^");
        let width = if let Some(placeholder_width) = placeholder.width {
            max(placeholder_width, min_field_width)
        } else {
            min_field_width
        };

        match placeholder.placeholder {
            Some("nm") => ansi_strings.push(minus_number_style.paint(format_line_number(
                minus_number,
                alignment_spec,
                width,
                None,
                config,
            ))),
            Some("np") => ansi_strings.push(plus_number_style.paint(format_line_number(
                plus_number,
                alignment_spec,
                width,
                Some(plus_file),
                config,
            ))),
            None => {}
            Some(_) => unreachable!(),
        }
        suffix = placeholder.suffix;
    }
    ansi_strings.push(style.paint(suffix));
    ansi_strings
}

/// Return line number formatted according to `alignment` and `width`.
fn format_line_number(
    line_number: Option<usize>,
    alignment: &str,
    width: usize,
    plus_file: Option<&str>,
    config: &config::Config,
) -> String {
    let format_n = |n| match alignment {
        "<" => format!("{0:<1$}", n, width),
        "^" => format!("{0:^1$}", n, width),
        ">" => format!("{0:>1$}", n, width),
        _ => unreachable!(),
    };
    match (line_number, config.hyperlinks, plus_file) {
        (None, _, _) => format_n(""),
        (Some(n), true, Some(file)) => hyperlinks::format_osc8_file_hyperlink(
            file,
            line_number,
            &format_n(&n.to_string()),
            config,
        )
        .to_string(),
        (Some(n), _, _) => format_n(&n.to_string()),
    }
}

#[cfg(test)]
pub mod tests {
    use regex::Captures;

    use crate::ansi::strip_ansi_codes;
    use crate::tests::integration_test_utils::{make_config_from_args, run_delta};

    use super::*;

    #[test]
    fn test_line_number_format_regex_1() {
        assert_eq!(
            parse_line_number_format("{nm}"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: Some("nm"),
                alignment_spec: None,
                width: None,
                suffix: "",
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_2() {
        assert_eq!(
            parse_line_number_format("{np:4}"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: Some("np"),
                alignment_spec: None,
                width: Some(4),
                suffix: "",
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_3() {
        assert_eq!(
            parse_line_number_format("{np:>4}"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: Some("np"),
                alignment_spec: Some(">"),
                width: Some(4),
                suffix: "",
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_4() {
        assert_eq!(
            parse_line_number_format("{np:_>4}"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: Some("np"),
                alignment_spec: Some(">"),
                width: Some(4),
                suffix: "",
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_5() {
        assert_eq!(
            parse_line_number_format("__{np:_>4}@@"),
            vec![LineNumberPlaceholderData {
                prefix: "__",
                placeholder: Some("np"),
                alignment_spec: Some(">"),
                width: Some(4),
                suffix: "@@",
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_6() {
        assert_eq!(
            parse_line_number_format("__{nm:<3}@@---{np:_>4}**"),
            vec![
                LineNumberPlaceholderData {
                    prefix: "__",
                    placeholder: Some("nm"),
                    alignment_spec: Some("<"),
                    width: Some(3),
                    suffix: "@@---{np:_>4}**",
                },
                LineNumberPlaceholderData {
                    prefix: "@@---",
                    placeholder: Some("np"),
                    alignment_spec: Some(">"),
                    width: Some(4),
                    suffix: "**",
                }
            ]
        )
    }

    #[test]
    fn test_line_number_format_regex_7() {
        assert_eq!(
            parse_line_number_format("__@@---**"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: None,
                alignment_spec: None,
                width: None,
                suffix: "__@@---**",
            },]
        )
    }

    fn _get_capture<'a>(i: usize, j: usize, caps: &'a Vec<Captures>) -> &'a str {
        caps[i].get(j).map_or("", |m| m.as_str())
    }

    #[test]
    fn test_two_minus_lines() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), " 1  ⋮    │a = 1");
        assert_eq!(strip_ansi_codes(line_2), " 2  ⋮    │b = 2");
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), "    ⋮ 1  │a = 1");
        assert_eq!(strip_ansi_codes(line_2), "    ⋮ 2  │b = 2");
    }

    #[test]
    fn test_one_minus_one_plus_line() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(7);
        assert_eq!(lines.next().unwrap(), " 1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮ 2  │bb = 2");
    }

    #[test]
    fn test_repeated_placeholder() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4} {nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(7);
        assert_eq!(lines.next().unwrap(), " 1    1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2    2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "         ⋮ 2  │bb = 2");
    }

    #[test]
    fn test_five_digit_line_number() {
        let config = make_config_from_args(&["--line-numbers"]);
        let output = run_delta(FIVE_DIGIT_LINE_NUMBER_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(7);
        assert_eq!(lines.next().unwrap(), "10000⋮10000│a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10001│bb = 2");
    }

    #[test]
    fn test_unequal_digit_line_number() {
        let config = make_config_from_args(&["--line-numbers"]);
        let output = run_delta(UNEQUAL_DIGIT_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(7);
        assert_eq!(lines.next().unwrap(), "10000⋮9999 │a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10000│bb = 2");
    }

    #[test]
    fn test_color_only() {
        let config = make_config_from_args(&["--line-numbers", "--color-only"]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(5);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), " 1  ⋮    │-a = 1");
        assert_eq!(strip_ansi_codes(line_2), " 2  ⋮    │-b = 2");
    }

    #[test]
    fn test_hunk_header_style_is_omit() {
        let config = make_config_from_args(&["--line-numbers", "--hunk-header-style", "omit"]);
        let output = run_delta(TWO_LINE_DIFFS, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮ 2  │bb = 2");
        assert_eq!(lines.next().unwrap(), "");
        assert_eq!(lines.next().unwrap(), "499 ⋮499 │a = 3");
        assert_eq!(lines.next().unwrap(), "500 ⋮    │b = 4");
        assert_eq!(lines.next().unwrap(), "    ⋮500 │bb = 4");
    }

    pub const TWO_MINUS_LINES_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +0,0 @@
-a = 1
-b = 2
";

    pub const TWO_PLUS_LINES_DIFF: &str = "\
diff --git c/a.py i/a.py
new file mode 100644
index 0000000..223ca50
--- /dev/null
+++ i/a.py
@@ -0,0 +1,2 @@
+a = 1
+b = 2
";

    pub const ONE_MINUS_ONE_PLUS_LINE_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
";

    const TWO_LINE_DIFFS: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
@@ -499,2 +499,2 @@
 a = 3
-b = 4
+bb = 4
";

    const FIVE_DIGIT_LINE_NUMBER_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -10000,2 +10000,2 @@
 a = 1
-b = 2
+bb = 2
";

    const UNEQUAL_DIGIT_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -10000,2 +9999,2 @@
 a = 1
-b = 2
+bb = 2
";
}
