use std::io::Write;
#[cfg(target_os = "windows")]
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use crate::features::OptionValueFunction;
use crate::utils::bat::output::PagerCfg;

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
        ),
        (
            "hunk-label",
            String,
            None,
            _opt => "•"
        )
    ])
}

// Construct the regexp used by less for paging, if --show-themes or --navigate is enabled.
pub fn make_navigate_regex(
    show_themes: bool,
    file_modified_label: &str,
    file_added_label: &str,
    file_removed_label: &str,
    file_renamed_label: &str,
    hunk_label: &str,
) -> String {
    if show_themes {
        "^Theme:".to_string()
    } else {
        let optional_regexp = |find: &str| {
            if !find.is_empty() {
                format!("|{}", regex::escape(find))
            } else {
                "".to_string()
            }
        };
        format!(
            "^(commit{}{}{}{}{})",
            optional_regexp(file_added_label),
            optional_regexp(file_removed_label),
            optional_regexp(file_renamed_label),
            optional_regexp(file_modified_label),
            optional_regexp(hunk_label),
        )
    }
}

// Append the navigate regex to the user's less history file. This has the
// effect that 'n' or 'N' in delta's less process will search for the navigate
// regex, without the undesirable aspects of using --pattern. See
// https://github.com/dandavison/delta/issues/237#issuecomment-780654036. Note
// that with the current implementation, delta's automatically-added navigate
// regexp will be stored in less history.
pub fn copy_less_hist_file_and_append_navigate_regex(
    config: &PagerCfg,
) -> std::io::Result<PathBuf> {
    let less_hist_file = get_less_hist_file().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Can't find less hist file")
    })?;
    let mut contents = std::fs::read_to_string(&less_hist_file)
        .unwrap_or_else(|_| ".less-history-file:\n".to_string());
    if !contents.ends_with(".search\n") {
        contents = format!("{contents}.search\n");
    }
    writeln!(
        std::fs::File::create(&less_hist_file)?,
        "{}\"{}",
        contents,
        config.navigate_regex.as_ref().unwrap(),
    )?;
    Ok(less_hist_file)
}

// LESSHISTFILE
//        Name of the history file used to remember search commands
//        and shell commands between invocations of less.  If set to
//        "-" or "/dev/null", a history file is not used.  The
//        default is "$HOME/.lesshst" on Unix systems,
//        "$HOME/_lesshst" on DOS and Windows systems, or
//        "$HOME/lesshst.ini" or "$INIT/lesshst.ini" on OS/2
//        systems.
fn get_less_hist_file() -> Option<PathBuf> {
    if let Some(home_dir) = dirs::home_dir() {
        match std::env::var("LESSHISTFILE").as_deref() {
            Ok("-") | Ok("/dev/null") => {
                // The user has explicitly disabled less history.
                None
            }
            Ok(path) => {
                // The user has specified a custom histfile
                Some(PathBuf::from(path))
            }
            Err(_) => {
                // The user is using the default less histfile location.
                let mut hist_file = home_dir;
                hist_file.push(if cfg!(windows) {
                    "_lesshst"
                } else {
                    ".lesshst"
                });
                Some(hist_file)
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs::remove_file;

    use crate::tests::integration_test_utils;

    #[test]
    fn test_navigate_with_overridden_key_in_main_section() {
        let git_config_contents = b"
[delta]
    features = navigate
    file-modified-label = \"modified: \"
";
        let git_config_path = "delta__test_navigate_with_overridden_key_in_main_section.gitconfig";

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
    fn test_navigate_with_overridden_key_in_custom_navigate_section() {
        let git_config_contents = b"
[delta]
    features = navigate

[delta \"navigate\"]
    file-modified-label = \"modified: \"
";
        let git_config_path =
            "delta__test_navigate_with_overridden_key_in_custom_navigate_section.gitconfig";

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
