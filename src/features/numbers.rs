use ansi_term;
use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::style::Style;

pub fn get_formatted_line_number_components<'a>(
    line_numbers: &'a Option<(Option<usize>, Option<usize>)>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let (minus, plus) = line_numbers.unwrap();

    let number_minus_style = get_zero_or_default_style(
        minus,
        plus,
        config.number_zero_style,
        config.number_minus_style,
    );

    let number_plus_style = get_zero_or_default_style(
        minus,
        plus,
        config.number_zero_style,
        config.number_plus_style,
    );

    let mut formatted_numbers = Vec::new();

    formatted_numbers.extend(format_number_components(
        minus,
        plus,
        &config.number_left_format,
        &config.number_left_format_style,
        &number_minus_style,
        &number_plus_style,
    ));
    formatted_numbers.extend(format_number_components(
        minus,
        plus,
        &config.number_right_format,
        &config.number_right_format_style,
        &number_minus_style,
        &number_plus_style,
    ));

    formatted_numbers
}

lazy_static! {
    static ref LINE_NUMBER_REGEXP: Regex = Regex::new(r"%(lm|lp)").unwrap();
}

fn format_number_components<'a>(
    minus: Option<usize>,
    plus: Option<usize>,
    format_string: &'a str,
    number_format_style: &Style,
    number_minus_style: &Style,
    number_plus_style: &Style,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut formatted_number_strings = Vec::new();

    let mut offset = 0;
    for _match in LINE_NUMBER_REGEXP.find_iter(&format_string) {
        formatted_number_strings
            .push(number_format_style.paint(&format_string[offset.._match.start()]));

        match _match.as_str() {
            "%lm" => {
                formatted_number_strings.push(number_minus_style.paint(format_line_number(minus)))
            }
            "%lp" => {
                formatted_number_strings.push(number_plus_style.paint(format_line_number(plus)))
            }
            _ => unreachable!(),
        }
        offset = _match.end();
    }
    formatted_number_strings.push(number_format_style.paint(&format_string[offset..]));
    formatted_number_strings
}

fn format_line_number(line_number: Option<usize>) -> String {
    format!(
        "{:^4}",
        line_number
            .map(|n| format!("{}", n))
            .as_deref()
            .unwrap_or_else(|| "")
    )
}

fn get_zero_or_default_style(
    minus: Option<usize>,
    plus: Option<usize>,
    zero_style: Option<Style>,
    default_style: Style,
) -> Style {
    match (zero_style, minus, plus) {
        (Some(z), Some(_), Some(_)) => z,
        _ => default_style,
    }
}

#[cfg(test)]
pub mod tests {
    use console::strip_ansi_codes;

    use crate::tests::integration_test_utils::integration_test_utils::{make_config, run_delta};

    #[test]
    fn test_two_minus_lines() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm⋮",
            "--number-right-format",
            "%lp│",
        ]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(4);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), " 1  ⋮    │a = 1");
        assert_eq!(strip_ansi_codes(line_2), " 2  ⋮    │b = 2");

        assert!(line_1.starts_with(
            &ansi_term::ANSIStrings(&[
                config.number_left_format_style.paint(" "),
                config.number_minus_style.paint("1  "),
                config.number_left_format_style.paint("⋮"),
                config.number_right_format_style.paint(" "),
                config.number_plus_style.paint("   "),
                config.number_right_format_style.paint("│"),
            ])
            .to_string()
        ));
        assert!(line_2.starts_with(
            &ansi_term::ANSIStrings(&[
                config.number_left_format_style.paint(" "),
                config.number_minus_style.paint("2  "),
                config.number_left_format_style.paint("⋮"),
                config.number_right_format_style.paint(" "),
                config.number_plus_style.paint("   "),
                config.number_right_format_style.paint("│"),
            ])
            .to_string()
        ));
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm⋮",
            "--number-right-format",
            "%lp│",
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
            "%lm⋮",
            "--number-right-format",
            "%lp│",
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
            "%lm %lm⋮",
            "--number-right-format",
            "%lp│",
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
