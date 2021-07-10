use crate::features::raw;
use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    _make_feature(false)
}

pub fn _make_feature(bold: bool) -> Vec<(String, OptionValueFunction)> {
    let mut feature = raw::make_feature();
    feature = feature
        .into_iter()
        .filter(|(s, _)| s != "keep-plus-minus-markers" && s != "tabs")
        .collect();
    feature.extend(builtin_feature!([
        (
            "commit-style",
            String,
            Some("color.diff.commit"),
            _opt => "raw"
        ),
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
    ]));
    feature
}

#[cfg(test)]
mod test_utils {
    use std::fs::remove_file;

    use crate::tests::integration_test_utils;

    #[test]
    fn test_diff_highlight_defaults() {
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "diff-highlight"],
            None,
            None,
        );
        assert_eq!(opt.minus_style, "red");
        assert_eq!(opt.minus_non_emph_style, "red");
        assert_eq!(opt.minus_emph_style, "red reverse");
        assert_eq!(opt.zero_style, "normal");
        assert_eq!(opt.plus_style, "green");
        assert_eq!(opt.plus_non_emph_style, "green");
        assert_eq!(opt.plus_emph_style, "green reverse");
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

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "diff-highlight"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.minus_style, "red bold");
        assert_eq!(opt.minus_non_emph_style, "ul red bold");
        assert_eq!(opt.minus_emph_style, "red bold 52");
        assert_eq!(opt.zero_style, "normal");
        assert_eq!(opt.plus_style, "green bold");
        assert_eq!(opt.plus_non_emph_style, "ul green bold");
        assert_eq!(opt.plus_emph_style, "green bold 22");

        remove_file(git_config_path).unwrap();
    }
}
