use crate::features::diff_highlight;
use crate::features::FeatureValueFunction;

pub fn make_feature() -> Vec<(String, FeatureValueFunction)> {
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
            "commit-style",
            String,
            None,
            _opt => "bold yellow"
        ),
        (
            "commit-decoration-style",
            String,
            None,
            _opt => "none"
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
            _opt => "bold syntax"
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

    use crate::features;

    #[test]
    fn test_diff_so_fancy_defaults() {
        let config = features::tests::make_config(&["--features", "diff-so-fancy"], None, None);

        assert_eq!(
            config.commit_style.ansi_term_style,
            features::tests::make_style("bold yellow").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            features::tests::make_decoration_style("none")
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            features::tests::make_style("11").ansi_term_style
        );
        assert_eq!(
            config.file_style.decoration_style,
            features::tests::make_decoration_style("bold yellow ul ol")
        );

        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            features::tests::make_style("bold syntax").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            features::tests::make_decoration_style("magenta box")
        );
    }

    #[test]
    fn test_diff_so_fancy_respects_git_config() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse
";
        let git_config_path = "delta__test_diff_so_fancy.gitconfig";

        let config = features::tests::make_config(
            &["--features", "diff-so-fancy some-other-feature"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.commit_style.ansi_term_style,
            features::tests::make_style("yellow bold").ansi_term_style
        );
        assert_eq!(
            config.file_style.ansi_term_style,
            features::tests::make_style("11").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            features::tests::make_style("magenta bold").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            features::tests::make_decoration_style("none")
        );
        assert_eq!(
            config.file_style.decoration_style,
            features::tests::make_decoration_style("yellow bold ul ol")
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            features::tests::make_decoration_style("magenta box")
        );

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

        let config = features::tests::make_config(
            &["--features", "decorations diff-so-fancy"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            features::tests::make_style("11").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            features::tests::make_decoration_style("yellow bold ul ol")
        );

        let config = features::tests::make_config(
            &["--features", "diff-so-fancy decorations"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            features::tests::make_style("ul bold 19").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            features::tests::make_decoration_style("none")
        );

        remove_file(git_config_path).unwrap();
    }
}
