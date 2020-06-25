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
            "line-numbers-minus-style",
            String,
            Some("color.diff.old"),
            _opt => "red"
        ),
        (
            "line-numbers-zero-style",
            String,
            None,
            _opt => "#bbbbbb"
        ),
        (
            "line-numbers-plus-style",
            String,
            Some("color.diff.new"),
            _opt => "green"
        )
    ])
}

/// Return a vec of `ansi_term::ANSIGenericString`s representing the left and right fields of the
/// two-column line number display.
pub fn format_and_paint_line_numbers<'a>(
    line_numbers: &'a Option<(Option<usize>, Option<usize>)>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
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

    formatted_numbers.extend(format_and_paint_line_number_field(
        &config.line_numbers_left_format,
        &config.line_numbers_left_style,
        minus_number,
        plus_number,
        &minus_number_style,
        &plus_number_style,
    ));
    formatted_numbers.extend(format_and_paint_line_number_field(
        &config.line_numbers_right_format,
        &config.line_numbers_right_style,
        minus_number,
        plus_number,
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
    ([^<^>])?   #         2: Optional fill character
    ([<^>])     #         3: Alignment spec
  )?            #
  (\d+)         #     4: Width
)?              #
\}
"
    )
    .unwrap();
}

fn format_and_paint_line_number_field<'a>(
    format_string: &'a str,
    style: &Style,
    minus_number: Option<usize>,
    plus_number: Option<usize>,
    minus_number_style: &Style,
    plus_number_style: &Style,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut ansi_strings = Vec::new();

    let mut offset = 0;
    for caps in LINE_NUMBER_FORMAT_REGEX.captures_iter(&format_string) {
        let _match = caps.get(0).unwrap();
        ansi_strings.push(style.paint(&format_string[offset.._match.start()]));

        match &caps[1] {
            "nm" => ansi_strings.push(minus_number_style.paint(format_line_number(
                minus_number,
                &caps[3],
                &caps[4],
            ))),
            "np" => ansi_strings.push(plus_number_style.paint(format_line_number(
                plus_number,
                &caps[3],
                &caps[4],
            ))),
            _ => unreachable!(),
        }
        offset = _match.end();
    }
    ansi_strings.push(style.paint(&format_string[offset..]));
    ansi_strings
}

/// Return line number formatted according to `alignment` and `width`.
fn format_line_number(line_number: Option<usize>, alignment: &str, width: &str) -> String {
    let n = line_number
        .map(|n| format!("{}", n))
        .unwrap_or_else(|| "".to_string());
    let default_width = 4; // Used only if \d+ cannot be parsed as usize
    let w: usize = width.parse().unwrap_or(default_width);
    match alignment {
        "<" => format!("{0:<1$}", n, w),
        "^" | "" => format!("{0:^1$}", n, w),
        ">" => format!("{0:>1$}", n, w),
        _ => unreachable!(),
    }
}

#[cfg(test)]
pub mod tests {
    use console::strip_ansi_codes;
    use regex::Captures;

    use crate::tests::integration_test_utils::integration_test_utils::{make_config, run_delta};

    use super::LINE_NUMBER_FORMAT_REGEX;

    #[test]
    fn test_line_number_format_regex_1() {
        let caps = LINE_NUMBER_FORMAT_REGEX
            .captures_iter("{nm}")
            .collect::<Vec<Captures>>();
        assert_eq!(caps.len(), 1);
        assert_eq!(_get_capture(0, 1, &caps), "nm");
        assert_eq!(_get_capture(0, 2, &caps), "");
        assert_eq!(_get_capture(0, 3, &caps), "");
        assert_eq!(_get_capture(0, 4, &caps), "");
    }

    #[test]
    fn test_line_number_format_regex_2() {
        let caps = LINE_NUMBER_FORMAT_REGEX
            .captures_iter("{np:4}")
            .collect::<Vec<Captures>>();
        assert_eq!(caps.len(), 1);
        assert_eq!(_get_capture(0, 1, &caps), "np");
        assert_eq!(_get_capture(0, 2, &caps), "");
        assert_eq!(_get_capture(0, 3, &caps), "");
        assert_eq!(_get_capture(0, 4, &caps), "4");
    }

    #[test]
    fn test_line_number_format_regex_3() {
        let caps = LINE_NUMBER_FORMAT_REGEX
            .captures_iter("{np:>4}")
            .collect::<Vec<Captures>>();
        assert_eq!(caps.len(), 1);
        assert_eq!(_get_capture(0, 1, &caps), "np");
        assert_eq!(_get_capture(0, 2, &caps), "");
        assert_eq!(_get_capture(0, 3, &caps), ">");
        assert_eq!(_get_capture(0, 4, &caps), "4");
    }

    #[test]
    fn test_line_number_format_regex_4() {
        let caps = LINE_NUMBER_FORMAT_REGEX
            .captures_iter("{np:_>4}")
            .collect::<Vec<Captures>>();
        assert_eq!(caps.len(), 1);
        assert_eq!(_get_capture(0, 1, &caps), "np");
        assert_eq!(_get_capture(0, 2, &caps), "_");
        assert_eq!(_get_capture(0, 3, &caps), ">");
        assert_eq!(_get_capture(0, 4, &caps), "4");
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
}
