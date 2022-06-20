use std::borrow::Cow;
use std::path::Path;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;
use crate::features::OptionValueFunction;
use crate::git_config::{GitConfig, GitConfigEntry, GitRemoteRepo};

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "hyperlinks",
            bool,
            None,
            _opt => true
        )
    ])
}

pub fn format_commit_line_with_osc8_commit_hyperlink<'a>(
    line: &'a str,
    config: &Config,
) -> Cow<'a, str> {
    if let Some(commit_link_format) = &config.hyperlinks_commit_link_format {
        COMMIT_LINE_REGEX.replace(line, |captures: &Captures| {
            let prefix = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let commit = captures.get(2).map(|m| m.as_str()).unwrap();
            let suffix = captures.get(3).map(|m| m.as_str()).unwrap_or("");
            let formatted_commit =
                format_osc8_hyperlink(&commit_link_format.replace("{commit}", commit), commit);
            format!("{}{}{}", prefix, formatted_commit, suffix)
        })
    } else if let Some(GitConfigEntry::GitRemote(repo)) =
        config.git_config.as_ref().and_then(get_remote_url)
    {
        COMMIT_LINE_REGEX.replace(line, |captures: &Captures| {
            format_commit_line_captures_with_osc8_commit_hyperlink(captures, &repo)
        })
    } else {
        Cow::from(line)
    }
}

fn get_remote_url(git_config: &GitConfig) -> Option<GitConfigEntry> {
    git_config
        .repo
        .as_ref()?
        .find_remote("origin")
        .ok()?
        .url()
        .and_then(|url| {
            GitRemoteRepo::from_str(url)
                .ok()
                .map(GitConfigEntry::GitRemote)
        })
}

/// Create a file hyperlink, displaying `text`.
pub fn format_osc8_file_hyperlink<'a, P>(
    absolute_path: P,
    line_number: Option<usize>,
    text: &str,
    config: &Config,
) -> Cow<'a, str>
where
    P: AsRef<Path>,
    P: std::fmt::Debug,
{
    debug_assert!(absolute_path.as_ref().is_absolute());
    let mut url = config
        .hyperlinks_file_link_format
        .replace("{path}", &absolute_path.as_ref().to_string_lossy());
    if let Some(n) = line_number {
        url = url.replace("{line}", &format!("{}", n))
    } else {
        url = url.replace("{line}", "")
    };
    Cow::from(format_osc8_hyperlink(&url, text))
}

fn format_osc8_hyperlink(url: &str, text: &str) -> String {
    format!(
        "{osc}8;;{url}{st}{text}{osc}8;;{st}",
        url = url,
        text = text,
        osc = "\x1b]",
        st = "\x1b\\"
    )
}

lazy_static! {
    static ref COMMIT_LINE_REGEX: Regex = Regex::new("(.* )?([0-9a-f]{8,40})(.*)").unwrap();
}

fn format_commit_line_captures_with_osc8_commit_hyperlink(
    captures: &Captures,
    repo: &GitRemoteRepo,
) -> String {
    let commit = captures.get(2).unwrap().as_str();
    format!(
        "{prefix}{osc}8;;{url}{st}{commit}{osc}8;;{st}{suffix}",
        url = repo.format_commit_url(commit),
        commit = commit,
        prefix = captures.get(1).map(|m| m.as_str()).unwrap_or(""),
        suffix = captures.get(3).unwrap().as_str(),
        osc = "\x1b]",
        st = "\x1b\\"
    )
}

#[cfg(not(target_os = "windows"))]
#[cfg(test)]
pub mod tests {
    use std::iter::FromIterator;
    use std::path::PathBuf;

    use super::*;
    use crate::{
        tests::integration_test_utils::{self, DeltaTest},
        utils,
    };

    #[test]
    fn test_paths_and_hyperlinks_user_in_repo_root_dir() {
        // Expectations are uninfluenced by git's --relative and delta's relative_paths options.
        let input_type = InputType::GitDiff;
        let true_location_of_file_relative_to_repo_root = PathBuf::from("a");
        let git_prefix_env_var = Some("");

        for (delta_relative_paths_option, calling_cmd) in vec![
            (false, Some("git diff")),
            (false, Some("git diff --relative")),
            (true, Some("git diff")),
            (true, Some("git diff --relative")),
        ] {
            run_test(FilePathsTestCase {
                name: &format!(
                    "delta relative_paths={} calling_cmd={:?}",
                    delta_relative_paths_option, calling_cmd
                ),
                true_location_of_file_relative_to_repo_root:
                    true_location_of_file_relative_to_repo_root.as_path(),
                git_prefix_env_var,
                delta_relative_paths_option,
                input_type,
                calling_cmd,
                path_in_delta_input: "a",
                expected_displayed_path: "a",
            })
        }
    }

    #[test]
    fn test_paths_and_hyperlinks_user_in_subdir_file_in_same_subdir() {
        let input_type = InputType::GitDiff;
        let true_location_of_file_relative_to_repo_root = PathBuf::from_iter(&["b", "a"]);
        let git_prefix_env_var = Some("b");

        run_test(FilePathsTestCase {
            name: "b/a from b",
            input_type,
            calling_cmd: Some("git diff"),
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            delta_relative_paths_option: false,
            path_in_delta_input: "b/a",
            expected_displayed_path: "b/a",
        });
        run_test(FilePathsTestCase {
            name: "b/a from b",
            input_type,
            calling_cmd: Some("git diff --relative"),
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            delta_relative_paths_option: false,
            path_in_delta_input: "a",
            // delta saw a and wasn't configured to make any changes
            expected_displayed_path: "a",
        });
        run_test(FilePathsTestCase {
            name: "b/a from b",
            input_type,
            calling_cmd: Some("git diff"),
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            delta_relative_paths_option: true,
            path_in_delta_input: "b/a",
            // delta saw b/a and changed it to a
            expected_displayed_path: "a",
        });
        run_test(FilePathsTestCase {
            name: "b/a from b",
            input_type,
            calling_cmd: Some("git diff --relative"),
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            delta_relative_paths_option: true,
            path_in_delta_input: "a",
            // delta saw a and didn't change it
            expected_displayed_path: "a",
        });
    }

    #[test]
    fn test_paths_and_hyperlinks_user_in_subdir_file_in_different_subdir() {
        let input_type = InputType::GitDiff;
        let true_location_of_file_relative_to_repo_root = PathBuf::from_iter(&["b", "a"]);
        let git_prefix_env_var = Some("c");

        run_test(FilePathsTestCase {
            name: "b/a from c",
            input_type,
            calling_cmd: Some("git diff"),
            delta_relative_paths_option: false,
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            path_in_delta_input: "b/a",
            expected_displayed_path: "b/a",
        });
        run_test(FilePathsTestCase {
            name: "b/a from c",
            input_type,
            calling_cmd: Some("git diff --relative"),
            delta_relative_paths_option: false,
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            path_in_delta_input: "../b/a",
            expected_displayed_path: "../b/a",
        });
        run_test(FilePathsTestCase {
            name: "b/a from c",
            input_type,
            calling_cmd: Some("git diff"),
            delta_relative_paths_option: true,
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var,
            path_in_delta_input: "b/a",
            expected_displayed_path: "../b/a",
        });
    }

    #[test]
    fn test_paths_and_hyperlinks_git_grep_user_in_root() {
        let input_type = InputType::Grep;
        let true_location_of_file_relative_to_repo_root = PathBuf::from_iter(&["b", "a.txt"]);

        run_test(FilePathsTestCase {
            name: "git grep: b/a.txt from root dir",
            input_type,
            calling_cmd: Some("git grep foo"),
            delta_relative_paths_option: false,
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var: Some(""),
            path_in_delta_input: "b/a.txt",
            expected_displayed_path: "b/a.txt:",
        });
    }

    #[test]
    fn test_paths_and_hyperlinks_grep_user_in_subdir_file_in_same_subdir() {
        _run_test_grep_user_in_subdir_file_in_same_subdir(Some("git grep foo"));
        _run_test_grep_user_in_subdir_file_in_same_subdir(Some("rg foo"));
    }

    fn _run_test_grep_user_in_subdir_file_in_same_subdir(calling_cmd: Option<&str>) {
        let input_type = InputType::Grep;
        let true_location_of_file_relative_to_repo_root = PathBuf::from_iter(&["b", "a.txt"]);
        run_test(FilePathsTestCase {
            name: "git grep: b/a.txt from b/ dir",
            input_type,
            calling_cmd,
            delta_relative_paths_option: false,
            true_location_of_file_relative_to_repo_root:
                true_location_of_file_relative_to_repo_root.as_path(),
            git_prefix_env_var: Some("b/"),
            path_in_delta_input: "a.txt",
            expected_displayed_path: "a.txt:",
        });
    }

    const GIT_DIFF_OUTPUT: &str = r#"
diff --git a/__path__ b/__path__
index 587be6b..975fbec 100644
--- a/__path__
+++ b/__path__
@@ -1 +1 @@
-x
+y
    "#;

    const GIT_GREP_OUTPUT: &str = "\
__path__:  some matching line
";

    struct FilePathsTestCase<'a> {
        // True location of file in repo
        true_location_of_file_relative_to_repo_root: &'a Path,

        // Git spawns delta from repo root, and stores in this env var the cwd in which the user invoked delta.
        git_prefix_env_var: Option<&'a str>,

        delta_relative_paths_option: bool,
        input_type: InputType,
        calling_cmd: Option<&'a str>,
        path_in_delta_input: &'a str,
        expected_displayed_path: &'a str,
        #[allow(dead_code)]
        name: &'a str,
    }

    #[derive(Debug)]
    enum GitDiffRelative {
        Yes,
        No,
    }

    #[derive(Debug)]
    enum CallingProcess {
        GitDiff(GitDiffRelative),
        GitGrep,
        OtherGrep,
    }

    #[derive(Clone, Copy, Debug)]
    enum InputType {
        GitDiff,
        Grep,
    }

    impl<'a> FilePathsTestCase<'a> {
        pub fn get_args(&self) -> Vec<String> {
            let mut args = vec![
                "--navigate", // helps locate the file path in the output
                "--line-numbers",
                "--hyperlinks",
                "--hyperlinks-file-link-format",
                "{path}",
                "--grep-file-style",
                "raw",
                "--grep-line-number-style",
                "raw",
                "--hunk-header-file-style",
                "raw",
                "--hunk-header-line-number-style",
                "raw",
                "--line-numbers-plus-style",
                "raw",
                "--line-numbers-left-style",
                "raw",
                "--line-numbers-right-style",
                "raw",
                "--line-numbers-left-format",
                "{nm}અ",
                "--line-numbers-right-format",
                "{np}જ",
            ];
            if self.delta_relative_paths_option {
                args.push("--relative-paths");
            }
            args.iter().map(|s| s.to_string()).collect()
        }

        pub fn calling_process(&self) -> CallingProcess {
            match (&self.input_type, self.calling_cmd) {
                (InputType::GitDiff, Some(s)) if s.starts_with("git diff --relative") => {
                    CallingProcess::GitDiff(GitDiffRelative::Yes)
                }
                (InputType::GitDiff, Some(s)) if s.starts_with("git diff") => {
                    CallingProcess::GitDiff(GitDiffRelative::No)
                }
                (InputType::Grep, Some(s)) if s.starts_with("git grep") => CallingProcess::GitGrep,
                (InputType::Grep, Some(s)) if s.starts_with("rg") => CallingProcess::OtherGrep,
                (InputType::Grep, None) => CallingProcess::GitGrep,
                _ => panic!(
                    "Unexpected calling spec: {:?} {:?}",
                    self.input_type, self.calling_cmd
                ),
            }
        }

        pub fn path_in_git_output(&self) -> String {
            match self.calling_process() {
                CallingProcess::GitDiff(GitDiffRelative::No) => self
                    .true_location_of_file_relative_to_repo_root
                    .to_string_lossy()
                    .to_string(),
                CallingProcess::GitDiff(GitDiffRelative::Yes) => pathdiff::diff_paths(
                    self.true_location_of_file_relative_to_repo_root,
                    self.git_prefix_env_var.unwrap(),
                )
                .unwrap()
                .to_string_lossy()
                .into(),
                _ => panic!("Unexpected calling process: {:?}", self.calling_process()),
            }
        }

        /// Return the relative path as it would appear in grep output, i.e. accounting for facts
        /// such as that that the user may have invoked the grep command from a non-root directory
        /// in the repo.
        pub fn path_in_grep_output(&self) -> String {
            use CallingProcess::*;
            match (self.calling_process(), self.git_prefix_env_var) {
                (GitGrep, None) => self
                    .true_location_of_file_relative_to_repo_root
                    .to_string_lossy()
                    .into(),
                (GitGrep, Some(dir)) => {
                    // Delta must have been invoked as core.pager since GIT_PREFIX env var is set.
                    // Note that it is possible that `true_location_of_file_relative_to_repo_root`
                    // is not under `git_prefix_env_var` since one can do things like `git grep foo
                    // ..`
                    pathdiff::diff_paths(self.true_location_of_file_relative_to_repo_root, dir)
                        .unwrap()
                        .to_string_lossy()
                        .into()
                }
                (OtherGrep, None) => {
                    // Output from e.g. rg has been piped to delta.
                    // Therefore
                    // (a) the cwd that the delta process reports is the user's shell process cwd
                    // (b) the file in question must be under this cwd
                    // (c) grep output will contain the path relative to this cwd

                    // So to compute the path as it would appear in grep output, we could form the
                    // absolute path to the file and strip off the config.cwd_of_delta_process
                    // prefix. The absolute path to the file could be constructed as (absolute path
                    // to repo root) + true_location_of_file_relative_to_repo_root). But I don't
                    // think we know the absolute path to repo root.
                    panic!("Not implemented")
                }
                _ => panic!("Not implemented"),
            }
        }

        pub fn expected_hyperlink_path(&self) -> PathBuf {
            utils::path::fake_delta_cwd_for_tests()
                .join(self.true_location_of_file_relative_to_repo_root)
        }
    }

    fn run_test(test_case: FilePathsTestCase) {
        let mut config = integration_test_utils::make_config_from_args(
            &test_case
                .get_args()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .as_slice(),
        );
        // The test is simulating delta invoked by git hence these are the same
        config.cwd_relative_to_repo_root = test_case.git_prefix_env_var.map(|s| s.to_string());
        config.cwd_of_user_shell_process = utils::path::cwd_of_user_shell_process(
            config.cwd_of_delta_process.as_ref(),
            config.cwd_relative_to_repo_root.as_deref(),
        );
        let mut delta_test = DeltaTest::with_config(&config);
        if let Some(cmd) = test_case.calling_cmd {
            delta_test = delta_test.with_calling_process(cmd)
        }
        let delta_test = match test_case.calling_process() {
            CallingProcess::GitDiff(_) => {
                assert_eq!(
                    test_case.path_in_delta_input,
                    test_case.path_in_git_output()
                );
                delta_test
                    .with_input(&GIT_DIFF_OUTPUT.replace("__path__", test_case.path_in_delta_input))
            }
            CallingProcess::GitGrep => {
                assert_eq!(
                    test_case.path_in_delta_input,
                    test_case.path_in_grep_output()
                );
                delta_test.with_input(
                    &GIT_GREP_OUTPUT.replace("__path__", &test_case.path_in_delta_input),
                )
            }
            CallingProcess::OtherGrep => delta_test
                .with_input(&GIT_GREP_OUTPUT.replace("__path__", &test_case.path_in_delta_input)),
        };
        let make_expected_hyperlink = |text| {
            format_osc8_hyperlink(
                &PathBuf::from(test_case.expected_hyperlink_path()).to_string_lossy(),
                text,
            )
        };
        match test_case.calling_process() {
            CallingProcess::GitDiff(_) => {
                let line_number = "1";
                delta_test
                    .inspect_raw()
                    // file hyperlink
                    .expect_raw_contains(&format!(
                        "Δ {}",
                        make_expected_hyperlink(test_case.expected_displayed_path)
                    ))
                    // hunk header hyperlink
                    .expect_raw_contains(&format!("• {}", make_expected_hyperlink(line_number)))
                    // line number hyperlink
                    .expect_raw_contains(&format!("અ{}જ", make_expected_hyperlink(line_number)));
            }
            CallingProcess::GitGrep | CallingProcess::OtherGrep => {
                delta_test
                    .inspect_raw()
                    .expect_raw_contains(&make_expected_hyperlink(
                        test_case.expected_displayed_path,
                    ));
            }
        }
    }
}
