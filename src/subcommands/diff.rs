use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;

use bytelines::ByteLinesReader;

use crate::config::{self, delta_unreachable};
use crate::delta;

/// Run `git diff` on the files provided on the command line and display the output.
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
    let via_process_substitution =
        |f: &Path| f.starts_with("/proc/self/fd/") || f.starts_with("/dev/fd/");

    let diff_cmd = if via_process_substitution(minus_file) || via_process_substitution(plus_file) {
        ["diff", "-u", "--"].as_slice()
    } else {
        ["git", "diff", "--no-index", "--color", "--"].as_slice()
    };

    let diff_bin = diff_cmd[0];
    let diff_path = match grep_cli::resolve_binary(PathBuf::from(diff_bin)) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to resolve command '{}': {}", diff_bin, err);
            return config.error_exit_code;
        }
    };

    let diff_process = process::Command::new(diff_path)
        .args(&diff_cmd[1..])
        .args(&[minus_file, plus_file])
        .stdout(process::Stdio::piped())
        .spawn();

    if let Err(err) = diff_process {
        eprintln!("Failed to execute the command '{}': {}", diff_bin, err);
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
                eprintln!("{}", error);
                return config.error_exit_code;
            }
        }
    };

    // Return the exit code from the diff process, so that the exit code
    // contract of `delta file_A file_B` is the same as that of `diff file_A
    // file_B` (i.e. 0 if same, 1 if different, 2 if error).
    diff_process
        .wait()
        .unwrap_or_else(|_| {
            delta_unreachable(&format!("'{}' process not running.", diff_bin));
        })
        .code()
        .unwrap_or_else(|| {
            eprintln!("'{}' process terminated without exit status.", diff_bin);
            config.error_exit_code
        })
}

#[cfg(test)]
mod main_tests {
    use std::io::{Cursor, Read, Seek, SeekFrom};
    use std::path::PathBuf;

    use super::diff;
    use crate::tests::integration_test_utils;

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
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![]);
        let exit_code = diff(
            &PathBuf::from(file_a),
            &PathBuf::from(file_b),
            &config,
            &mut writer,
        );
        assert_eq!(exit_code, if expect_diff { 1 } else { 0 });
    }

    fn _read_to_string(cursor: &mut Cursor<Vec<u8>>) -> String {
        let mut s = String::new();
        cursor.seek(SeekFrom::Start(0)).unwrap();
        cursor.read_to_string(&mut s).unwrap();
        s
    }
}
