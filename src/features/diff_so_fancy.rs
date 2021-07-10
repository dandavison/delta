use crate::features::diff_highlight;
use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    let mut feature = diff_highlight::_make_feature(true);
    feature.extend(builtin_feature!([
        (
            "minus-emph-style",
            String,
            Some("color.diff-highlight.oldHighlight"),
            _opt => "bold red 52"
        ),
        (
            "plus-emph-style",
            String,
            Some("color.diff-highlight.newHighlight"),
            _opt => "bold green 22"
        ),
        (
            "file-style",
            String,
            Some("color.diff.meta"),
            _opt => "11"
        ),
        (
            "file-decoration-style",
            String,
            None,
            _opt => "bold yellow ul ol"
        ),
        (
            "hunk-header-style",
            String,
            Some("color.diff.frag"),
            _opt => "file line-number bold syntax"
        ),
        (
            "hunk-header-decoration-style",
            String,
            None,
            _opt => "magenta box"
        )
    ]));
    feature
}

#[cfg(test)]
pub mod tests {
    use std::fs::remove_file;

    use crate::tests::integration_test_utils;

    #[test]
    fn test_diff_so_fancy_defaults() {
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "diff-so-fancy"],
            None,
            None,
        );

        assert_eq!(opt.commit_style, "raw");
        assert_eq!(opt.commit_decoration_style, "none");

        assert_eq!(opt.file_style, "11");
        assert_eq!(opt.file_decoration_style, "bold yellow ul ol");

        assert_eq!(opt.hunk_header_style, "file line-number bold syntax");
        assert_eq!(opt.hunk_header_decoration_style, "magenta box");
    }

    #[test]
    fn test_diff_so_fancy_respects_git_config() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = purple bold
    old = red bold
    new = green bold
    whitespace = red reverse
";
        let git_config_path = "delta__test_diff_so_fancy.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "diff-so-fancy some-other-feature"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.commit_style, "purple bold");
        assert_eq!(opt.file_style, "11");
        assert_eq!(opt.hunk_header_style, "magenta bold");
        assert_eq!(opt.commit_decoration_style, "none");
        assert_eq!(opt.file_decoration_style, "bold yellow ul ol");
        assert_eq!(opt.hunk_header_decoration_style, "magenta box");

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_obeys_feature_precedence_rules() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse

[delta \"decorations\"]
    commit-decoration-style = bold box ul
    file-style = bold 19 ul
    file-decoration-style = none
";
        let git_config_path = "delta__test_diff_so_fancy_obeys_feature_precedence_rules.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "decorations diff-so-fancy"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.file_style, "11");

        assert_eq!(opt.file_decoration_style, "bold yellow ul ol");

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &["--features", "diff-so-fancy decorations"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.file_style, "bold 19 ul");

        assert_eq!(opt.file_decoration_style, "none");

        remove_file(git_config_path).unwrap();
    }
}
