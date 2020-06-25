use ansi_term;
use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::style::Style;

/// Return a vec of `ansi_term::ANSIGenericString`s representing the left and right fields of the
/// two-column line number display.
pub fn format_and_paint_line_numbers<'a>(
    line_numbers: &'a Option<(Option<usize>, Option<usize>)>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let (minus_number, plus_number) = line_numbers.unwrap();

    // If both minus and plus numbers are present then the line is a zero line.
    let (number_minus_style, number_plus_style) =
        match (minus_number, plus_number, config.number_zero_style) {
            (Some(_), Some(_), Some(zero_style)) => (zero_style, zero_style),
            _ => (config.number_minus_style, config.number_plus_style),
        };

    let mut formatted_numbers = Vec::new();

    formatted_numbers.extend(format_and_paint_line_number_field(
        &config.number_left_format,
        &config.number_left_format_style,
        minus_number,
        plus_number,
        &number_minus_style,
        &number_plus_style,
    ));
    formatted_numbers.extend(format_and_paint_line_number_field(
        &config.number_right_format,
        &config.number_right_format_style,
        minus_number,
        plus_number,
        &number_minus_style,
        &number_plus_style,
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
    number_format_style: &Style,
    minus: Option<usize>,
    plus: Option<usize>,
    number_minus_style: &Style,
    number_plus_style: &Style,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut formatted_number_strings = Vec::new();

    let mut offset = 0;
    for caps in LINE_NUMBER_FORMAT_REGEX.captures_iter(&format_string) {
        let _match = caps.get(0).unwrap();
        formatted_number_strings
            .push(number_format_style.paint(&format_string[offset.._match.start()]));

        match &caps[1] {
            "nm" => formatted_number_strings
                .push(number_minus_style.paint(format_line_number(minus, &caps[3], &caps[4]))),
            "np" => formatted_number_strings
                .push(number_plus_style.paint(format_line_number(plus, &caps[3], &caps[4]))),
            _ => unreachable!(),
        }
        offset = _match.end();
    }
    formatted_number_strings.push(number_format_style.paint(&format_string[offset..]));
    formatted_number_strings
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
            "--number",
            "--number-left-format",
            "{nm:^4}⋮",
            "--number-right-format",
            "{np:^4}│",
            "--number-left-format-style",
            "0 1",
            "--number-minus-style",
            "0 2",
            "--number-right-format-style",
            "0 3",
            "--number-plus-style",
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
            "--number",
            "--number-left-format",
            "{nm:^4}⋮",
            "--number-right-format",
            "{np:^4}│",
            "--number-left-format-style",
            "0 1",
            "--number-minus-style",
            "0 2",
            "--number-right-format-style",
            "0 3",
            "--number-plus-style",
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
            "--number",
            "--number-left-format",
            "{nm:^4}⋮",
            "--number-right-format",
            "{np:^4}│",
            "--number-left-format-style",
            "0 1",
            "--number-minus-style",
            "0 2",
            "--number-right-format-style",
            "0 3",
            "--number-plus-style",
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
            "--number",
            "--number-left-format",
            "{nm:^4} {nm:^4}⋮",
            "--number-right-format",
            "{np:^4}│",
            "--number-left-format-style",
            "0 1",
            "--number-minus-style",
            "0 2",
            "--number-right-format-style",
            "0 3",
            "--number-plus-style",
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
