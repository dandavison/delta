use std::io::{BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;

use bytelines::ByteLinesReader;

use crate::config::{self, delta_unreachable};
use crate::delta;
use crate::utils::git::retrieve_git_version;

#[derive(Debug, PartialEq)]
enum Differ {
    GitDiff,
    Diff,
}

/// Run `git diff` on the files provided on the command line and display the output. Fall back to
/// `diff` if the supplied "files" use process substitution.
pub fn diff(
    minus_file: &Path,
    plus_file: &Path,
    config: &config::Config,
    writer: &mut dyn Write,
) -> i32 {
    use std::io::BufReader;

    let mut diff_args = match shell_words::split(config.diff_args.trim()) {
        Ok(words) => words,
        Err(err) => {
            eprintln!("Failed to parse diff args: {}: {err}", config.diff_args);
            return config.error_exit_code;
        }
    };
    // Permit e.g. -@U1
    if diff_args
        .first()
        .map(|arg| !arg.is_empty() && !arg.starts_with('-'))
        .unwrap_or(false)
    {
        diff_args[0] = format!("-{}", diff_args[0])
    }

    let via_process_substitution =
        |f: &Path| f.starts_with("/proc/self/fd/") || f.starts_with("/dev/fd/");

    // https://stackoverflow.com/questions/22706714/why-does-git-diff-not-work-with-process-substitution
    // git <2.42 does not support process substitution
    let (differ, mut diff_cmd) = match retrieve_git_version() {
        Some(version)
            if version >= (2, 42)
                || !(via_process_substitution(minus_file)
                    || via_process_substitution(plus_file)) =>
        {
            (
                Differ::GitDiff,
                vec!["git", "diff", "--no-index", "--color"],
            )
        }
        _ => (
            Differ::Diff,
            if diff_args_set_unified_context(&diff_args) {
                vec!["diff"]
            } else {
                vec!["diff", "-U3"]
            },
        ),
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
        for line in BufReader::new(diff_process.stderr.unwrap()).lines() {
            eprintln!("{}", line.unwrap_or("<delta: could not parse line>".into()));
            if code == 129 && differ == Differ::GitDiff {
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
    use std::io::Cursor;
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

    enum ExpectDiff {
        Yes,
        No,
    }

    #[cfg(not(target_os = "windows"))]
    #[rstest]
    // #[case("/dev/null", "/dev/null", ExpectDiff::No)] https://github.com/dandavison/delta/pull/546#issuecomment-835852373
    #[case("/etc/group", "/etc/passwd", ExpectDiff::Yes)]
    #[case("/dev/null", "/etc/passwd", ExpectDiff::Yes)]
    #[case("/etc/passwd", "/etc/passwd", ExpectDiff::No)]
    fn test_diff_real_files(
        #[case] file_a: &str,
        #[case] file_b: &str,
        #[case] expect_diff: ExpectDiff,
        #[values(vec![], vec!["-@''"], vec!["-@-u"], vec!["-@-U99"], vec!["-@-U0"])] args: Vec<
            &str,
        >,
    ) {
        let config = integration_test_utils::make_config_from_args(&args);
        let mut writer = Cursor::new(vec![]);
        let exit_code = diff(
            &PathBuf::from(file_a),
            &PathBuf::from(file_b),
            &config,
            &mut writer,
        );
        assert_eq!(
            exit_code,
            match expect_diff {
                ExpectDiff::Yes => 1,
                ExpectDiff::No => 0,
            }
        );
    }
}
