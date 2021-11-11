use std::collections::HashSet;

use sysinfo::{Pid, ProcessExt, SystemExt};

#[cfg(test)]
const SKIP_ARGS: usize = 1;

#[cfg(not(test))]
const SKIP_ARGS: usize = 2;

fn guess_filename_extension_from_args(args: &[String]) -> Option<String> {
    let mut saw_dash_dash = false;
    // skip "git blame" (but during testing only "cargo", not "test" as well)
    for x in args.iter().skip(SKIP_ARGS) {
        if x == "--" {
            saw_dash_dash = true;
            continue;
        }
        if saw_dash_dash || !x.starts_with('-') {
            return x.split('.').last().map(str::to_owned);
        }
    }

    None
}

// Given `command --aa val -bc -d val e f` return
// ({"--aa"}, {"-b", "-c", "-d"})
pub fn get_command_options(args: &[String]) -> Option<(HashSet<String>, HashSet<String>)> {
    let mut longs = HashSet::new();
    let mut shorts = HashSet::new();

    for s in args.iter() {
        if s == "--" {
            break;
        } else if s.starts_with("--") {
            longs.insert(s.to_owned());
        } else if let Some(suffix) = s.strip_prefix('-') {
            shorts.extend(suffix.chars().map(|c| format!("-{}", c)));
        }
    }

    Some((longs, shorts))
}

pub fn parent_filename_extension() -> Option<String> {
    process_parent_cmd_args(guess_filename_extension_from_args)
}

pub fn parent_command_options() -> Option<(HashSet<String>, HashSet<String>)> {
    process_parent_cmd_args(get_command_options)
}

fn process_parent_cmd_args<F, T>(f: F) -> Option<T>
where
    F: Fn(&[String]) -> Option<T>,
{
    let mut info = sysinfo::System::new();

    let my_pid = std::process::id() as Pid;
    info.refresh_process(my_pid).then(|| ())?;

    let parent_pid = info.process(my_pid)?.parent()?;
    info.refresh_process(parent_pid).then(|| ())?;
    let parent_process = info.process(parent_pid)?;

    f(parent_process.cmd())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_filename_extension_from_args() {
        fn make_string_vec(args: &[&str]) -> Vec<String> {
            args.iter().map(|&x| x.to_owned()).collect::<Vec<String>>()
        }
        let args = make_string_vec(&["blame", "hello.txt"]);
        assert_eq!(
            guess_filename_extension_from_args(&args),
            Some("txt".into())
        );

        let args = make_string_vec(&["blame", "-s", "-f", "--", "hello.txt"]);
        assert_eq!(
            guess_filename_extension_from_args(&args),
            Some("txt".into())
        );

        let args = make_string_vec(&["blame", "--", "--not.an.argument"]);
        assert_eq!(
            guess_filename_extension_from_args(&args),
            Some("argument".into())
        );

        let args = make_string_vec(&["blame", "--help.txt"]);
        assert_eq!(guess_filename_extension_from_args(&args), None);

        let args = make_string_vec(&["blame", "README"]);
        assert_eq!(
            guess_filename_extension_from_args(&args),
            Some("README".into())
        );
    }

    #[test]
    fn test_process_parent_cmd_args() {
        let _ = parent_filename_extension();
        let parent_arg0 = process_parent_cmd_args(|args| {
            // tests that caller is something like "cargo test"
            assert!(args.iter().any(|a| a == "test"));
            Some(args[0].clone())
        });
        assert!(parent_arg0.is_some());
    }

    #[test]
    fn test_get_command_options() {
        fn make_string_vec(args: &[&str]) -> Vec<String> {
            args.iter().map(|&x| x.to_owned()).collect::<Vec<String>>()
        }
        fn make_hash_sets(arg1: &[&str], arg2: &[&str]) -> (HashSet<String>, HashSet<String>) {
            let f = |strs: &[&str]| strs.iter().map(|&s| s.to_owned()).collect();
            (f(arg1), f(arg2))
        }

        let args = make_string_vec(&["grep", "hello.txt"]);
        assert_eq!(get_command_options(&args), Some(make_hash_sets(&[], &[])));

        let args = make_string_vec(&["grep", "--", "--not.an.argument"]);
        assert_eq!(get_command_options(&args), Some(make_hash_sets(&[], &[])));

        let args = make_string_vec(&[
            "grep",
            "-ab",
            "--function-context",
            "-n",
            "--show-function",
            "-W",
            "--",
            "hello.txt",
        ]);
        assert_eq!(
            get_command_options(&args),
            Some(make_hash_sets(
                &["--function-context", "--show-function"],
                &["-a", "-b", "-n", "-W"]
            ))
        );

        let args = make_string_vec(&[
            "grep",
            "val",
            "-ab",
            "val",
            "--function-context",
            "val",
            "-n",
            "val",
            "--show-function",
            "val",
            "-W",
            "val",
            "--",
            "hello.txt",
        ]);
        assert_eq!(
            get_command_options(&args),
            Some(make_hash_sets(
                &["--function-context", "--show-function"],
                &["-a", "-b", "-n", "-W"]
            ))
        );
    }
}
