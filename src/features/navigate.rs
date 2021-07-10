use std::io::Write;
#[cfg(target_os = "windows")]
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

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

// Construct the regexp used by less for paging, if --show-themes or --navigate is enabled.
pub fn make_navigate_regexp(
    show_themes: bool,
    file_modified_label: &str,
    file_added_label: &str,
    file_removed_label: &str,
    file_renamed_label: &str,
) -> String {
    if show_themes {
        "^Theme:".to_string()
    } else {
        format!(
            "^(commit|{}|{}|{}|{})",
            file_modified_label, file_added_label, file_removed_label, file_renamed_label,
        )
    }
}

// Create a less history file to be used by delta's child less process. This file is initialized
// with the contents of user's real less hist file, to which the navigate regexp is appended. This
// has the effect that 'n' or 'N' in delta's less process will search for the navigate regexp,
// without the undesirable aspects of using --pattern, yet without polluting the user's less search
// history with delta's navigate regexp. See
// https://github.com/dandavison/delta/issues/237#issuecomment-780654036. Note that with the
// current implementation, no writes to the delta less history file are propagated back to the real
// history file so, for example, a (non-navigate) search performed in the delta less process will
// not be stored in history.
pub fn copy_less_hist_file_and_append_navigate_regexp(config: &Config) -> std::io::Result<PathBuf> {
    let delta_less_hist_file = get_delta_less_hist_file()?;
    let initial_contents = ".less-history-file:\n".to_string();
    let mut contents = if let Some(hist_file) = get_less_hist_file() {
        std::fs::read_to_string(hist_file).unwrap_or(initial_contents)
    } else {
        initial_contents
    };
    if !contents.ends_with(".search\n") {
        contents = format!("{}.search\n", contents);
    }
    writeln!(
        std::fs::File::create(&delta_less_hist_file)?,
        "{}\"{}",
        contents,
        config.navigate_regexp.as_ref().unwrap(),
    )?;
    Ok(delta_less_hist_file)
}

#[cfg(target_os = "windows")]
fn get_delta_less_hist_file() -> std::io::Result<PathBuf> {
    let mut path = dirs_next::home_dir()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Can't determine home dir"))?;
    path.push(".delta.lesshst");
    Ok(path)
}

#[cfg(not(target_os = "windows"))]
fn get_delta_less_hist_file() -> std::io::Result<PathBuf> {
    let dir = xdg::BaseDirectories::with_prefix("delta")?;
    dir.place_data_file("lesshst")
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
    if let Some(home_dir) = dirs_next::home_dir() {
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
