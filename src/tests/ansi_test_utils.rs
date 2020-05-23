#[cfg(test)]
pub mod ansi_test_utils {
    use ansi_term;
    use console::strip_ansi_codes;

    use crate::config::{color_from_rgb_or_ansi_code, Config};

    pub fn has_foreground_color(string: &str, color: ansi_term::Color) -> bool {
        let style = ansi_term::Style::default().fg(color);
        string.starts_with(&style.prefix().to_string())
    }

    pub fn assert_line_has_foreground_color(
        output: &str,
        line_number: usize,
        expected_prefix: &str,
        expected_color: &str,
        config: &Config,
    ) {
        let line = output.lines().nth(line_number).unwrap();
        assert!(strip_ansi_codes(line).starts_with(expected_prefix));
        assert!(has_foreground_color(
            line,
            color_from_rgb_or_ansi_code(expected_color, config.true_color)
        ));
    }

    pub fn assert_line_has_no_color(output: &str, line_number: usize, expected_prefix: &str) {
        let line = output.lines().nth(line_number).unwrap();
        let stripped_line = strip_ansi_codes(line);
        assert!(stripped_line.starts_with(expected_prefix));
        assert_eq!(line, stripped_line);
    }

    pub fn assert_has_color_other_than_plus_color(string: &str, config: &Config) {
        let (string_without_any_color, string_with_plus_color_only) =
            get_color_variants(string, config);
        assert_ne!(string, string_without_any_color);
        assert_ne!(string, string_with_plus_color_only);
    }

    pub fn assert_has_plus_color_only(string: &str, config: &Config) {
        let (string_without_any_color, string_with_plus_color_only) =
            get_color_variants(string, config);
        assert_ne!(string, string_without_any_color);
        assert_eq!(string, string_with_plus_color_only);
    }

    pub fn get_color_variants(string: &str, config: &Config) -> (String, String) {
        let string_without_any_color = strip_ansi_codes(string).to_string();
        let string_with_plus_color_only = config
            .plus_style
            .ansi_term_style
            .paint(&string_without_any_color);
        (
            string_without_any_color.to_string(),
            string_with_plus_color_only.to_string(),
        )
    }
}
