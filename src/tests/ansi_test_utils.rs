#[cfg(test)]
pub mod ansi_test_utils {
    use ansi_parser::{self, AnsiParser};
    use console::strip_ansi_codes;
    use itertools::Itertools;
    use syntect::highlighting::StyleModifier;

    use crate::bat::assets::HighlightingAssets;
    use crate::cli;
    use crate::config::{ColorLayer::*, Config};
    use crate::delta::State;
    use crate::paint;

    pub fn is_syntax_highlighted(line: &str) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                ansi_parser::Output::TextBlock(s) => string_has_some_foreground_color(s),
                _ => false,
            })
            .next()
            .is_some()
    }

    pub fn line_has_background_color(line: &str, state: &State, config: &Config) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                ansi_parser::Output::TextBlock(s) => string_has_background_color(s, state, config),
                _ => false,
            })
            .next()
            .is_some()
    }

    pub fn line_has_no_background_color(line: &str) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                ansi_parser::Output::TextBlock(s) => string_has_some_background_color(s),
                _ => false,
            })
            .next()
            .is_none()
    }

    fn string_has_background_color(string: &str, state: &State, config: &Config) -> bool {
        let painted = paint::paint_text_background(
            "",
            config
                .get_color(state, Background)
                .unwrap_or_else(|| panic!("state {:?} does not have a background color", state)),
            true,
        );
        let ansi_sequence = painted.trim_end_matches(paint::ANSI_SGR_RESET);
        string.starts_with(ansi_sequence)
    }

    fn string_has_some_background_color(s: &str) -> bool {
        s.starts_with("\x1b[48;")
    }

    fn string_has_some_foreground_color(s: &str) -> bool {
        s.starts_with("\x1b[38;")
    }

    pub fn assert_line_has_expected_ansi_sequences(line: &str, expected: &Vec<(&str, &str)>) {
        assert_eq!(line.ansi_parse().count(), expected.len());
        for ((expected_ansi_sequence, _), ref token) in expected.iter().zip_eq(line.ansi_parse()) {
            match token {
                ansi_parser::Output::TextBlock(s) => {
                    assert!(s.starts_with(*expected_ansi_sequence));
                }
                ansi_parser::Output::Escape(_) => {
                    assert_eq!(expected_ansi_sequence, &paint::ANSI_SGR_RESET);
                }
            }
        }
    }

    pub fn assert_has_color_other_than_plus_color(string: &str, options: &cli::Opt) {
        let (string_without_any_color, string_with_plus_color_only) =
            get_color_variants(string, &options);
        assert_ne!(string, string_without_any_color);
        assert_ne!(string, string_with_plus_color_only);
    }

    pub fn assert_has_plus_color_only(string: &str, options: &cli::Opt) {
        let (string_without_any_color, string_with_plus_color_only) =
            get_color_variants(string, &options);
        assert_ne!(string, string_without_any_color);
        assert_eq!(string, string_with_plus_color_only);
    }

    pub fn get_color_variants(string: &str, options: &cli::Opt) -> (String, String) {
        let assets = HighlightingAssets::new();
        let config = cli::process_command_line_arguments(&assets, &options);

        let string_without_any_color = strip_ansi_codes(string).to_string();
        let string_with_plus_color_only = paint_text(
            &string_without_any_color,
            config.plus_style_modifier,
            &config,
        );
        (string_without_any_color, string_with_plus_color_only)
    }

    fn paint_text(input: &str, style_modifier: StyleModifier, config: &Config) -> String {
        let mut output = String::new();
        let style = config.no_style.apply(style_modifier);
        paint::paint_text(&input, style, &mut output, config.true_color);
        output
    }
}
