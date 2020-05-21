#[cfg(test)]
pub mod ansi_test_utils {
    use console::strip_ansi_codes;

    use crate::config::Config;

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
