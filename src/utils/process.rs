use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use lazy_static::lazy_static;
use sysinfo::{Pid, PidExt, Process, ProcessExt, ProcessRefreshKind, SystemExt};

pub type DeltaPid = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallingProcess {
    GitDiff(CommandLine),
    GitShow(CommandLine, Option<String>), // element 2 is file extension
    GitLog(CommandLine),
    GitReflog(CommandLine),
    GitGrep(CommandLine),
    OtherGrep, // rg, grep, ag, ack, etc
    None,      // no matching process could be found
    Pending,   // calling process is currently being determined
}
// TODO: Git blame is currently handled differently

impl CallingProcess {
    pub fn paths_in_input_are_relative_to_cwd(&self) -> bool {
        match self {
            CallingProcess::GitDiff(cmd) if cmd.long_options.contains("--relative") => true,
            CallingProcess::GitShow(cmd, _) if cmd.long_options.contains("--relative") => true,
            CallingProcess::GitLog(cmd) if cmd.long_options.contains("--relative") => true,
            CallingProcess::GitGrep(_) | CallingProcess::OtherGrep => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandLine {
    pub long_options: HashSet<String>,
    pub short_options: HashSet<String>,
    last_arg: Option<String>,
}

lazy_static! {
    static ref CALLER: Arc<(Mutex<CallingProcess>, Condvar)> =
        Arc::new((Mutex::new(CallingProcess::Pending), Condvar::new()));
}

pub fn start_determining_calling_process_in_thread() {
    // The handle is neither kept nor returned nor joined but dropped, so the main
    // thread can exit early if it does not need to know its parent process.
    std::thread::Builder::new()
        .name("find_calling_process".into())
        .spawn(move || {
            let calling_process = determine_calling_process();

            let (caller_mutex, determine_done) = &**CALLER;

            let mut caller = caller_mutex.lock().unwrap();
            *caller = calling_process;
            determine_done.notify_all();
        })
        .unwrap();
}

#[cfg(not(test))]
pub fn calling_process() -> MutexGuard<'static, CallingProcess> {
    let (caller_mutex, determine_done) = &**CALLER;

    determine_done
        .wait_while(caller_mutex.lock().unwrap(), |caller| {
            *caller == CallingProcess::Pending
        })
        .unwrap()
}

// The return value is duck-typed to work in place of a MutexGuard when testing.
#[cfg(test)]
pub fn calling_process() -> Box<CallingProcess> {
    type _UnusedImport = MutexGuard<'static, i8>;

    if crate::utils::process::tests::FakeParentArgs::are_set() {
        // If the (thread-local) FakeParentArgs are set, then the following command returns
        // these, so the cached global real ones can not be used.
        Box::new(determine_calling_process())
    } else {
        let (caller_mutex, _) = &**CALLER;

        let mut caller = caller_mutex.lock().unwrap();
        if *caller == CallingProcess::Pending {
            *caller = determine_calling_process();
        }

        Box::new(caller.clone())
    }
}

fn determine_calling_process() -> CallingProcess {
    calling_process_cmdline(ProcInfo::new(), describe_calling_process)
        .unwrap_or(CallingProcess::None)
}

// Return value of `extract_args(args: &[String]) -> ProcessArgs<T>` function which is
// passed to `calling_process_cmdline()`.
#[derive(Debug, PartialEq, Eq)]
pub enum ProcessArgs<T> {
    // A result has been successfully extracted from args.
    Args(T),
    // The extraction has failed.
    ArgError,
    // The process does not match, others may be inspected.
    OtherProcess,
}

pub fn git_blame_filename_extension() -> Option<String> {
    calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension)
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
        [git, "blame", .., last_arg] if is_git_binary(git) => match last_arg.split('.').last() {
            Some(arg) => ProcessArgs::Args(arg.to_string()),
            None => ProcessArgs::ArgError,
        },
        [git, "blame"] if is_git_binary(git) => ProcessArgs::ArgError,
        _ => ProcessArgs::OtherProcess,
    }
}

pub fn describe_calling_process(args: &[String]) -> ProcessArgs<CallingProcess> {
    let mut args = args.iter().map(|s| s.as_str());

    fn is_any_of<'a, I>(cmd: Option<&str>, others: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        cmd.map(|cmd| others.into_iter().any(|o| o.eq_ignore_ascii_case(cmd)))
            .unwrap_or(false)
    }

    match args.next() {
        Some(command) => match Path::new(command).file_stem() {
            Some(s) if s.to_str().map(is_git_binary).unwrap_or(false) => {
                let mut args = args.skip_while(|s| {
                    *s != "diff" && *s != "show" && *s != "log" && *s != "reflog" && *s != "grep"
                });
                match args.next() {
                    Some("diff") => {
                        ProcessArgs::Args(CallingProcess::GitDiff(parse_command_line(args)))
                    }
                    Some("show") => {
                        let command_line = parse_command_line(args);
                        let extension = if let Some(last_arg) = &command_line.last_arg {
                            match last_arg.split_once(':') {
                                Some((_, suffix)) => {
                                    suffix.split('.').last().map(|s| s.to_string())
                                }
                                None => None,
                            }
                        } else {
                            None
                        };
                        ProcessArgs::Args(CallingProcess::GitShow(command_line, extension))
                    }
                    Some("log") => {
                        ProcessArgs::Args(CallingProcess::GitLog(parse_command_line(args)))
                    }
                    Some("reflog") => {
                        ProcessArgs::Args(CallingProcess::GitReflog(parse_command_line(args)))
                    }
                    Some("grep") => {
                        ProcessArgs::Args(CallingProcess::GitGrep(parse_command_line(args)))
                    }
                    _ => {
                        // It's git, but not a subcommand that we parse. Don't
                        // look at any more processes.
                        ProcessArgs::ArgError
                    }
                }
            }
            // TODO: parse_style_sections is failing to parse ANSI escape sequences emitted by
            // grep (BSD and GNU), ag, pt. See #794
            Some(s) if is_any_of(s.to_str(), ["rg", "ack", "sift"]) => {
                ProcessArgs::Args(CallingProcess::OtherGrep)
            }
            Some(_) => {
                // It's not git, and it's not another grep tool. Keep
                // looking at other processes.
                ProcessArgs::OtherProcess
            }
            _ => {
                // Could not parse file stem (not expected); keep looking at
                // other processes.
                ProcessArgs::OtherProcess
            }
        },
        _ => {
            // Empty arguments (not expected); keep looking.
            ProcessArgs::OtherProcess
        }
    }
}

fn is_git_binary(git: &str) -> bool {
    // Ignore case, for e.g. NTFS or APFS file systems
    Path::new(git)
        .file_stem()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.eq_ignore_ascii_case("git"))
        .unwrap_or(false)
}

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

// Given `--aa val -bc -d val e f -- ...` return
// ({"--aa"}, {"-b", "-c", "-d"})
fn parse_command_line<'a>(args: impl Iterator<Item = &'a str>) -> CommandLine {
    let mut long_options = HashSet::new();
    let mut short_options = HashSet::new();
    let mut last_arg = None;

    for s in args {
        if s == "--" {
            break;
        } else if s.starts_with("--") {
            long_options.insert(s.split('=').next().unwrap().to_owned());
        } else if let Some(suffix) = s.strip_prefix('-') {
            short_options.extend(suffix.chars().map(|c| format!("-{}", c)));
        } else {
            last_arg = Some(s);
        }
    }

    CommandLine {
        long_options,
        short_options,
        last_arg: last_arg.map(|s| s.to_string()),
    }
}

struct ProcInfo {
    info: sysinfo::System,
}
impl ProcInfo {
    fn new() -> Self {
        // On Linux sysinfo optimizes for repeated process queries and keeps per-process
        // /proc file descriptors open. This caching is not needed here, so
        // set this to zero (this does nothing on other platforms).
        // Also, there is currently a kernel bug which slows down syscalls when threads are
        // involved (here: the ctrlc handler) and a lot of files are kept open.
        sysinfo::set_open_files_limit(0);

        ProcInfo {
            info: sysinfo::System::new(),
        }
    }
}

trait ProcActions {
    fn cmd(&self) -> &[String];
    fn parent(&self) -> Option<DeltaPid>;
    fn pid(&self) -> DeltaPid;
    fn start_time(&self) -> u64;
}

impl<T> ProcActions for T
where
    T: ProcessExt,
{
    fn cmd(&self) -> &[String] {
        ProcessExt::cmd(self)
    }
    fn parent(&self) -> Option<DeltaPid> {
        ProcessExt::parent(self).map(|p| p.as_u32())
    }
    fn pid(&self) -> DeltaPid {
        ProcessExt::pid(self).as_u32()
    }
    fn start_time(&self) -> u64 {
        ProcessExt::start_time(self)
    }
}

trait ProcessInterface {
    type Out: ProcActions;

    fn my_pid(&self) -> DeltaPid;

    fn process(&self, pid: DeltaPid) -> Option<&Self::Out>;
    fn processes(&self) -> &HashMap<Pid, Self::Out>;

    fn refresh_process(&mut self, pid: DeltaPid) -> bool;
    fn refresh_processes(&mut self);

    fn parent_process(&mut self, pid: DeltaPid) -> Option<&Self::Out> {
        self.refresh_process(pid).then_some(())?;
        let parent_pid = self.process(pid)?.parent()?;
        self.refresh_process(parent_pid).then_some(())?;
        self.process(parent_pid)
    }
    fn naive_sibling_process(&mut self, pid: DeltaPid) -> Option<&Self::Out> {
        let sibling_pid = pid - 1;
        self.refresh_process(sibling_pid).then_some(())?;
        self.process(sibling_pid)
    }
    fn find_sibling_in_refreshed_processes<F, T>(
        &mut self,
        pid: DeltaPid,
        extract_args: &F,
    ) -> Option<T>
    where
        F: Fn(&[String]) -> ProcessArgs<T>,
        Self: Sized,
    {
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

        let this_start_time = self.process(pid)?.start_time();

        let mut pid_distances = HashMap::<DeltaPid, usize>::new();
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
                    iter_parents(self, pid.as_u32(), &mut sum_distance);

                    if length_of_process_chain == usize::MAX {
                        None
                    } else {
                        Some((length_of_process_chain, args))
                    }
                }
                _ => None,
            })
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, result)| result);

        cmdline_of_closest_matching_process
    }
}

impl ProcessInterface for ProcInfo {
    type Out = Process;

    fn my_pid(&self) -> DeltaPid {
        std::process::id()
    }
    fn refresh_process(&mut self, pid: DeltaPid) -> bool {
        self.info
            .refresh_process_specifics(Pid::from_u32(pid), ProcessRefreshKind::new())
    }
    fn process(&self, pid: DeltaPid) -> Option<&Self::Out> {
        self.info.process(Pid::from_u32(pid))
    }
    fn processes(&self) -> &HashMap<Pid, Self::Out> {
        self.info.processes()
    }
    fn refresh_processes(&mut self) {
        self.info
            .refresh_processes_specifics(ProcessRefreshKind::new())
    }
}

fn calling_process_cmdline<P, F, T>(mut info: P, extract_args: F) -> Option<T>
where
    P: ProcessInterface,
    F: Fn(&[String]) -> ProcessArgs<T>,
{
    #[cfg(test)]
    {
        if let Some(args) = tests::FakeParentArgs::get() {
            match extract_args(&args) {
                ProcessArgs::Args(result) => return Some(result),
                _ => return None,
            }
        }
    }

    let my_pid = info.my_pid();

    // 1) Try the parent process(es). If delta is set as the pager in git, then git is the parent process.
    // If delta is started by a script check the parent's parent as well.
    let mut current_pid = my_pid;
    'parent_iter: for depth in [1, 2, 3] {
        let parent = match info.parent_process(current_pid) {
            None => {
                break 'parent_iter;
            }
            Some(parent) => parent,
        };
        let parent_pid = parent.pid();

        match extract_args(parent.cmd()) {
            ProcessArgs::Args(result) => return Some(result),
            ProcessArgs::ArgError => return None,

            // 2) The 1st parent process was something else, this can happen if git output is piped into delta, e.g.
            // `git blame foo.txt | delta`. When the shell sets up the pipe it creates the two processes, the pids
            // are usually consecutive, so naively check if the process with `my_pid - 1` matches.
            ProcessArgs::OtherProcess if depth == 1 => {
                let sibling = info.naive_sibling_process(current_pid);
                if let Some(proc) = sibling {
                    if let ProcessArgs::Args(result) = extract_args(proc.cmd()) {
                        return Some(result);
                    }
                }
            }
            // This check is not done for the parent's parent etc.
            ProcessArgs::OtherProcess => {}
        }
        current_pid = parent_pid;
    }

    /*
    3) Neither parent(s) nor the direct sibling were a match.
    The most likely case is that the input program of the pipe wrote all its data and exited before delta
    started, so no command line can be parsed. Same if the data was piped from an input file.

    There might also be intermediary scripts in between or piped input with a gap in pids or (rarely)
    randomized pids, so check processes for the closest match in the process tree.
    The size of this process tree can be reduced by only refreshing selected processes.

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

    // Also `add` because `A_has_pid101 | delta_has_pid102`, but if A is a wrapper which then calls
    // git (no `exec`), then the final pid of the git process might be 103 or greater.
    let pid_range = my_pid.saturating_sub(10)..my_pid.saturating_add(10);
    for p in pid_range {
        // Processes which were not refreshed do not exist for sysinfo, so by selectively
        // letting it know about processes the `find_sibling..` function will only
        // consider these.
        if info.process(p).is_none() {
            info.refresh_process(p);
        }
    }

    match info.find_sibling_in_refreshed_processes(my_pid, &extract_args) {
        None => {
            #[cfg(not(target_os = "linux"))]
            let full_scan = true;

            // The full scan is expensive on Linux and rarely successful, so disable it by default.
            #[cfg(target_os = "linux")]
            let full_scan = std::env::var("DELTA_CALLING_PROCESS_QUERY_ALL")
                .map_or(false, |v| !["0", "false", "no"].iter().any(|&n| n == v));

            if full_scan {
                info.refresh_processes();
                info.find_sibling_in_refreshed_processes(my_pid, &extract_args)
            } else {
                None
            }
        }
        some => some,
    }
}

// Walk up the process tree, calling `f` with the pid and the distance to `starting_pid`.
// Prerequisite: `info.refresh_processes()` has been called.
fn iter_parents<P, F>(info: &P, starting_pid: DeltaPid, f: F)
where
    P: ProcessInterface,
    F: FnMut(DeltaPid, usize),
{
    fn inner_iter_parents<P, F>(info: &P, pid: DeltaPid, mut f: F, distance: usize)
    where
        P: ProcessInterface,
        F: FnMut(u32, usize),
    {
        // Probably bad input, not a tree:
        if distance > 2000 {
            return;
        }
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

    use super::*;

    use itertools::Itertools;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static FAKE_ARGS: RefCell<TlsState<Vec<String>>> = RefCell::new(TlsState::None);
    }

    #[derive(Debug, PartialEq)]
    enum TlsState<T> {
        Once(T),
        Scope(T),
        With(usize, Rc<Vec<T>>),
        None,
        Invalid,
    }

    // When calling `FakeParentArgs::get()`, it can return `Some(values)` which were set earlier
    // during in the #[test]. Otherwise returns None.
    // This value can be valid once: `FakeParentArgs::once(val)`, for the entire scope:
    // `FakeParentArgs::for_scope(val)`, or can be different values every time `get()` is called:
    // `FakeParentArgs::with([val1, val2, val3])`.
    // It is an error if `once` or `with` values remain unused, or are overused.
    // Note: The values are stored per-thread, so the expectation is that no thread boundaries are
    // crossed.
    pub struct FakeParentArgs {}
    impl FakeParentArgs {
        pub fn once(args: &str) -> Self {
            Self::new(args, TlsState::Once, "once")
        }
        pub fn for_scope(args: &str) -> Self {
            Self::new(args, TlsState::Scope, "for_scope")
        }
        fn new<F>(args: &str, initial: F, from_: &str) -> Self
        where
            F: Fn(Vec<String>) -> TlsState<Vec<String>>,
        {
            let string_vec = args.split(' ').map(str::to_owned).collect();
            if FAKE_ARGS.with(|a| a.replace(initial(string_vec))) != TlsState::None {
                Self::error(from_);
            }
            FakeParentArgs {}
        }
        pub fn with(args: &[&str]) -> Self {
            let with = TlsState::With(
                0,
                Rc::new(
                    args.iter()
                        .map(|a| a.split(' ').map(str::to_owned).collect())
                        .collect(),
                ),
            );
            if FAKE_ARGS.with(|a| a.replace(with)) != TlsState::None || args.is_empty() {
                Self::error("with creation");
            }
            FakeParentArgs {}
        }
        pub fn get() -> Option<Vec<String>> {
            FAKE_ARGS.with(|a| {
                let old_value = a.replace_with(|old_value| match old_value {
                    TlsState::Once(_) => TlsState::Invalid,
                    TlsState::Scope(args) => TlsState::Scope(args.clone()),
                    TlsState::With(n, args) => TlsState::With(*n + 1, Rc::clone(args)),
                    TlsState::None => TlsState::None,
                    TlsState::Invalid => TlsState::Invalid,
                });

                match old_value {
                    TlsState::Once(args) | TlsState::Scope(args) => Some(args),
                    TlsState::With(n, args) if n < args.len() => Some(args[n].clone()),
                    TlsState::None => None,
                    TlsState::Invalid | TlsState::With(_, _) => Self::error("get"),
                }
            })
        }
        pub fn are_set() -> bool {
            FAKE_ARGS.with(|a| *a.borrow() != TlsState::None)
        }
        fn error(where_: &str) -> ! {
            panic!(
                "test logic error (in {}): wrong FakeParentArgs scope?",
                where_
            );
        }
    }
    impl Drop for FakeParentArgs {
        fn drop(&mut self) {
            // Clears an Invalid state and tests if a Once or With value has been used.
            FAKE_ARGS.with(|a| {
                let old_value = a.replace(TlsState::None);
                match old_value {
                    TlsState::With(n, args) => {
                        if n != args.len() {
                            Self::error("drop with")
                        }
                    }
                    TlsState::Once(_) | TlsState::None => Self::error("drop"),
                    TlsState::Scope(_) | TlsState::Invalid => {}
                }
            });
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

    #[derive(Debug)]
    struct FakeProc {
        #[allow(dead_code)]
        pid: DeltaPid,
        start_time: u64,
        cmd: Vec<String>,
        ppid: Option<DeltaPid>,
    }
    impl Default for FakeProc {
        fn default() -> Self {
            Self {
                pid: 0,
                start_time: 0,
                cmd: Vec::new(),
                ppid: None,
            }
        }
    }
    impl FakeProc {
        fn new(pid: DeltaPid, start_time: u64, cmd: Vec<String>, ppid: Option<DeltaPid>) -> Self {
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
        fn parent(&self) -> Option<DeltaPid> {
            self.ppid
        }
        fn pid(&self) -> DeltaPid {
            self.pid
        }
        fn start_time(&self) -> u64 {
            self.start_time
        }
    }

    #[derive(Debug)]
    struct MockProcInfo {
        delta_pid: DeltaPid,
        info: HashMap<Pid, FakeProc>,
    }
    impl Default for MockProcInfo {
        fn default() -> Self {
            Self {
                delta_pid: 0,
                info: HashMap::new(),
            }
        }
    }
    impl MockProcInfo {
        fn with(processes: &[(DeltaPid, u64, &str, Option<DeltaPid>)]) -> Self {
            MockProcInfo {
                delta_pid: processes.last().map(|p| p.0).unwrap_or(1),
                info: processes
                    .iter()
                    .map(|(pid, start_time, cmd, ppid)| {
                        let cmd_vec = cmd.split(' ').map(str::to_owned).collect();
                        (
                            Pid::from_u32(*pid),
                            FakeProc::new(*pid, *start_time, cmd_vec, *ppid),
                        )
                    })
                    .collect(),
            }
        }
    }

    impl ProcessInterface for MockProcInfo {
        type Out = FakeProc;

        fn my_pid(&self) -> DeltaPid {
            self.delta_pid
        }
        fn process(&self, pid: DeltaPid) -> Option<&Self::Out> {
            self.info.get(&Pid::from_u32(pid))
        }
        fn processes(&self) -> &HashMap<Pid, Self::Out> {
            &self.info
        }
        fn refresh_processes(&mut self) {}
        fn refresh_process(&mut self, _pid: DeltaPid) -> bool {
            true
        }
    }

    fn set(arg1: &[&str]) -> HashSet<String> {
        arg1.iter().map(|&s| s.to_owned()).collect()
    }

    #[test]
    fn test_process_testing() {
        {
            let _args = FakeParentArgs::once("git blame hello");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
                Some("hello".into())
            );
        }
        {
            let _args = FakeParentArgs::once("git blame world.txt");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
                Some("txt".into())
            );
        }
        {
            let _args = FakeParentArgs::for_scope("git blame hello world.txt");
            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
                Some("txt".into())
            );

            assert_eq!(
                calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
                Some("txt".into())
            );
        }
    }

    #[test]
    #[should_panic]
    fn test_process_testing_assert() {
        let _args = FakeParentArgs::once("git blame do.not.panic");
        assert_eq!(
            calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
            Some("panic".into())
        );

        calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension);
    }

    #[test]
    #[should_panic]
    fn test_process_testing_assert_never_used() {
        let _args = FakeParentArgs::once("never used");

        // causes a panic while panicking, so can't test:
        // let _args = FakeParentArgs::for_scope(&"never used");
        // let _args = FakeParentArgs::once(&"never used");
    }

    #[test]
    fn test_process_testing_scope_can_remain_unused() {
        let _args = FakeParentArgs::for_scope("never used");
    }

    #[test]
    fn test_process_testing_n_times_panic() {
        let _args = FakeParentArgs::with(&["git blame once", "git blame twice"]);
        assert_eq!(
            calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
            Some("once".into())
        );

        assert_eq!(
            calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
            Some("twice".into())
        );
    }

    #[test]
    #[should_panic]
    fn test_process_testing_n_times_unused() {
        let _args = FakeParentArgs::with(&["git blame once", "git blame twice"]);
    }

    #[test]
    #[should_panic]
    fn test_process_testing_n_times_underused() {
        let _args = FakeParentArgs::with(&["git blame once", "git blame twice"]);
        assert_eq!(
            calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
            Some("once".into())
        );
    }

    #[test]
    #[should_panic]
    #[ignore]
    fn test_process_testing_n_times_overused() {
        let _args = FakeParentArgs::with(&["git blame once"]);
        assert_eq!(
            calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension),
            Some("once".into())
        );
        // ignored: dropping causes a panic while panicking, so can't test
        calling_process_cmdline(ProcInfo::new(), guess_git_blame_filename_extension);
    }

    #[test]
    fn test_process_blame_no_parent_found() {
        let two_trees = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git blame src/main.rs", Some(2)),
            (4, 100, "call_delta.sh", None),
            (5, 100, "delta", Some(4)),
        ]);
        assert_eq!(
            calling_process_cmdline(two_trees, guess_git_blame_filename_extension),
            None
        );
    }

    #[test]
    fn test_process_blame_info_with_parent() {
        let no_processes = MockProcInfo::with(&[]);
        assert_eq!(
            calling_process_cmdline(no_processes, guess_git_blame_filename_extension),
            None
        );

        let parent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git blame hello.txt", Some(2)),
            (4, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(parent, guess_git_blame_filename_extension),
            Some("txt".into())
        );

        let grandparent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git blame src/main.rs", Some(2)),
            (4, 100, "call_delta.sh", Some(3)),
            (5, 100, "delta", Some(4)),
        ]);
        assert_eq!(
            calling_process_cmdline(grandparent, guess_git_blame_filename_extension),
            Some("rs".into())
        );
    }

    #[test]
    fn test_process_blame_info_with_sibling() {
        let sibling = MockProcInfo::with(&[
            (2, 100, "-xterm", None),
            (3, 100, "-shell", Some(2)),
            (4, 100, "git blame src/main.rs", Some(3)),
            (5, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(sibling, guess_git_blame_filename_extension),
            Some("rs".into())
        );

        let indirect_sibling = MockProcInfo::with(&[
            (2, 100, "-xterm", None),
            (3, 100, "-shell", Some(2)),
            (4, 100, "Git.exe blame --correct src/main.abc", Some(3)),
            (
                10,
                100,
                "Git.exe blame --ignored-child src/main.def",
                Some(4),
            ),
            (5, 100, "delta.sh", Some(3)),
            (20, 100, "delta", Some(5)),
        ]);
        assert_eq!(
            calling_process_cmdline(indirect_sibling, guess_git_blame_filename_extension),
            Some("abc".into())
        );

        let indirect_sibling2 = MockProcInfo::with(&[
            (2, 100, "-xterm", None),
            (3, 100, "-shell", Some(2)),
            (4, 100, "git wrap src/main.abc", Some(3)),
            (10, 100, "git blame src/main.def", Some(4)),
            (5, 100, "delta.sh", Some(3)),
            (20, 100, "delta", Some(5)),
        ]);
        assert_eq!(
            calling_process_cmdline(indirect_sibling2, guess_git_blame_filename_extension),
            Some("def".into())
        );

        // 3 blame processes, 2 with matching start times, pick the one with lower
        // distance but larger start time difference.
        let indirect_sibling_start_times = MockProcInfo::with(&[
            (2, 100, "-xterm", None),
            (3, 100, "-shell", Some(2)),
            (4, 109, "git wrap src/main.abc", Some(3)),
            (10, 109, "git blame src/main.def", Some(4)),
            (20, 100, "git wrap1 src/main.abc", Some(3)),
            (21, 100, "git wrap2 src/main.def", Some(20)),
            (22, 101, "git blame src/main.not", Some(21)),
            (23, 102, "git blame src/main.this", Some(20)),
            (5, 100, "delta.sh", Some(3)),
            (20, 100, "delta", Some(5)),
        ]);
        assert_eq!(
            calling_process_cmdline(
                indirect_sibling_start_times,
                guess_git_blame_filename_extension
            ),
            Some("this".into())
        );
    }

    #[test]
    fn test_describe_calling_process_grep() {
        let no_processes = MockProcInfo::with(&[]);
        assert_eq!(
            calling_process_cmdline(no_processes, describe_calling_process),
            None
        );

        let empty_command_line = CommandLine {
            long_options: [].into(),
            short_options: [].into(),
            last_arg: Some("hello.txt".to_string()),
        };
        let parent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "git grep pattern hello.txt", Some(2)),
            (4, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(parent, describe_calling_process),
            Some(CallingProcess::GitGrep(empty_command_line.clone()))
        );

        let parent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, "Git.exe grep pattern hello.txt", Some(2)),
            (4, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(parent, describe_calling_process),
            Some(CallingProcess::GitGrep(empty_command_line))
        );

        for grep_command in &[
            "/usr/local/bin/rg pattern hello.txt",
            "RG.exe pattern hello.txt",
            "/usr/local/bin/ack pattern hello.txt",
            "ack.exe pattern hello.txt",
        ] {
            let parent = MockProcInfo::with(&[
                (2, 100, "-shell", None),
                (3, 100, grep_command, Some(2)),
                (4, 100, "delta", Some(3)),
            ]);
            assert_eq!(
                calling_process_cmdline(parent, describe_calling_process),
                Some(CallingProcess::OtherGrep)
            );
        }

        let git_grep_command =
            "git grep -ab --function-context -n --show-function -W --foo=val pattern hello.txt";

        let expected_result = Some(CallingProcess::GitGrep(CommandLine {
            long_options: set(&["--function-context", "--show-function", "--foo"]),
            short_options: set(&["-a", "-b", "-n", "-W"]),
            last_arg: Some("hello.txt".to_string()),
        }));

        let parent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, git_grep_command, Some(2)),
            (4, 100, "delta", Some(3)),
        ]);
        assert_eq!(
            calling_process_cmdline(parent, describe_calling_process),
            expected_result
        );

        let grandparent = MockProcInfo::with(&[
            (2, 100, "-shell", None),
            (3, 100, git_grep_command, Some(2)),
            (4, 100, "call_delta.sh", Some(3)),
            (5, 100, "delta", Some(4)),
        ]);
        assert_eq!(
            calling_process_cmdline(grandparent, describe_calling_process),
            expected_result
        );
    }

    #[test]
    fn test_describe_calling_process_git_show() {
        for (command, expected_extension) in [
            (
                "/usr/local/bin/git show --abbrev-commit -w 775c3b84:./src/hello.rs",
                "rs",
            ),
            (
                "/usr/local/bin/git show --abbrev-commit -w HEAD~1:Makefile",
                "Makefile",
            ),
            (
                "git -c x.y=z show --abbrev-commit -w 775c3b84:./src/hello.bye.R",
                "R",
            ),
        ] {
            let parent = MockProcInfo::with(&[
                (2, 100, "-shell", None),
                (3, 100, command, Some(2)),
                (4, 100, "delta", Some(3)),
            ]);
            if let Some(CallingProcess::GitShow(cmd_line, ext)) =
                calling_process_cmdline(parent, describe_calling_process)
            {
                assert_eq!(cmd_line.long_options, set(&["--abbrev-commit"]));
                assert_eq!(cmd_line.short_options, set(&["-w"]));
                assert_eq!(ext, Some(expected_extension.to_string()));
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn test_process_calling_cmdline() {
        // Github runs CI tests for arm under qemu where where sysinfo can not find the parent process.
        if std::env::vars().any(|(key, _)| key == "CROSS_RUNNER" || key == "QEMU_LD_PREFIX") {
            return;
        }

        let mut info = ProcInfo::new();
        info.refresh_processes();
        let mut ppid_distance = Vec::new();

        iter_parents(&info, std::process::id(), |pid, distance| {
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

        // Tests that caller is something like "cargo test" or "cargo tarpaulin"
        let find_test = |args: &[String]| find_calling_process(args, &["t", "test", "tarpaulin"]);
        assert_eq!(calling_process_cmdline(info, find_test), Some(()));

        let nonsense = ppid_distance
            .iter()
            .map(|i| i.to_string())
            .join("Y40ii4RihK6lHiK4BDsGSx");

        let find_nothing = |args: &[String]| find_calling_process(args, &[&nonsense]);
        assert_eq!(calling_process_cmdline(ProcInfo::new(), find_nothing), None);
    }
}
