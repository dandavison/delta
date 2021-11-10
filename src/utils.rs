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

pub fn parent_filename_extension() -> Option<String> {
    process_parent_cmd_args(guess_filename_extension_from_args)
}

fn process_parent_cmd_args<F>(f: F) -> Option<String>
where
    F: Fn(&[String]) -> Option<String>,
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
}
