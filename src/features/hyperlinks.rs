use std::borrow::Cow;
use std::path::Path;

use lazy_static::lazy_static;
use regex::{Match, Matches, Regex};

use crate::config::Config;
use crate::features::OptionValueFunction;

#[cfg(test)]
use crate::git_config::GitConfig;

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

lazy_static! {
    // Commit hashes can be abbreviated to 7 characters, these necessarily become longer
    // when more objects are in a repository.
    // Note: pure numbers are filtered out later again.
    static ref COMMIT_HASH_REGEX: Regex = Regex::new(r"\b[0-9a-f]{7,40}\b").unwrap();
}

pub fn format_commit_line_with_osc8_commit_hyperlink<'a>(
    line: &'a str,
    config: &Config,
) -> Cow<'a, str> {
    // Given matches in a line, m = matches[0] and pos = 0: store line[pos..m.start()] first, then
    // store the T(line[m.start()..m.end()]) match transformation, then set pos = m.end().
    // Repeat for matches[1..]. Finally, store line[pos..].
    struct HyperlinkCommits<T>(T)
    where
        T: Fn(&str) -> String;
    impl<T: for<'b> Fn(&'b str) -> String> HyperlinkCommits<T> {
        fn _m(&self, result: &mut String, line: &str, m: &Match, prev_pos: usize) -> usize {
            result.push_str(&line[prev_pos..m.start()]);
            let commit = &line[m.start()..m.end()];
            // Do not link numbers, require at least one non-decimal:
            if commit.contains(|c| matches!(c, 'a'..='f')) {
                result.push_str(&format_osc8_hyperlink(&self.0(commit), commit));
            } else {
                result.push_str(commit);
            }
            m.end()
        }
        fn with_input(&self, line: &str, m0: &Match, matches123: &mut Matches) -> String {
            let mut result = String::new();
            let mut pos = self._m(&mut result, line, m0, 0);
            // limit number of matches per line, an exhaustive `find_iter` is O(len(line) * len(regex)^2)
            for m in matches123.take(12) {
                pos = self._m(&mut result, line, &m, pos);
            }
            result.push_str(&line[pos..]);
            result
        }
    }

    if let Some(commit_link_format) = &config.hyperlinks_commit_link_format {
        let mut matches = COMMIT_HASH_REGEX.find_iter(line);
        if let Some(first_match) = matches.next() {
            let result =
                HyperlinkCommits(|commit_hash| commit_link_format.replace("{commit}", commit_hash))
                    .with_input(line, &first_match, &mut matches);
            return Cow::from(result);
        }
    } else if let Some(config) = config.git_config() {
        if let Some(repo) = config.get_remote_url() {
            let mut matches = COMMIT_HASH_REGEX.find_iter(line);
            if let Some(first_match) = matches.next() {
                let result = HyperlinkCommits(|commit_hash| repo.format_commit_url(commit_hash))
                    .with_input(line, &first_match, &mut matches);
                return Cow::from(result);
            }
        }
    }
    Cow::from(line)
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
    if let Some(host) = &config.hostname {
        url = url.replace("{host}", host)
    }
    let n = line_number.unwrap_or(1);
    url = url.replace("{line}", &format!("{n}"));
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

#[cfg(not(target_os = "windows"))]
#[cfg(test)]
pub mod tests {
    use std::iter::FromIterator;
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use super::*;

    use crate::{
        tests::integration_test_utils::{self, make_config_from_args, DeltaTest},
        utils,
    };

    #[test]
    fn test_file_hyperlink_line_number_defaults_to_one() {
        let config =
            make_config_from_args(&["--hyperlinks-file-link-format", "file://{path}:{line}"]);

        let result =
            format_osc8_file_hyperlink("/absolute/path/to/file.rs", Some(42), "file.rs", &config);
        assert_eq!(
            result,
            "\u{1b}]8;;file:///absolute/path/to/file.rs:42\u{1b}\\file.rs\u{1b}]8;;\u{1b}\\",
        );

        let result =
            format_osc8_file_hyperlink("/absolute/path/to/file.rs", None, "file.rs", &config);
        assert_eq!(
            result,
            "\u{1b}]8;;file:///absolute/path/to/file.rs:1\u{1b}\\file.rs\u{1b}]8;;\u{1b}\\",
        );
    }

    #[test]
    fn test_formatted_hyperlinks() {
        let config = make_config_from_args(&["--hyperlinks-commit-link-format", "HERE:{commit}"]);

        let line = "001234abcdf";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "\u{1b}]8;;HERE:001234abcdf\u{1b}\\001234abcdf\u{1b}]8;;\u{1b}\\",
        );

        let line = "a2272718f0b398e48652ace17fca85c1962b3fc22"; // length: 41 > 40
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(result, "a2272718f0b398e48652ace17fca85c1962b3fc22",);

        let line = "a2272718f0+b398e48652ace17f,ca85c1962b3fc2";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(result, "\u{1b}]8;;HERE:a2272718f0\u{1b}\\a2272718f0\u{1b}]8;;\u{1b}\\+\u{1b}]8;;\
        HERE:b398e48652ace17f\u{1b}\\b398e48652ace17f\u{1b}]8;;\u{1b}\\,\u{1b}]8;;HERE:ca85c1962b3fc2\
        \u{1b}\\ca85c1962b3fc2\u{1b}]8;;\u{1b}\\");

        let line = "This 01234abcdf Hash";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "This \u{1b}]8;;HERE:01234abcdf\u{1b}\\01234abcdf\u{1b}]8;;\u{1b}\\ Hash",
        );

        let line =
            "Another 01234abcdf hash but also this one: dc623b084ad2dd14fe5d90189cacad5d49bfbfd3!";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "Another \u{1b}]8;;HERE:01234abcdf\u{1b}\\01234abcdf\u{1b}]8;;\u{1b}\\ hash but \
         also this one: \u{1b}]8;;HERE:dc623b084ad2dd14fe5d90189cacad5d49bfbfd3\u{1b}\
         \\dc623b084ad2dd14fe5d90189cacad5d49bfbfd3\u{1b}]8;;\u{1b}\\!"
        );

        let line = "01234abcdf 03043baf30 12abcdef0 12345678";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "\u{1b}]8;;HERE:01234abcdf\u{1b}\\01234abcdf\u{1b}]8;;\u{1b}\\ \u{1b}]8;;\
        HERE:03043baf30\u{1b}\\03043baf30\u{1b}]8;;\u{1b}\\ \u{1b}]8;;HERE:12abcdef0\u{1b}\\\
        12abcdef0\u{1b}]8;;\u{1b}\\ 12345678"
        );
    }

    #[test]
    fn test_hyperlinks_to_repo() {
        let mut config = make_config_from_args(&["--hyperlinks"]);
        config.git_config = GitConfig::for_testing();

        let line = "This a589ff9debaefdd delta commit";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "This \u{1b}]8;;https://github.com/dandavison/delta/commit/a589ff9debaefdd\u{1b}\
            \\a589ff9debaefdd\u{1b}]8;;\u{1b}\\ delta commit",
        );

        let line =
            "Another a589ff9debaefdd hash but also this one: c5696757c0827349a87daa95415656!";
        let result = format_commit_line_with_osc8_commit_hyperlink(line, &config);
        assert_eq!(
            result,
            "Another \u{1b}]8;;https://github.com/dandavison/delta/commit/a589ff9debaefdd\
        \u{1b}\\a589ff9debaefdd\u{1b}]8;;\u{1b}\\ hash but also this one: \u{1b}]8;;\
        https://github.com/dandavison/delta/commit/c5696757c0827349a87daa95415656\u{1b}\
        \\c5696757c0827349a87daa95415656\u{1b}]8;;\
         \u{1b}\\!"
        );
    }

    #[test]
    fn test_paths_and_hyperlinks_user_in_repo_root_dir() {
        // Expectations are uninfluenced by git's --relative and delta's relative_paths options.
        let input_type = InputType::GitDiff;
        let true_location_of_file_relative_to_repo_root = PathBuf::from("a");
        let git_prefix_env_var = Some("");

        for (delta_relative_paths_option, calling_cmd) in [
            (false, Some("git diff")),
            (false, Some("git diff --relative")),
            (true, Some("git diff")),
            (true, Some("git diff --relative")),
        ] {
            run_test(FilePathsTestCase {
                name: &format!(
                    "delta relative_paths={delta_relative_paths_option} calling_cmd={calling_cmd:?}",
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
                "--grep-output-type",
                "classic",
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
        /// such as that the user may have invoked the grep command from a non-root directory
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
            test_case
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
                delta_test
                    .with_input(&GIT_GREP_OUTPUT.replace("__path__", test_case.path_in_delta_input))
            }
            CallingProcess::OtherGrep => delta_test
                .with_input(&GIT_GREP_OUTPUT.replace("__path__", test_case.path_in_delta_input)),
        };
        let make_expected_hyperlink = |text| {
            format_osc8_hyperlink(&test_case.expected_hyperlink_path().to_string_lossy(), text)
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
