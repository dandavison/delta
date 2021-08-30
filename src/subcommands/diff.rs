use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process;

use bytelines::ByteLinesReader;

use crate::config::{self, delta_unreachable};
use crate::delta;

/// Run `git diff` on the files provided on the command line and display the output.
pub fn diff(
    minus_file: Option<&PathBuf>,
    plus_file: Option<&PathBuf>,
    config: &config::Config,
    writer: &mut dyn Write,
) -> i32 {
    use std::io::BufReader;
    if minus_file.is_none() || plus_file.is_none() {
        eprintln!(
            "\
The main way to use delta is to configure it as the pager for git: \
see https://github.com/dandavison/delta#configuration. \
You can also use delta to diff two files: `delta file_A file_B`."
        );
        return config.error_exit_code;
    }
    let minus_file = minus_file.unwrap();
    let plus_file = plus_file.unwrap();

    let diff_command = "git";
    let diff_command_path = match grep_cli::resolve_binary(PathBuf::from(diff_command)) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to resolve command '{}': {}", diff_command, err);
            return config.error_exit_code;
        }
    };

    let diff_process = process::Command::new(diff_command_path)
        .args(&["diff", "--no-index"])
        .args(&[minus_file, plus_file])
        .stdout(process::Stdio::piped())
        .spawn();

    if let Err(err) = diff_process {
        eprintln!("Failed to execute the command '{}': {}", diff_command, err);
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

    // Return the exit code from the `git diff` processl, so that the exit code
    // contract of `delta file_A file_B` is the same as that of `diff file_A
    // file_B` (i.e. 0 if same, 1 if different, 2 if error).
    diff_process
        .wait()
        .unwrap_or_else(|_| {
            delta_unreachable(&format!("'{}' process not running.", diff_command));
        })
        .code()
        .unwrap_or_else(|| {
            eprintln!("'{}' process terminated without exit status.", diff_command);
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
            Some(&PathBuf::from(file_a)),
            Some(&PathBuf::from(file_b)),
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
