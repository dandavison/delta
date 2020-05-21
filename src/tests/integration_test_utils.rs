#[cfg(test)]
pub mod integration_test_utils {
    use bytelines::ByteLines;
    use console::strip_ansi_codes;
    use std::io::BufReader;

    use crate::cli;
    use crate::config;
    use crate::delta::delta;

    // TODO: These should be set in a more principled way, based on the default
    // argument values.
    pub fn get_command_line_options() -> cli::Opt {
        cli::Opt {
            light: false,
            dark: false,
            minus_style: "normal auto".to_string(),
            minus_emph_style: "normal auto".to_string(),
            zero_style: "syntax normal".to_string(),
            plus_style: "syntax auto".to_string(),
            plus_emph_style: "syntax auto".to_string(),
            deprecated_minus_background_color: None,
            deprecated_minus_emph_background_color: None,
            deprecated_plus_background_color: None,
            deprecated_plus_emph_background_color: None,
            color_only: false,
            keep_plus_minus_markers: false,
            theme: None,
            deprecated_highlight_minus_lines: false,
            commit_style: cli::SectionStyle::Plain,
            commit_color: "Yellow".to_string(),
            file_style: cli::SectionStyle::Underline,
            file_color: "Blue".to_string(),
            hunk_style: cli::SectionStyle::Box,
            hunk_color: "blue".to_string(),
            true_color: "always".to_string(),
            width: Some("variable".to_string()),
            paging_mode: "auto".to_string(),
            tab_width: 4,
            show_background_colors: false,
            list_languages: false,
            list_theme_names: false,
            list_themes: false,
            max_line_distance: 0.3,
        }
    }

    pub fn get_line_of_code_from_delta<'a>(
        input: &str,
        options: cli::Opt,
    ) -> (String, config::Config<'a>) {
        let (output, config) = run_delta(&input, options);
        let line_of_code = output.lines().nth(12).unwrap();
        assert!(strip_ansi_codes(line_of_code) == " class X:");
        (line_of_code.to_string(), config)
    }

    pub fn run_delta<'a>(input: &str, options: cli::Opt) -> (String, config::Config<'a>) {
        let mut writer: Vec<u8> = Vec::new();

        let config = cli::process_command_line_arguments(options);

        delta(
            ByteLines::new(BufReader::new(input.as_bytes())),
            &mut writer,
            &config,
        )
        .unwrap();
        (String::from_utf8(writer).unwrap(), config)
    }
}
