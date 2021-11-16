use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, Process, ProcessExt, SystemExt};

// Return value of `extract_args(args: &[String]) -> ProcessArgs<T>` function which is
// passed to `calling_process_cmdline()`.
#[derive(Debug, PartialEq)]
pub enum ProcessArgs<T> {
    // A result has been successfully extracted from args.
    Args(T),
    // The extraction has failed.
    ArgError,
    // The process does not match, others may be inspected.
    OtherProcess,
}

pub fn git_blame_filename_extension() -> Option<String> {
    calling_process_cmdline(blame::guess_git_blame_filename_extension)
}

mod blame {
    use super::*;

    // Skip all arguments starting with '-' from `args_it`. Also skip all arguments listed in
    // `skip_this_plus_parameter` plus their respective next argument.
    // Keep all arguments once a '--' is encountered.
    // (Note that some arguments work with and without '=': '--foo' 'bar' / '--foo=bar')
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

    pub fn guess_git_blame_filename_extension(args: &[String]) -> ProcessArgs<String> {
        {
            let mut it = args.iter();
            match (it.next(), it.next()) {
                // git blame or git -C/-c etc. and then (maybe) blame
                (Some(git), Some(blame))
                    if git.contains("git") && (blame == "blame" || blame.starts_with('-')) => {}
                _ => return ProcessArgs::OtherProcess,
            }
        }

        let args = args.iter().skip(2).map(|s| s.as_str());

        // See git(1) and git-blame(1). Some arguments separate their parameter with space or '=', e.g.
        // --date 2015 or --date=2015.
        let git_blame_options_with_parameter =
            "-C -c -L --since --ignore-rev --ignore-revs-file --contents --reverse --date";

        match skip_uninteresting_args(args, git_blame_options_with_parameter.split(' '))
            .last()
            .and_then(|&s| s.split('.').last())
            .map(str::to_owned)
        {
            Some(ext) => ProcessArgs::Args(ext),
            None => ProcessArgs::ArgError,
        }
    }
} // mod blame

fn calling_process_cmdline<F, T>(extract_args: F) -> Option<T>
where
    F: Fn(&[String]) -> ProcessArgs<T>,
{
    let mut info = sysinfo::System::new();
    let my_pid = std::process::id() as Pid;

    // 1) Try the parent process. If delta is set as the pager in git, then git is the parent process.
    let parent = parent_process(&mut info, my_pid)?;

    match extract_args(parent.cmd()) {
        ProcessArgs::Args(ext) => return Some(ext),
        ProcessArgs::ArgError => return None,

        // 2) The parent process was something else, this can happen if git output is piped into delta, e.g.
        // `git blame foo.txt | delta`. When the shell sets up the pipe it creates the two processes, the pids
        // are usually consecutive, so check if the process with `my_pid - 1` matches.
        ProcessArgs::OtherProcess => {
            let sibling = naive_sibling_process(&mut info, my_pid);
            if let Some(proc) = sibling {
                if let ProcessArgs::Args(ext) = extract_args(proc.cmd()) {
                    return Some(ext);
                }
            }
            // else try the fallback
        }
    }

    /*
    3) Neither parent nor direct sibling were a match.
    The most likely case is that the input program of the pipe wrote all its data and exited before delta
    started, so no command line can be parsed. Same if the data was piped from an input file.

    There might also be intermediary scripts in between or piped input with a gap in pids or (rarely)
    randomized pids, so check all processes for the closest match in the process tree.

    100 /usr/bin/some-terminal-emulator
    124  \_ -shell
    301  |   \_ /usr/bin/git blame src/main.rs
    302  |       \_ wraps_delta.sh
    303  |           \_ delta
    304  |               \_ less --RAW-CONTROL-CHARS --quit-if-one-screen
    125  \_ -shell
    800  |   \_ /usr/bin/git blame src/main.rs
    200  |   \_ delta
    400  |       \_ less --RAW-CONTROL-CHARS --quit-if-one-screen
    126  \_ -shell
    501  |   \_ /bin/sh /wrapper/for/git blame src/main.rs
    555  |   |   \_ /usr/bin/git blame src/main.rs
    502  |   \_ delta
    567  |       \_ less --RAW-CONTROL-CHARS --quit-if-one-screen

    */
    find_sibling_process(&mut info, my_pid, extract_args)
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

// Walk up the process tree, calling `f` with the pid and the distance to `starting_pid`.
// Prerequisite: `info.refresh_processes()` has been called.
fn iter_parents<F>(info: &sysinfo::System, starting_pid: Pid, f: F)
where
    F: FnMut(Pid, usize),
{
    fn inner_iter_parents<F>(info: &sysinfo::System, pid: Pid, mut f: F, distance: usize)
    where
        F: FnMut(Pid, usize),
    {
        if let Some(proc) = info.process(pid) {
            if let Some(pid) = proc.parent() {
                f(pid, distance);
                inner_iter_parents(info, pid, f, distance + 1)
            }
        }
    }
    inner_iter_parents(info, starting_pid, f, 1)
}

fn find_sibling_process<F, T>(info: &mut sysinfo::System, my_pid: Pid, extract_args: F) -> Option<T>
where
    F: Fn(&[String]) -> ProcessArgs<T>,
{
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
    let mut collect_parent_pids = |pid, distance| {
        pid_distances.insert(pid, distance);
    };

    iter_parents(info, my_pid, &mut collect_parent_pids);

    let process_start_time_difference_less_than_3s = |a, b| (a as i64 - b as i64).abs() < 3;

    let cmdline_of_closest_matching_process = info
        .processes()
        .iter()
        .filter(|(_, proc)| {
            process_start_time_difference_less_than_3s(this_start_time, proc.start_time())
        })
        .filter_map(|(&pid, proc)| match extract_args(proc.cmd()) {
            ProcessArgs::Args(args) => {
                let mut length_of_process_chain = usize::MAX;

                let mut sum_distance = |pid, distance| {
                    if length_of_process_chain == usize::MAX {
                        if let Some(distance_to_first_common_parent) = pid_distances.get(&pid) {
                            length_of_process_chain = distance_to_first_common_parent + distance;
                        }
                    }
                };
                iter_parents(info, pid, &mut sum_distance);

                Some((length_of_process_chain, args))
            }
            _ => None,
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, ext)| ext);

    cmdline_of_closest_matching_process
}

#[cfg(test)]
mod tests {
    use super::blame::*;
    use super::*;

    use itertools::Itertools;

    #[test]
    fn test_guess_git_blame_filename_extension() {
        use ProcessArgs::ArgError;
        use ProcessArgs::Args;

        fn make_string_vec(args: &[&str]) -> Vec<String> {
            args.iter().map(|&x| x.to_owned()).collect::<Vec<String>>()
        }
        let args = make_string_vec(&["git", "blame", "hello", "world.txt"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Args("txt".into())
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
            Args("txt".into())
        );

        let args = make_string_vec(&["git", "blame", "-s", "-f", "--", "hello.txt"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Args("txt".into())
        );

        let args = make_string_vec(&["git", "blame", "--", "--not.an.argument"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Args("argument".into())
        );

        let args = make_string_vec(&["foo", "bar", "-a", "--123", "not.git"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            ProcessArgs::OtherProcess
        );

        let args = make_string_vec(&["git", "blame", "--help.txt"]);
        assert_eq!(guess_git_blame_filename_extension(&args), ArgError);

        let args = make_string_vec(&["git", "-c", "a=b", "blame", "main.rs"]);
        assert_eq!(guess_git_blame_filename_extension(&args), Args("rs".into()));

        let args = make_string_vec(&["git", "blame", "README"]);
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            Args("README".into())
        );

        let args = make_string_vec(&["git", "blame", ""]);
        assert_eq!(guess_git_blame_filename_extension(&args), Args("".into()));
    }

    #[test]
    fn test_calling_process_cmdline() {
        // Github runs CI tests for arm under qemu where where sysinfo can not find the parent processr.
        if std::env::vars().any(|(key, _)| key == "CROSS_RUNNER" || key == "QEMU_LD_PREFIX") {
            return;
        }

        let mut info = sysinfo::System::new();
        info.refresh_processes();
        let mut ppid_distance = Vec::new();

        iter_parents(&info, std::process::id() as Pid, |pid, distance| {
            ppid_distance.push(pid as i32);
            ppid_distance.push(distance as i32)
        });

        assert!(ppid_distance[1] == 1);

        fn find_calling_process(args: &[String], want: &[&str]) -> ProcessArgs<()> {
            if args.iter().any(|have| want.iter().any(|want| want == have)) {
                ProcessArgs::Args(())
            } else {
                ProcessArgs::ArgError
            }
        }

        // Tests that caller is something like "cargo test" or "tarpaulin"
        let find_test = |args: &[String]| find_calling_process(args, &["test", "tarpaulin"]);
        assert_eq!(calling_process_cmdline(find_test), Some(()));

        let nonsense = ppid_distance
            .iter()
            .map(|i| i.to_string())
            .join("Y40ii4RihK6lHiK4BDsGSx");

        let find_nothing = |args: &[String]| find_calling_process(args, &[&nonsense]);
        assert_eq!(calling_process_cmdline(find_nothing), None);
    }
}
