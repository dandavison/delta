#[cfg(test)]
pub mod ansi_test_utils {
    use ansi_parser::{self, AnsiParser};
    use console::strip_ansi_codes;
    use itertools::Itertools;

    use crate::config::{ColorLayer::*, Config};
    use crate::delta::State;
    use crate::paint;

    use ansi_parser::AnsiSequence::*;
    use ansi_parser::Output::*;

    // Note that ansi_parser seems to be parsing 24-bit sequences as TextBlock
    // rather than Escape(SetGraphicsMode). As a workaround, we examime the
    // TextBlock string value using functions such as
    // string_has_some_background_color and string_has_some_foreground_color.

    pub fn is_syntax_highlighted(line: &str) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                TextBlock(s) => string_has_some_foreground_color(s),
                Escape(SetGraphicsMode(parameters)) => parameters[0] == 38,
                _ => false,
            })
            .next()
            .is_some()
    }

    pub fn line_has_background_color(line: &str, state: &State, config: &Config) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                TextBlock(s) => string_has_background_color(s, state, config),
                _ => false,
            })
            .next()
            .is_some()
    }

    pub fn line_has_no_background_color(line: &str) -> bool {
        line.ansi_parse()
            .filter(|tok| match tok {
                TextBlock(s) => string_has_some_background_color(s),
                _ => false,
            })
            .next()
            .is_none()
    }

    fn string_has_background_color(string: &str, state: &State, config: &Config) -> bool {
        let painted = config
            .get_color(state, Background)
            .unwrap_or_else(|| panic!("state {:?} does not have a background color", state))
            .paint("");
        let ansi_sequence = painted
            .trim_end_matches(paint::ANSI_SGR_RESET)
            .trim_end_matches("m");
        string.starts_with(ansi_sequence)
    }

    fn string_has_some_background_color(s: &str) -> bool {
        s.starts_with("\x1b[48;")
    }

    fn string_has_some_foreground_color(s: &str) -> bool {
        s.starts_with("\x1b[38;")
    }

    pub fn assert_line_has_expected_ansi_sequences(line: &str, expected: &Vec<(&str, &str)>) {
        let parsed_line = line.ansi_parse().filter(|token| match token {
            Escape(SetGraphicsMode(parameters)) if parameters == &vec![0 as u32] => false,
            _ => true,
        });
        for ((expected_ansi_sequence, _), ref token) in expected.iter().zip_eq(parsed_line) {
            match token {
                TextBlock(s) => {
                    assert!(s.starts_with(*expected_ansi_sequence));
                }
                Escape(SetGraphicsMode(parameters)) => assert_eq!(parameters, &vec![0 as u32]),
                _ => panic!("Unexpected token: {:?}", token),
            }
        }
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
        let string_with_plus_color_only = config.plus_style.paint(&string_without_any_color);
        (
            string_without_any_color.to_string(),
            string_with_plus_color_only.to_string(),
        )
    }
}
