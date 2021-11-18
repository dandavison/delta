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
    calling_process_cmdline(ProcInfo::new(), blame::guess_git_blame_filename_extension)
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
        let all_args = args.iter().map(|s| s.as_str());

        // See git(1) and git-blame(1). Some arguments separate their parameter with space or '=', e.g.
        // --date 2015 or --date=2015.
        let git_blame_options_with_parameter =
            "-C -c -L --since --ignore-rev --ignore-revs-file --contents --reverse --date";

        let selected_args =
            skip_uninteresting_args(all_args, git_blame_options_with_parameter.split(' '));

        match selected_args.as_slice() {
            [_git, "blame", .., last_arg] => match last_arg.split('.').last() {
                Some(arg) => ProcessArgs::Args(arg.to_string()),
                None => ProcessArgs::ArgError,
            },
            [_git, "blame"] => ProcessArgs::ArgError,
            _ => ProcessArgs::OtherProcess,
        }
    }
} // mod blame

struct ProcInfo {
    info: sysinfo::System,
}
impl ProcInfo {
    fn new() -> Self {
        ProcInfo {
            info: sysinfo::System::new(),
        }
    }
}

trait ProcActions {
    fn cmd(&self) -> &[String];
    fn parent(&self) -> Option<Pid>;
    fn start_time(&self) -> u64;
}

impl<T> ProcActions for T
where
    T: ProcessExt,
{
    fn cmd(&self) -> &[String] {
        ProcessExt::cmd(self)
    }
    fn parent(&self) -> Option<Pid> {
        ProcessExt::parent(self)
    }
    fn start_time(&self) -> u64 {
        ProcessExt::start_time(self)
    }
}

trait ProcessInterface {
    type Out: ProcActions;

    fn my_pid(&self) -> Pid;

    fn process(&self, pid: Pid) -> Option<&Self::Out>;
    fn processes(&self) -> &HashMap<Pid, Self::Out>;

    fn refresh_process(&mut self, pid: Pid) -> bool;
    fn refresh_processes(&mut self);

    fn parent_process(&mut self, pid: Pid) -> Option<&Self::Out> {
        self.refresh_process(pid).then(|| ())?;
        let parent_pid = self.process(pid)?.parent()?;
        self.refresh_process(parent_pid).then(|| ())?;
        self.process(parent_pid)
    }
    fn naive_sibling_process(&mut self, pid: Pid) -> Option<&Self::Out> {
        let sibling_pid = pid - 1;
        self.refresh_process(sibling_pid).then(|| ())?;
        self.process(sibling_pid)
    }
    fn find_sibling_process<F, T>(&mut self, pid: Pid, extract_args: F) -> Option<T>
    where
        F: Fn(&[String]) -> ProcessArgs<T>,
        Self: Sized,
    {
        self.refresh_processes();

        let this_start_time = self.process(pid)?.start_time();

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

        iter_parents(self, pid, &mut collect_parent_pids);

        let process_start_time_difference_less_than_3s = |a, b| (a as i64 - b as i64).abs() < 3;

        let cmdline_of_closest_matching_process = self
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
                                length_of_process_chain =
                                    distance_to_first_common_parent + distance;
                            }
                        }
                    };
                    iter_parents(self, pid, &mut sum_distance);

                    Some((length_of_process_chain, args))
                }
                _ => None,
            })
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, ext)| ext);

        cmdline_of_closest_matching_process
    }
}

impl ProcessInterface for ProcInfo {
    type Out = Process;

    fn my_pid(&self) -> Pid {
        std::process::id() as Pid
    }
    fn refresh_process(&mut self, pid: Pid) -> bool {
        self.info.refresh_process(pid)
    }
    fn process(&self, pid: Pid) -> Option<&Self::Out> {
        self.info.process(pid)
    }
    fn processes(&self) -> &HashMap<Pid, Self::Out> {
        self.info.processes()
    }
    fn refresh_processes(&mut self) {
        self.info.refresh_processes()
    }
}

fn calling_process_cmdline<P, F, T>(mut info: P, extract_args: F) -> Option<T>
where
    P: ProcessInterface,
    F: Fn(&[String]) -> ProcessArgs<T>,
{
    #[cfg(test)]
    {
        if let Some(args) = tests::cfg::WithArgs::get() {
            match extract_args(&args) {
                ProcessArgs::Args(ext) => return Some(ext),
                _ => return None,
            }
        }
    }
    let my_pid = info.my_pid();

    // 1) Try the parent process. If delta is set as the pager in git, then git is the parent process.
    let parent = info.parent_process(my_pid)?;

    match extract_args(parent.cmd()) {
        ProcessArgs::Args(ext) => return Some(ext),
        ProcessArgs::ArgError => return None,

        // 2) The parent process was something else, this can happen if git output is piped into delta, e.g.
        // `git blame foo.txt | delta`. When the shell sets up the pipe it creates the two processes, the pids
        // are usually consecutive, so check if the process with `my_pid - 1` matches.
        ProcessArgs::OtherProcess => {
            let sibling = info.naive_sibling_process(my_pid);
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
    info.find_sibling_process(my_pid, extract_args)
}

// Walk up the process tree, calling `f` with the pid and the distance to `starting_pid`.
// Prerequisite: `info.refresh_processes()` has been called.
fn iter_parents<P, F>(info: &P, starting_pid: Pid, f: F)
where
    P: ProcessInterface,
    F: FnMut(Pid, usize),
{
    fn inner_iter_parents<P, F>(info: &P, pid: Pid, mut f: F, distance: usize)
    where
        P: ProcessInterface,
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

#[cfg(test)]
pub mod tests {
    use super::blame::*;
    use super::*;

    use itertools::Itertools;

    pub mod cfg {
        use std::cell::RefCell;

        #[derive(Debug, PartialEq)]
        enum TlsState<T> {
            Some(T),
            None,
            Invalid,
        }

        thread_local! {
            static FAKE_ARGS: RefCell<TlsState<Vec<String>>> = RefCell::new(TlsState::None);
        }

        pub struct WithArgs {}
        impl WithArgs {
            pub fn new(args: &str) -> Self {
                let string_vec = args.split(' ').map(str::to_owned).collect();
                assert!(
                    FAKE_ARGS.with(|a| a.replace(TlsState::Some(string_vec))) != TlsState::Invalid,
                    "test logic error (in new): wrong WithArgs scope?"
                );
                WithArgs {}
            }
            pub fn get() -> Option<Vec<String>> {
                FAKE_ARGS.with(|a| {
                    let old_value = a.replace_with(|old_value| match old_value {
                        TlsState::Some(_) => TlsState::Invalid,
                        TlsState::None => TlsState::None,
                        TlsState::Invalid => TlsState::Invalid,
                    });

                    match old_value {
                        TlsState::Some(args) => Some(args),
                        TlsState::None => None,
                        TlsState::Invalid => {
                            panic!("test logic error (in get): wrong WithArgs scope?")
                        }
                    }
                })
            }
        }
        impl Drop for WithArgs {
            fn drop(&mut self) {
                // clears an invalid state
                FAKE_ARGS.with(|a| a.replace(TlsState::None));
            }
        }
    }

    #[test]
    fn test_guess_git_blame_filename_extension() {
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
        assert_eq!(
            guess_git_blame_filename_extension(&args),
            ProcessArgs::ArgError
        );

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

    #[derive(Debug, Default)]
    struct FakeProc {
        pid: Pid,
        start_time: u64,
        cmd: Vec<String>,
        ppid: Option<Pid>,
    }
    impl FakeProc {
        fn new(pid: Pid, start_time: u64, cmd: Vec<String>, ppid: Option<Pid>) -> Self {
            FakeProc {
                pid,
                start_time,
                cmd,
                ppid,
            }
        }
    }

    impl ProcActions for FakeProc {
        fn cmd(&self) -> &[String] {
            &self.cmd
        }
        fn parent(&self) -> Option<Pid> {
            self.ppid
        }
        fn start_time(&self) -> u64 {
            self.start_time
        }
    }

    #[derive(Debug, Default)]
    struct MockProcInfo {
        delta_pid: Pid,
        info: HashMap<Pid, FakeProc>,
    }
    impl MockProcInfo {
        fn with(processes: &[(Pid, u64, &str, Option<Pid>)]) -> Self {
            MockProcInfo {
                delta_pid: processes.last().map(|p| p.0).unwrap_or(1),
                info: processes
                    .into_iter()
                    .map(|(pid, start_time, cmd, ppid)| {
                        let cmd_vec = cmd.split(' ').map(str::to_owned).collect();
                        (*pid, FakeProc::new(*pid, *start_time, cmd_vec, *ppid))
                    })
                    .collect(),
            }
        }
    }

    impl ProcessInterface for MockProcInfo {
        type Out = FakeProc;

        fn my_pid(&self) -> Pid {
            self.delta_pid
        }
        fn process(&self, pid: Pid) -> Option<&Self::Out> {
            self.info.get(&pid)
        }
        fn processes(&self) -> &HashMap<Pid, Self::Out> {
            &self.info
        }
        fn refresh_processes(&mut self) {}
        fn refresh_process(&mut self, _pid: Pid) -> bool {
            true
        }
    }

    #[test]
    fn test_process_testing() {
        {
            let _args = cfg::WithArgs::new(&"git blame hello");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), blame::guess_git_blame_filename_extension),
                Some("hello".into())
            );
        }
        {
            let _args = cfg::WithArgs::new(&"git blame world.txt");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), blame::guess_git_blame_filename_extension),
                Some("txt".into())
            );
        }
    }

    #[test]
    #[should_panic]
    fn test_process_testing_assert() {
        {
            let _args = cfg::WithArgs::new(&"git blame do.not.panic");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), blame::guess_git_blame_filename_extension),
                Some("panic".into())
            );

            calling_process_cmdline(ProcInfo::new(), blame::guess_git_blame_filename_extension);
        }
    }

    #[test]
    fn test_process_blame_info_with_parent() {
        let no_processes = MockProcInfo::with(&[]);
        assert_eq!(
            calling_process_cmdline(no_processes, blame::guess_git_blame_filename_extension),
            None
        );

        let parent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git blame hello.txt", Some(2)),
            (4, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(parent, blame::guess_git_blame_filename_extension),
            Some("txt".into())
        );

        let grandparent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git blame src/main.rs", Some(2)),
            (4, 100, "call_delta.sh", Some(3)),
            (5, 100, "delta", Some(4)),
        ]);
        assert_eq!(
            calling_process_cmdline(grandparent, blame::guess_git_blame_filename_extension),
            Some("rs".into())
        );
    }

    #[test]
    fn test_process_calling_cmdline() {
        // Github runs CI tests for arm under qemu where where sysinfo can not find the parent processr.
        if std::env::vars().any(|(key, _)| key == "CROSS_RUNNER" || key == "QEMU_LD_PREFIX") {
            return;
        }

        let mut info = ProcInfo::new();
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
        assert_eq!(calling_process_cmdline(info, find_test), Some(()));

        let nonsense = ppid_distance
            .iter()
            .map(|i| i.to_string())
            .join("Y40ii4RihK6lHiK4BDsGSx");

        let find_nothing = |args: &[String]| find_calling_process(args, &[&nonsense]);
        assert_eq!(calling_process_cmdline(ProcInfo::new(), find_nothing), None);
    }
}
