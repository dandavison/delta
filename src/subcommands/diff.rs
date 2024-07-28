use std::ffi::OsString;
use std::path::Path;

use crate::cli::Call::SubCommand;
use crate::cli::Opt;
use crate::config::{self};

/// Run `git diff` on the files provided on the command line and display the output.
pub fn diff(minus_file: &Path, plus_file: &Path, _config: &config::Config) -> Vec<OsString> {
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

    // TODO, check if paths exist
    if minus_file == Path::new("rg") || minus_file == Path::new("show") {
        let args = std::env::args_os().collect::<Vec<_>>();
        if let SubCommand(_, subcommand) = Opt::try_subcmds(
            &args,
            clap::Error::new(clap::error::ErrorKind::InvalidSubcommand),
        ) {
            return subcommand;
        }
        unreachable!()
    }

    let mut result: Vec<_> = diff_cmd.iter().map(|&arg| arg.into()).collect();
    result.push(minus_file.into());
    result.push(plus_file.into());

    result
}

#[cfg(test)]
mod main_tests {
    use std::io::{Cursor, Read, Seek};
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
        // let config = integration_test_utils::make_config_from_args(&[]);
        // let mut writer = Cursor::new(vec![]);
        // let exit_code = diff(
        //     &PathBuf::from(file_a),
        //     &PathBuf::from(file_b),
        //     &config,
        //     &mut writer,
        // );
        // assert_eq!(exit_code, if expect_diff { 1 } else { 0 });
    }

    fn _read_to_string(cursor: &mut Cursor<Vec<u8>>) -> String {
        let mut s = String::new();
        cursor.rewind().unwrap();
        cursor.read_to_string(&mut s).unwrap();
        s
    }
}
