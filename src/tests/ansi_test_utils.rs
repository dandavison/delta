#[cfg(test)]
pub mod ansi_test_utils {
    use ansi_term;
    use console::strip_ansi_codes;

    use crate::config::{color_from_rgb_or_ansi_code, Config};
    use crate::paint;
    use crate::style::Style;

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

    /// Assert that the specified line number of output (a) matches
    /// `expected_prefix` and (b) for the length of expected_prefix is
    /// syntax-highlighted according to `language_extension`.
    pub fn assert_line_is_syntax_highlighted(
        output: &str,
        line_number: usize,
        expected_prefix: &str,
        language_extension: &str,
        config: &Config,
    ) {
        let line = output.lines().nth(line_number).unwrap();
        let stripped_line = &strip_ansi_codes(line);
        assert!(stripped_line.starts_with(expected_prefix));
        let painted_line = paint_line(expected_prefix, language_extension, config);
        // remove trailing newline appended by paint::paint_lines.
        assert!(line.starts_with(painted_line.trim_end()));
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

    pub fn paint_line(line: &str, language_extension: &str, config: &Config) -> String {
        let mut output_buffer = String::new();
        let mut unused_writer = Vec::<u8>::new();
        let mut painter = paint::Painter::new(&mut unused_writer, config);
        let syntax_highlighted_style = Style {
            is_syntax_highlighted: true,
            ..Style::new()
        };
        painter.set_syntax(Some(language_extension));
        painter.set_highlighter();
        let lines = vec![line];
        let syntax_style_sections = painter.highlighter.highlight(line, &config.syntax_set);
        paint::Painter::paint_lines(
            vec![syntax_style_sections],
            vec![vec![(syntax_highlighted_style, lines[0])]],
            &mut output_buffer,
            config,
            "",
            config.null_style,
            config.null_style,
            None,
        );
        output_buffer
    }
}
