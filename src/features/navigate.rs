/// Activate diff navigation: use n to jump forwards and N to jump backwards. To change the
/// file labels used see --file-modified-label, --file-removed-label, --file-added-label,
/// --file-renamed-label.
use crate::features::FeatureValueFunction;

pub fn make_feature() -> Vec<(String, FeatureValueFunction)> {
    builtin_feature!([
        (
            "navigate",
            bool,
            None,
            _opt => true
        ),
        (
            "file-modified-label",
            String,
            None,
            _opt => "Δ"
        )
    ])
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use itertools;

    use crate::config;
    use crate::git_config::GitConfig;
    use crate::style::Style;

    #[test]
    fn test_navigate() {
        let git_config_contents = b"
[delta]
    features = navigate
[delta \"navigate\"]
    file-modified-label = \"modified: \"
";
        let git_config_path = "delta__test_file_modified_label.gitconfig";

        assert_eq!(make_config(&[], None, None).file_modified_label, "");
        assert_eq!(
            make_config(&["--features", "navigate"], None, None).file_modified_label,
            "Δ"
        );
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).file_modified_label,
            "modified: "
        );

        let git_config_contents = b"
[delta \"my-navigate-feature\"]
    features = navigate
    file-modified-label = \"modified: \"
";

        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).file_modified_label,
            ""
        );
        assert_eq!(
            make_config(
                &["--features", "my-navigate-feature"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            "modified: "
        );

        remove_file(git_config_path).unwrap();
    }

    fn _make_style(s: &str, is_emph: bool) -> Style {
        Style::from_str(s, None, None, None, true, is_emph)
    }

    fn make_git_config(contents: &[u8], path: &str) -> GitConfig {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        GitConfig::from_path(&path)
    }

    fn make_config(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config {
        let args: Vec<&str> = itertools::chain(
            &["/dev/null", "/dev/null", "--24-bit-color", "always"],
            args,
        )
        .map(|s| *s)
        .collect();
        let mut git_config = match (git_config_contents, path) {
            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
            _ => None,
        };
        config::Config::from_args(&args, &mut git_config)
    }
}
