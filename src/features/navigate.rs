use crate::config::Config;
use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
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

pub fn make_navigate_regexp(config: &Config) -> String {
    format!(
        "^(commit|{}|{}|{}|{})",
        config.file_modified_label,
        config.file_added_label,
        config.file_removed_label,
        config.file_renamed_label
    )
}

#[cfg(test)]
mod tests {
    use std::fs::remove_file;

    use crate::tests::integration_test_utils::integration_test_utils;

    #[test]
    fn test_navigate_with_overriden_key_in_main_section() {
        let git_config_contents = b"
[delta]
    features = navigate
    file-modified-label = \"modified: \"
";
        let git_config_path = "delta__test_navigate_with_overriden_key_in_main_section.gitconfig";

        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(&[], None, None)
                .file_modified_label,
            ""
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &["--features", "navigate"],
                None,
                None
            )
            .file_modified_label,
            "Δ"
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &["--navigate"],
                None,
                None
            )
            .file_modified_label,
            "Δ"
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            "modified: "
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_navigate_with_overriden_key_in_custom_navigate_section() {
        let git_config_contents = b"
[delta]
    features = navigate

[delta \"navigate\"]
    file-modified-label = \"modified: \"
";
        let git_config_path =
            "delta__test_navigate_with_overriden_key_in_custom_navigate_section.gitconfig";

        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(&[], None, None)
                .file_modified_label,
            ""
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &["--features", "navigate"],
                None,
                None
            )
            .file_modified_label,
            "Δ"
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            "modified: "
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_navigate_activated_by_custom_feature() {
        let git_config_contents = b"
[delta \"my-navigate-feature\"]
    features = navigate
    file-modified-label = \"modified: \"
";
        let git_config_path = "delta__test_navigate_activated_by_custom_feature.gitconfig";

        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            ""
        );
        assert_eq!(
            integration_test_utils::make_options_from_args_and_git_config(
                &["--features", "my-navigate-feature"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            "modified: "
        );

        remove_file(git_config_path).unwrap();
    }
}
