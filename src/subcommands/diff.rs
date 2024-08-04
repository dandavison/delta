use std::io::{BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;

use bytelines::ByteLinesReader;

use crate::config::{self, delta_unreachable};
use crate::delta;

pub enum Differ {
    GitDiff,
    Diff,
}

/// Run either `git diff` or `diff` on the files provided on the command line and display the
/// output. Try again with `diff` if `git diff` seems to have failed due to lack of support for
/// process substitution.
pub fn diff(
    minus_file: &Path,
    plus_file: &Path,
    differ: Differ,
    config: &config::Config,
    writer: &mut dyn Write,
) -> i32 {
    use std::io::BufReader;

    let diff_args = match shell_words::split(&config.diff_args) {
        Ok(words) => words,
        Err(err) => {
            eprintln!("Failed to parse diff args: {}: {err}", config.diff_args);
            return config.error_exit_code;
        }
    };

    let mut diff_cmd = match differ {
        Differ::GitDiff => vec!["git", "diff", "--no-index", "--color"],
        Differ::Diff => {
            if diff_args_set_unified_context(&diff_args) {
                vec!["diff"]
            } else {
                vec!["diff", "-U3"]
            }
        }
    };

    diff_cmd.extend(
        diff_args
            .iter()
            .filter(|s| !s.is_empty())
            .map(String::as_str),
    );
    diff_cmd.push("--");

    let (diff_bin, diff_cmd) = diff_cmd.split_first().unwrap();
    let diff_path = match grep_cli::resolve_binary(PathBuf::from(diff_bin)) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to resolve command '{diff_bin}': {err}");
            return config.error_exit_code;
        }
    };

    let diff_process = process::Command::new(&diff_path)
        .args(diff_cmd)
        .args([minus_file, plus_file])
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn();

    if let Err(err) = diff_process {
        eprintln!("Failed to execute the command '{diff_bin}': {err}");
        return config.error_exit_code;
    }
    let mut diff_process = diff_process.unwrap();

    if let Err(error) = delta::delta(
        BufReader::new(diff_process.stdout.take().unwrap()).byte_lines(),
        writer,
        config,
    ) {
        match error.kind() {
            ErrorKind::BrokenPipe => return 0,
            _ => {
                eprintln!("{error}");
                return config.error_exit_code;
            }
        }
    };

    // Return the exit code from the diff process, so that the exit code contract of `delta file1
    // file2` is the same as that of `diff file1 file2` (i.e. 0 if same, 1 if different, >= 2 if
    // error).
    let code = diff_process
        .wait()
        .unwrap_or_else(|_| {
            delta_unreachable(&format!("'{diff_bin}' process not running."));
        })
        .code()
        .unwrap_or_else(|| {
            eprintln!("'{diff_bin}' process terminated without exit status.");
            config.error_exit_code
        });

    if code >= 2 {
        let via_process_substitution =
            |f: &Path| f.starts_with("/proc/self/fd/") || f.starts_with("/dev/fd/");
        let is_git_diff = matches!(differ, Differ::GitDiff);
        if is_git_diff
            && code == 128
            && (via_process_substitution(minus_file) || via_process_substitution(plus_file))
        {
            // https://stackoverflow.com/questions/22706714/why-does-git-diff-not-work-with-process-substitution
            // When called as `delta <(echo foo) <(echo bar)`, then git < 2.42 just prints the diff of the
            // filenames which were created by the process substitution and does not read their content.

            // It looks like `git diff` failed due to lack of process substitution (version <2.42);
            // try again with `diff`.
            diff(minus_file, plus_file, Differ::Diff, config, writer)
        } else {
            for line in BufReader::new(diff_process.stderr.unwrap()).lines() {
                eprintln!("{}", line.unwrap_or("<delta: could not parse line>".into()));
                if code == 129 && is_git_diff {
                    // `git diff` unknown option: print first line (which is an error message) but not
                    // the remainder (which is the entire --help text).
                    break;
                }
            }
            eprintln!(
                "'{diff_bin}' process failed with exit status {code}. Command was: {}",
                format_args!(
                    "{} {} {} {}",
                    diff_path.display(),
                    shell_words::join(diff_cmd),
                    minus_file.display(),
                    plus_file.display()
                )
            );
            config.error_exit_code
        }
    } else {
        code
    }
}

/// Do the user-supplied `diff` args set the unified context?
fn diff_args_set_unified_context<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // This function is applied to `diff` args; not `git diff`.
    for arg in args {
        let arg = arg.as_ref();
        if arg == "-u" || arg == "-U" {
            // diff allows a space after -U (git diff does not)
            return true;
        }
        if (arg.starts_with("-U") || arg.starts_with("-u"))
            && arg.split_at(2).1.parse::<u32>().is_ok()
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod main_tests {
    use std::io::{Cursor, Read, Seek};
    use std::path::PathBuf;

    use super::{diff, diff_args_set_unified_context, Differ};
    use crate::tests::integration_test_utils;

    use rstest::rstest;

    #[rstest]
    #[case(&["-u"], true)]
    #[case(&["-u7"], true)]
    #[case(&["-u77"], true)]
    #[case(&["-ux"], false)]
    #[case(&["-U"], true)]
    #[case(&["-U7"], true)]
    #[case(&["-U77"], true)]
    #[case(&["-Ux"], false)]
    fn test_unified_diff_arg_is_detected_in_diff_args(
        #[case] diff_args: &[&str],
        #[case] expected: bool,
    ) {
        assert_eq!(diff_args_set_unified_context(diff_args), expected)
    }

    #[test]
    #[ignore] // https://github.com/dandavison/delta/pull/546
    fn test_diff_same_empty_file() {
        _do_diff_test("/dev/null", "/dev/null", false);
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_same_non_empty_file() {
        _do_diff_test("/etc/passwd", "/etc/passwd", false);
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_empty_vs_non_empty_file() {
        _do_diff_test("/dev/null", "/etc/passwd", true);
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_two_non_empty_files() {
        _do_diff_test("/etc/group", "/etc/passwd", true);
    }

    fn _do_diff_test(file_a: &str, file_b: &str, expect_diff: bool) {
        for args in [
            vec![],
            vec!["-@''"],
            vec!["-@-u"],
            vec!["-@-U99"],
            vec!["-@-U0"],
        ] {
            let config = integration_test_utils::make_config_from_args(&args);
            let mut writer = Cursor::new(vec![]);
            let exit_code = diff(
                &PathBuf::from(file_a),
                &PathBuf::from(file_b),
                Differ::GitDiff,
                &config,
                &mut writer,
            );
            assert_eq!(exit_code, if expect_diff { 1 } else { 0 });
        }
    }

    fn _read_to_string(cursor: &mut Cursor<Vec<u8>>) -> String {
        let mut s = String::new();
        cursor.rewind().unwrap();
        cursor.read_to_string(&mut s).unwrap();
        s
    }
}
