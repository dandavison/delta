use std::io::{BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;

use bytelines::ByteLinesReader;

use crate::config::{self, delta_unreachable};
use crate::delta;

/// Run `git diff` on the files provided on the command line and display the output. Fall back to
/// `diff` if the supplied "files" use process substitution.
pub fn diff(
    minus_file: &Path,
    plus_file: &Path,
    config: &config::Config,
    writer: &mut dyn Write,
) -> i32 {
    use std::io::BufReader;

    // When called as `delta <(echo foo) <(echo bar)`, then git as of version 2.34 just prints the
    // diff of the filenames which were created by the process substitution and does not read their
    // content, so fall back to plain `diff` which simply opens the given input as files.
    // This fallback ignores git settings, but is better than nothing.

    let diff_args = match shell_words::split(&config.diff_args) {
        Ok(words) => words,
        Err(err) => {
            eprintln!("Failed to parse diff args: {}: {err}", config.diff_args);
            return config.error_exit_code;
        }
    };

    // https://stackoverflow.com/questions/22706714/why-does-git-diff-not-work-with-process-substitution
    // TODO: git >= 2.42 supports process substitution
    let use_git_diff = true;
    let mut diff_cmd = if use_git_diff {
        vec!["git", "diff", "--no-index", "--color"]
    } else if diff_args_set_unified_context(&diff_args) {
        vec!["diff"]
    } else {
        vec!["diff", "-U3"]
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

    // Return the exit code from the diff process, so that the exit code
    // contract of `delta file_A file_B` is the same as that of `diff file_A
    // file_B` (i.e. 0 if same, 1 if different, >= 2 if error).
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
        for line in BufReader::new(diff_process.stderr.unwrap()).lines() {
            eprintln!("{}", line.unwrap_or("<delta: could not parse line>".into()));
            if code == 129 && use_git_diff {
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

    use super::{diff, diff_args_set_unified_context};
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
