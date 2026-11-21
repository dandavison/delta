use crate::features::OptionValueFunction;

/// Styles for a diff drawn as difftastic sees it, as delta's TUI draws it while this feature is
/// enabled. Only what changed has a background: a changed token has the emph background, and a
/// line changed throughout has the line background. The rest of a changed line has none, so that
/// it looks unchanged. All text is syntax-highlighted.
pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "minus-style",
            String,
            None,
            _opt => "syntax auto"
        ),
        (
            "minus-non-emph-style",
            String,
            None,
            _opt => "syntax"
        ),
        (
            "minus-emph-style",
            String,
            None,
            _opt => "syntax auto"
        ),
        (
            "plus-style",
            String,
            None,
            _opt => "syntax auto"
        ),
        (
            "plus-non-emph-style",
            String,
            None,
            _opt => "syntax"
        ),
        (
            "plus-emph-style",
            String,
            None,
            _opt => "syntax auto"
        ),
        (
            "zero-style",
            String,
            None,
            _opt => "syntax"
        )
    ])
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
        assert_eq!(opt.minus_emph_style, "syntax auto");
        assert_eq!(opt.plus_style, "syntax auto");
        assert_eq!(opt.plus_emph_style, "syntax auto");
        assert_eq!(opt.zero_style, "syntax");
        // The lines delta pairs itself keep a background on their unmarked parts, so that a removed
        // line with nothing marked on it still looks removed.
        assert_eq!(opt.minus_non_emph_style, "minus-style");
        assert_eq!(opt.plus_non_emph_style, "plus-style");
    }
}
