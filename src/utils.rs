use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, Process, ProcessExt, SystemExt};

#[derive(Debug, PartialEq)]
pub enum GitBlameExtension {
    Some(String),
    None,
    NotGitBlame,
}

pub fn git_blame_filename_extension() -> Option<String> {
    let mut info = sysinfo::System::new();
    let my_pid = std::process::id() as Pid;

    // 1) Try the parent process. If delta is set as the pager in git, then git is the parent process.
    let parent = parent_process(&mut info, my_pid)?;

    match guess_git_blame_filename_extension(parent.cmd()) {
        GitBlameExtension::Some(ext) => return Some(ext),
        GitBlameExtension::None => return None,

        // 2) The parent process was something else, this can happen if git output is piped into delta, e.g.
        // `git blame foo.txt | delta`. When the shell sets up the pipe it creates the two processes, the pids
        // are usually consecutive, so check if the proceess with `my_pid - 1` matches.
        GitBlameExtension::NotGitBlame => {
            let sibling = naive_sibling_process(&mut info, my_pid);
            if let Some(proc) = sibling {
                if let GitBlameExtension::Some(ext) = guess_git_blame_filename_extension(proc.cmd())
                {
                    return Some(ext);
                }
            }
            // else try the fallback
        }
    }

    /*
        3) Neither parent nor direct sibling were a match.
        The most likely case is that the input program of the pipe wrote all its data and exited before delta
        started, so no file extension can be retrieved. Same if the data was piped from an input file.

        There might also be intermediary scripts in between or piped input with randomized pids, so check all
        processes for the closest `git blame` in the process tree.

    100 /usr/bin/some-terminal-emulator
    124  \_ -shell
    301  |   \_ /usr/bin/git blame src/main.rs
    302  |       \_ wraps_delta.sh
    303  |           \_ delta
    304  |               \_ less --RAW-CONTROL-CHARS --quit-if-one-screen
    125  \_ -shell
    800  |   \_ /usr/bin/git blame src/main.rs
    400  |       \_ delta
    200  |           \_ less --RAW-CONTROL-CHARS --quit-if-one-screen
    126  \_ -shell
    501  |   \_ /bin/sh /wrapper/for/git blame src/main.rs
    555  |   |   \_ /usr/bin/git blame src/main.rs
    502  |   \_ delta
    567  |       \_ less --RAW-CONTROL-CHARS --quit-if-one-screen

    */
    find_sibling_process(&mut info, my_pid)
}

// Skip all arguments starting with '-' from `args_it`. Also skip all arguments listed in
// `skip_this_plus_parameter` plus their respective next argument.
// Keep all arguments once a '--' is encountered.
// (Note that some an argument work with and without '=', e.g. '--foo' 'bar' and '--foo=bar')
fn skip_uninteresting_args<'a, 'b, ArgsI, SkipI>(
    mut args_it: ArgsI,
    skip_this_plus_parameter: SkipI,
) -> Vec<&'a str>
where
    ArgsI: Iterator<Item = &'a str>,
    SkipI: Iterator<Item = &'b str>,
{
    let arg_follows_space: HashSet<&'b str> = skip_this_plus_parameter.into_iter().collect();

    let mut result = Vec::new();
    loop {
        match args_it.next() {
            None => break result,
            Some("--") => {
                result.extend(args_it);
                break result;
            }
            Some(arg) if arg_follows_space.contains(arg) => {
                let _skip_parameter = args_it.next();
            }
            Some(arg) if !arg.starts_with('-') => {
                result.push(arg);
            }
            Some(_) => { /* skip: --these -and --also=this */ }
        }
    }
}

fn guess_git_blame_filename_extension(args: &[String]) -> GitBlameExtension {
    {
        let mut it = args.iter();
        match (it.next(), it.next()) {
            // git blame or git -C/-c etc. and then (maybe) blame
            (Some(git), Some(blame))
                if git.contains("git") && (blame == "blame" || blame.starts_with('-')) => {}
            _ => return GitBlameExtension::NotGitBlame,
        }
    }

    let args = args.iter().skip(2).map(|s| s.as_str());

    // See git(1) and git-blame(1). Some arguments separate their parameter with space or '=', e.g.
    // --date=2015 or --date 2015.
    let git_blame_options_with_parameter =
        "-C -c -L --since --ignore-rev --ignore-revs-file --contents --reverse --date";

    match skip_uninteresting_args(args, git_blame_options_with_parameter.split(' '))
        .last()
        .and_then(|&s| s.split('.').last())
        .map(str::to_owned)
    {
        Some(ext) => GitBlameExtension::Some(ext),
        None => GitBlameExtension::None,
    }
}

fn parent_process(info: &mut sysinfo::System, my_pid: Pid) -> Option<&Process> {
    info.refresh_process(my_pid).then(|| ())?;

    let parent_pid = info.process(my_pid)?.parent()?;
    info.refresh_process(parent_pid).then(|| ())?;
    info.process(parent_pid)
}

fn naive_sibling_process(info: &mut sysinfo::System, my_pid: Pid) -> Option<&Process> {
    let sibling_pid = my_pid - 1;
    info.refresh_process(sibling_pid).then(|| ())?;
    info.process(sibling_pid)
}

fn iter_parents<F>(info: &sysinfo::System, pid: Pid, distance: usize, mut f: F)
where
    F: FnMut(Pid, usize),
{
    if let Some(proc) = info.process(pid) {
        if let Some(pid) = proc.parent() {
            f(pid, distance);
            iter_parents(info, pid, distance + 1, f)
        }
    }
}

fn find_sibling_process(info: &mut sysinfo::System, my_pid: Pid) -> Option<String> {
    info.refresh_processes();

    let this_start_time = info.process(my_pid)?.start_time();

    /*

    $ start_blame_of.sh src/main.rs | delta

    \_ /usr/bin/some-terminal-emulator
    |   \_ common_git_and_delta_ancestor
    |       \_ /bin/sh /opt/git/start_blame_of.sh src/main.rs
    |       |   \_ /bin/sh /opt/some/wrapper git blame src/main.rs
    |       |       \_ /usr/bin/git blame src/main.rs
    |       \_ /bin/sh /opt/some/wrapper delta
    |           \_ delta

    Walk up the process tree of delta and of every matching other process, counting the steps
    along the way.
    Find the common ancestor processes, calculate the distance, and select the one with the shortest.

    */

    let mut pid_distances = HashMap::<Pid, usize>::new();
    let mut collect_parent_pids = |pid: Pid, distance| {
        pid_distances.insert(pid, distance);
    };

    iter_parents(info, my_pid, 1, &mut collect_parent_pids);

    let process_start_time_difference_less_than_3s = |a, b| (a as i64 - b as i64).abs() < 3;

    let closest_git_blame_extension = info
        .processes()
        .iter()
        .filter(|(_, proc)| {
            process_start_time_difference_less_than_3s(this_start_time, proc.start_time())
        })
        .filter_map(
            |(pid, proc)| match guess_git_blame_filename_extension(proc.cmd()) {
                GitBlameExtension::Some(args) => {
                    let mut length_of_process_chain = usize::MAX;

                    let mut sum_distance = |pid: Pid, distance: usize| {
                        if length_of_process_chain == usize::MAX {
                            if let Some(distance_to_first_common_parent) = pid_distances.get(&pid) {
                                length_of_process_chain =
                                    distance_to_first_common_parent + distance;
                            }
                        }
                    };
                    iter_parents(info, *pid, 1, &mut sum_distance);

                    Some((length_of_process_chain, args))
                }
                _ => None,
            },
        )
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, ext)| ext);

    closest_git_blame_extension
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_filename_extension_from_args() {
        use GitBlameExtension::None;
        use GitBlameExtension::Some;

        fn make_string_vec(args: &[&str]) -> Vec<String> {
            args.iter().map(|&x| x.to_owned()).collect::<Vec<String>>()
        }
        let args = make_string_vec(&["git", "blame", "hello", "world.txt"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Some("txt".into())
        );

        let args = make_string_vec(&[
            "git",
            "blame",
            "-s",
            "-f",
            "hello.txt",
            "--date=2015",
            "--date",
            "now",
        ]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Some("txt".into())
        );

        let args = make_string_vec(&["git", "blame", "-s", "-f", "--", "hello.txt"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Some("txt".into())
        );

        let args = make_string_vec(&["git", "blame", "--", "--not.an.argument"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Some("argument".into())
        );

        let args = make_string_vec(&["foo", "bar", "-a", "--123", "not.git"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            GitBlameExtension::NotGitBlame
        );

        let args = make_string_vec(&["git", "blame", "--help.txt"]);
        assert_eq!(guess_git_blame_filename_extension(&args), None);

        let args = make_string_vec(&["git", "-c", "a=b", "blame", "main.rs"]);
        assert_eq!(guess_git_blame_filename_extension(&args), Some("rs".into()));

        let args = make_string_vec(&["git", "blame", "README"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Some("README".into())
        );

        let args = make_string_vec(&["git", "blame", ""]);
        assert_eq!(guess_git_blame_filename_extension(&args), Some("".into()));
    }

    #[test]
    fn test_process_parent_cmd_args() {
        // Github runs CI tests for arm under qemu where where sysinfo can not find the parent pid.
        if std::env::vars().any(|(key, _)| key == "CROSS_RUNNER" || key == "QEMU_LD_PREFIX") {
            return;
        }

        let mut info = sysinfo::System::new();
        let my_pid = std::process::id() as Pid;

        let parent = parent_process(&mut info, my_pid);

        assert!(parent.is_some());

        // Tests that caller is something like "cargo test"
        assert!(parent.unwrap().cmd().iter().any(|a| a == "test"));
    }
}
