use std::cmp::max;

use ansi_term;
use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
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
    line_numbers_data: Option<&'a LineNumbersData>,
    line_numbers: &'a Option<(Option<usize>, Option<usize>)>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let line_numbers_data = line_numbers_data.unwrap();
    let (minus_number, plus_number) = line_numbers.unwrap();

    // If both minus and plus numbers are present then the line is a zero line.
    let (minus_number_style, plus_number_style) = match (minus_number, plus_number) {
        (Some(_), Some(_)) => (
            config.line_numbers_zero_style,
            config.line_numbers_zero_style,
        ),
        _ => (
            config.line_numbers_minus_style,
            config.line_numbers_plus_style,
        ),
    };

    let mut formatted_numbers = Vec::new();
    let min_line_number_width = 1
        + (line_numbers_data.hunk_max_line_number as f64)
            .log10()
            .floor() as usize;

    formatted_numbers.extend(format_and_paint_line_number_field(
        &line_numbers_data.left_format_data,
        &config.line_numbers_left_style,
        minus_number,
        plus_number,
        min_line_number_width,
        &minus_number_style,
        &plus_number_style,
    ));
    formatted_numbers.extend(format_and_paint_line_number_field(
        &line_numbers_data.right_format_data,
        &config.line_numbers_right_style,
        minus_number,
        plus_number,
        min_line_number_width,
        &minus_number_style,
        &plus_number_style,
    ));

    formatted_numbers
}

lazy_static! {
    static ref LINE_NUMBER_FORMAT_REGEX: Regex = Regex::new(
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
    pub hunk_max_line_number: usize,
}

// Although it's probably unusual, a single format string can contain multiple placeholders. E.g.
// line-numbers-right-format = "{nm} {np}|"
pub type LineNumberFormatData<'a> = Vec<LineNumberPlaceholderData<'a>>;

#[derive(Debug, Default, PartialEq)]
pub struct LineNumberPlaceholderData<'a> {
    pub prefix: &'a str,
    pub placeholder: &'a str,
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
            hunk_max_line_number: 0,
        }
    }
}

fn parse_line_number_format<'a>(format_string: &'a str) -> LineNumberFormatData<'a> {
    let mut format_data = Vec::new();
    let mut offset = 0;

    for captures in LINE_NUMBER_FORMAT_REGEX.captures_iter(format_string) {
        let _match = captures.get(0).unwrap();
        format_data.push(LineNumberPlaceholderData {
            prefix: &format_string[offset.._match.start()],
            placeholder: captures.get(1).map(|m| m.as_str()).unwrap(),
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
    format_data
}

fn format_and_paint_line_number_field<'a>(
    format_data: &Vec<LineNumberPlaceholderData<'a>>,
    style: &Style,
    minus_number: Option<usize>,
    plus_number: Option<usize>,
    min_line_number_width: usize,
    minus_number_style: &Style,
    plus_number_style: &Style,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut ansi_strings = Vec::new();
    let mut suffix = "";
    for placeholder in format_data {
        ansi_strings.push(style.paint(placeholder.prefix));

        let alignment_spec = placeholder.alignment_spec.unwrap_or("^");
        let width = if let Some(placeholder_width) = placeholder.width {
            max(placeholder_width, min_line_number_width)
        } else {
            min_line_number_width
        };

        match placeholder.placeholder {
            "nm" => ansi_strings.push(minus_number_style.paint(format_line_number(
                minus_number,
                alignment_spec,
                width,
            ))),
            "np" => ansi_strings.push(plus_number_style.paint(format_line_number(
                plus_number,
                alignment_spec,
                width,
            ))),
            _ => unreachable!(),
        }
        suffix = placeholder.suffix;
    }
    ansi_strings.push(style.paint(suffix));
    ansi_strings
}

/// Return line number formatted according to `alignment` and `width`.
fn format_line_number(line_number: Option<usize>, alignment: &str, width: usize) -> String {
    let n = line_number
        .map(|n| format!("{}", n))
        .unwrap_or_else(|| "".to_string());
    match alignment {
        "<" => format!("{0:<1$}", n, width),
        "^" => format!("{0:^1$}", n, width),
        ">" => format!("{0:>1$}", n, width),
        _ => unreachable!(),
    }
}

#[cfg(test)]
pub mod tests {
    use console::strip_ansi_codes;
    use regex::Captures;

    use crate::tests::integration_test_utils::integration_test_utils::{make_config, run_delta};

    use super::*;

    #[test]
    fn test_line_number_format_regex_1() {
        assert_eq!(
            parse_line_number_format("{nm}"),
            vec![LineNumberPlaceholderData {
                prefix: "",
                placeholder: "nm",
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
                placeholder: "np",
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
                placeholder: "np",
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
                placeholder: "np",
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
                placeholder: "np",
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
                    placeholder: "nm",
                    alignment_spec: Some("<"),
                    width: Some(3),
                    suffix: "@@---{np:_>4}**",
                },
                LineNumberPlaceholderData {
                    prefix: "@@---",
                    placeholder: "np",
                    alignment_spec: Some(">"),
                    width: Some(4),
                    suffix: "**",
                }
            ]
        )
    }

    fn _get_capture<'a>(i: usize, j: usize, caps: &'a Vec<Captures>) -> &'a str {
        caps[i].get(j).map_or("", |m| m.as_str())
    }

    #[test]
    fn test_two_minus_lines() {
        let config = make_config(&[
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
        println!("{}", &output);
        let mut lines = output.lines().skip(4);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), " 1  ⋮    │a = 1");
        assert_eq!(strip_ansi_codes(line_2), " 2  ⋮    │b = 2");
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config(&[
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
        let mut lines = output.lines().skip(4);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), "    ⋮ 1  │a = 1");
        assert_eq!(strip_ansi_codes(line_2), "    ⋮ 2  │b = 2");
    }

    #[test]
    fn test_one_minus_one_plus_line() {
        let config = make_config(&[
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
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮ 2  │bb = 2");
    }

    #[test]
    fn test_repeated_placeholder() {
        let config = make_config(&[
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
        println!("{}", output);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1    1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2    2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "         ⋮ 2  │bb = 2");
    }

    #[test]
    fn test_five_digit_line_number() {
        let config = make_config(&["--line-numbers"]);
        let output = run_delta(FIVE_DIGIT_LINE_NUMBER_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), "10000⋮10000│a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10001│bb = 2");
    }

    #[test]
    fn test_unequal_digit_line_number() {
        let config = make_config(&["--line-numbers"]);
        let output = run_delta(UNEQUAL_DIGIT_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), "10000⋮9999 │a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10000│bb = 2");
    }

    const TWO_MINUS_LINES_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +0,0 @@
-a = 1
-b = 2
";

    const TWO_PLUS_LINES_DIFF: &str = "\
diff --git c/a.py i/a.py
new file mode 100644
index 0000000..223ca50
--- /dev/null
+++ i/a.py
@@ -0,0 +1,2 @@
+a = 1
+b = 2
";

    const ONE_MINUS_ONE_PLUS_LINE_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
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
