use crate::features::FeatureValueFunction;

pub fn make_feature() -> Vec<(String, FeatureValueFunction)> {
    _make_feature(false)
}

pub fn _make_feature(bold: bool) -> Vec<(String, FeatureValueFunction)> {
    builtin_feature!([
        (
            "minus-style",
            String,
            Some("color.diff.old"),
            _opt => if bold { "bold red" } else { "red" }
        ),
        (
            "minus-non-emph-style",
            String,
            Some("color.diff-highlight.oldNormal"),
            opt => opt.minus_style.clone()
        ),
        (
            "minus-emph-style",
            String,
            Some("color.diff-highlight.oldHighlight"),
            opt => format!("{} reverse", opt.minus_style)
        ),
        (
            "zero-style",
            String,
            None,
            _opt => "normal"
        ),
        (
            "plus-style",
            String,
            Some("color.diff.new"),
            _opt => if bold { "bold green" } else { "green" }
        ),
        (
            "plus-non-emph-style",
            String,
            Some("color.diff-highlight.newNormal"),
            opt => opt.plus_style.clone()
        ),
        (
            "plus-emph-style",
            String,
            Some("color.diff-highlight.newHighlight"),
            opt => format!("{} reverse", opt.plus_style)
        )
    ])
}

#[cfg(test)]
mod test_utils {
    use std::fs::remove_file;

    use crate::features;

    #[test]
    fn test_diff_highlight_defaults() {
        let config = features::tests::make_config(&["--features", "diff-highlight"], None, None);

        assert_eq!(config.minus_style, features::tests::make_style("red"));
        assert_eq!(
            config.minus_non_emph_style,
            features::tests::make_style("red")
        );
        assert_eq!(
            config.minus_emph_style,
            features::tests::make_emph_style("red reverse")
        );
        assert_eq!(config.zero_style, features::tests::make_style(""));
        assert_eq!(config.plus_style, features::tests::make_style("green"));
        assert_eq!(
            config.plus_non_emph_style,
            features::tests::make_style("green")
        );
        assert_eq!(
            config.plus_emph_style,
            features::tests::make_emph_style("green reverse")
        );
    }

    #[test]
    fn test_diff_highlight_respects_gitconfig() {
        let git_config_contents = b"
[color \"diff\"]
    old = red bold
    new = green bold

[color \"diff-highlight\"]
    oldNormal = ul red bold
    oldHighlight = red bold 52
    newNormal = ul green bold
    newHighlight = green bold 22
";
        let git_config_path = "delta__test_diff_highlight.gitconfig";

        let config = features::tests::make_config(
            &["--features", "diff-highlight"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(config.minus_style, features::tests::make_style("red bold"));
        assert_eq!(
            config.minus_non_emph_style,
            features::tests::make_style("ul red bold")
        );
        assert_eq!(
            config.minus_emph_style,
            features::tests::make_emph_style("red bold 52")
        );
        assert_eq!(config.zero_style, features::tests::make_style(""));
        assert_eq!(config.plus_style, features::tests::make_style("green bold"));
        assert_eq!(
            config.plus_non_emph_style,
            features::tests::make_style("ul green bold")
        );
        assert_eq!(
            config.plus_emph_style,
            features::tests::make_emph_style("green bold 22")
        );

        remove_file(git_config_path).unwrap();
    }
}
