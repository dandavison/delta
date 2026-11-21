use crate::features::OptionValueFunction;

/// Styles for a diff drawn as difftastic sees it, as delta's TUI draws it while this feature is
/// enabled.
pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    vec![]
}

#[cfg(test)]
pub mod tests {
    use crate::tests::integration_test_utils;

    #[test]
    fn test_difftastic_styles() {
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "difftastic"],
            None,
            None,
        );

        assert_eq!(opt.minus_style, "syntax auto");
        assert_eq!(opt.minus_non_emph_style, "syntax");
        assert_eq!(opt.minus_emph_style, "syntax auto");
        assert_eq!(opt.plus_style, "syntax auto");
        assert_eq!(opt.plus_non_emph_style, "syntax");
        assert_eq!(opt.plus_emph_style, "syntax auto");
        assert_eq!(opt.zero_style, "syntax");
    }
}
