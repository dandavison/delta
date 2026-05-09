mod align;
mod ansi;
mod cli;
mod color;
mod colors;
mod config;
mod delta;
mod edits;
mod env;
mod features;
mod format;
mod git_config;
mod handlers;
mod minusplus;
mod options;
mod paint;
mod parse_style;
mod parse_styles;
mod style;
mod utils;
mod wrapping;

mod subcommands;

mod tests;

use std::ffi::{OsStr, OsString};
use std::io::{self, BufRead, Cursor, ErrorKind, IsTerminal, Write};
use std::process::{self, Command, Stdio};

use bytelines::ByteLinesReader;

use crate::cli::Call;
use crate::config::delta_unreachable;
use crate::delta::delta;
use crate::subcommands::{SubCmdKind, SubCommand};
use crate::utils::bat::assets::list_languages;
use crate::utils::bat::output::{OutputType, PagingMode};

pub fn fatal<T>(errmsg: T) -> !
where
    T: AsRef<str> + std::fmt::Display,
{
    #[cfg(not(test))]
    {
        eprintln!("{errmsg}");
        // As in Config::error_exit_code: use 2 for error
        // because diff uses 0 and 1 for non-error.
        process::exit(2);
    }
    #[cfg(test)]
    panic!("{}\n", errmsg);
}

pub mod errors {
    pub use anyhow::{anyhow, Context, Error, Result};
}

#[cfg(not(tarpaulin_include))]
fn main() -> std::io::Result<()> {
    // Do this first because both parsing all the input in `run_app()` and
    // listing all processes takes about 50ms on Linux.
    // It also improves the chance that the calling process is still around when
    // input is piped into delta (e.g. `git show  --word-diff=color | delta`).
    utils::process::start_determining_calling_process_in_thread();

    // Ignore ctrl-c (SIGINT) to avoid leaving an orphaned pager process.
    // See https://github.com/dandavison/delta/issues/681
    ctrlc::set_handler(|| {})
        .unwrap_or_else(|err| eprintln!("Failed to set ctrl-c handler: {err}"));
    let exit_code = run_app(std::env::args_os().collect::<Vec<_>>(), None)?;
    // when you call process::exit, no drop impls are called, so we want to do it only once, here
    process::exit(exit_code);
}

#[cfg(not(tarpaulin_include))]
// An Ok result contains the desired process exit code. Note that 1 is used to
// report that two files differ when delta is called with two positional
// arguments and without standard input; 2 is used to report a real problem.
pub fn run_app(
    args: Vec<OsString>,
    capture_output: Option<&mut Cursor<Vec<u8>>>,
) -> std::io::Result<i32> {
    let env = env::DeltaEnv::init();
    let assets = utils::bat::assets::load_highlighting_assets();
    let (call, opt) = cli::Opt::from_args_and_git_config(args, &env, assets);

    if let Call::Version(msg) = call {
        writeln!(std::io::stdout(), "{}", msg.trim_end())?;
        return Ok(0);
    } else if let Call::Help(msg) = call {
        OutputType::oneshot_write(msg)?;
        return Ok(0);
    } else if let Call::SubCommand(_, cmd) = &call {
        // Set before creating the Config, which already asks for the calling process
        // (not required for Call::DeltaDiff)
        utils::process::set_calling_process(
            &cmd.args
                .iter()
                .map(|arg| OsStr::to_string_lossy(arg).to_string())
                .collect::<Vec<_>>(),
        );
    }
    let opt = opt.unwrap_or_else(|| delta_unreachable("Opt is set"));

    let subcommand_result = if let Some(shell) = opt.generate_completion {
        Some(subcommands::generate_completion::generate_completion_file(
            shell,
        ))
    } else if opt.list_languages {
        Some(list_languages())
    } else if opt.list_syntax_themes {
        Some(subcommands::list_syntax_themes::list_syntax_themes())
    } else if opt.show_syntax_themes {
        Some(subcommands::show_syntax_themes::show_syntax_themes())
    } else if opt.show_themes {
        Some(subcommands::show_themes::show_themes(
            opt.dark,
            opt.light,
            opt.computed.color_mode,
        ))
    } else if opt.show_colors {
        Some(subcommands::show_colors::show_colors())
    } else if opt.parse_ansi {
        Some(subcommands::parse_ansi::parse_ansi())
    } else {
        None
    };
    if let Some(result) = subcommand_result {
        if let Err(error) = result {
            match error.kind() {
                ErrorKind::BrokenPipe => {}
                _ => fatal(format!("{error}")),
            }
        }
        return Ok(0);
    };

    let _show_config = opt.show_config;
    let config = config::Config::from(opt);

    if _show_config {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        subcommands::show_config::show_config(&config, &mut stdout)?;
        return Ok(0);
    }

    // The following block structure is because of `writer` and related lifetimes:
    let pager_cfg = (&config).into();
    let paging_mode = if capture_output.is_some() {
        PagingMode::Capture
    } else if config.paging_mode != PagingMode::Always && !io::stdout().is_terminal() {
        PagingMode::Never
    } else {
        config.paging_mode
    };
    let mut output_type =
        OutputType::from_mode(&env, paging_mode, config.pager.clone(), &pager_cfg).unwrap();
    let mut writer: &mut dyn Write = if paging_mode == PagingMode::Capture {
        &mut capture_output.unwrap()
    } else {
        output_type.handle().unwrap()
    };

    let subcmd = match call {
        Call::DeltaDiff(_, minus, plus) => {
            match subcommands::diff::build_diff_cmd(&minus, &plus, &config) {
                Err(code) => return Ok(code),
                Ok(val) => val,
            }
        }
        Call::SubCommand(_, subcmd) => subcmd,
        Call::Delta(_) => SubCommand::none(),
        Call::Help(_) | Call::Version(_) => delta_unreachable("help/version handled earlier"),
    };

    if subcmd.is_none() {
        // Default delta run: read input from stdin, write to stdout or pager (pager started already^).

        if io::stdin().is_terminal() {
            eprintln!(
                "\
                    The main way to use delta is to configure it as the pager for git: \
                    see https://github.com/dandavison/delta#get-started. \
                    You can also use delta to diff two files: `delta file_A file_B`."
            );
            return Ok(config.error_exit_code);
        }

        let res = delta(io::stdin().lock().byte_lines(), &mut writer, &config);

        if let Err(error) = res {
            match error.kind() {
                ErrorKind::BrokenPipe => return Ok(0),
                _ => {
                    eprintln!("{error}");
                    return Ok(config.error_exit_code);
                }
            }
        }

        Ok(0)
    } else {
        // First start a subcommand, and pipe input from it to delta(). Also handle
        // subcommand exit code and stderr (maybe truncate it, e.g. for git and diff logic).

        let (subcmd_bin, subcmd_args) = subcmd.args.split_first().unwrap();
        let subcmd_kind = subcmd.kind; // for easier {} formatting

        let subcmd_bin_path = match grep_cli::resolve_binary(std::path::PathBuf::from(subcmd_bin)) {
            Ok(path) => path,
            Err(err) => {
                eprintln!("Failed to resolve command {subcmd_bin:?}: {err}");
                return Ok(config.error_exit_code);
            }
        };

        let cmd = Command::new(subcmd_bin)
            .args(subcmd_args.iter())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        if let Err(err) = cmd {
            eprintln!("Failed to execute the command {subcmd_bin:?}: {err}");
            return Ok(config.error_exit_code);
        }
        let mut cmd = cmd.unwrap();

        let cmd_stdout = cmd
            .stdout
            .take()
            .unwrap_or_else(|| panic!("Failed to open stdout"));
        let cmd_stdout_buf = io::BufReader::new(cmd_stdout);

        let res = delta(cmd_stdout_buf.byte_lines(), &mut writer, &config);

        if let Err(error) = res {
            let _ = cmd.wait(); // for clippy::zombie_processes
            match error.kind() {
                ErrorKind::BrokenPipe => return Ok(0),
                _ => {
                    eprintln!("{error}");
                    return Ok(config.error_exit_code);
                }
            }
        };

        let subcmd_status = cmd
            .wait()
            .unwrap_or_else(|_| {
                delta_unreachable(&format!("{subcmd_kind:?} process not running."));
            })
            .code()
            .unwrap_or_else(|| {
                eprintln!("delta: {subcmd_kind:?} process terminated without exit status.");
                config.error_exit_code
            });

        let mut stderr_lines = io::BufReader::new(
            cmd.stderr
                .unwrap_or_else(|| panic!("Failed to open stderr")),
        )
        .lines();
        if let Some(line1) = stderr_lines.next() {
            // prefix the first error line with the called subcommand
            eprintln!(
                "{}: {}",
                subcmd_kind,
                line1.unwrap_or("<delta: could not parse stderr line>".into())
            );
        }

        // On `git diff` unknown option error: stop after printing the first line above (which is
        // an error message), because the entire --help text follows.
        if !(subcmd_status == 129
            && matches!(subcmd_kind, SubCmdKind::GitDiff | SubCmdKind::Git(_)))
        {
            for line in stderr_lines {
                eprintln!(
                    "{}",
                    line.unwrap_or("<delta: could not parse stderr line>".into())
                );
            }
        }

        if matches!(subcmd_kind, SubCmdKind::GitDiff | SubCmdKind::Diff) && subcmd_status >= 2 {
            eprintln!(
                "{subcmd_kind:?} process failed with exit status {subcmd_status}. Command was: {}",
                format_args!(
                    "{} {}",
                    subcmd_bin_path.display(),
                    shell_words::join(
                        subcmd_args
                            .iter()
                            .map(|arg0: &OsString| std::ffi::OsStr::to_string_lossy(arg0))
                    ),
                )
            );
        }

        Ok(subcmd_status)
    }

    // `output_type` drop impl runs here
}

#[cfg(test)]
mod main_tests {
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Locate the delta binary in the cargo target directory.
    /// During `cargo test`, the test binary is at target/debug/deps/git_delta-<hash>,
    /// and the main binary is at target/debug/delta.
    fn delta_binary() -> Option<std::path::PathBuf> {
        let test_bin = std::env::current_exe().ok()?;
        let mut path = test_bin;
        path.pop(); // deps/
        path.pop(); // debug/ or release/
        path.push("delta");
        if path.exists() { Some(path) } else { None }
    }

    #[test]
    fn test_no_output_on_empty_stdin_when_stdout_piped() {
        let delta_bin = delta_binary().expect(
            "delta binary not found at target/debug/delta; run `cargo build --bin delta` first"
        );

        let mut child = Command::new(&delta_bin)
            .arg("--no-gitconfig")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn delta");

        child.stdin.take().unwrap().write_all(b"").unwrap();
        let output = child.wait_with_output().unwrap();

        assert!(
            output.status.success(),
            "delta exited with error on empty stdin: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout.len(),
            0,
            "Expected 0 bytes on empty stdin piped to delta, got {} bytes: {:?}",
            output.stdout.len(),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn test_empty_stdin_does_not_hang_with_paging_always() {
        let delta_bin = delta_binary().expect(
            "delta binary not found at target/debug/delta; run `cargo build --bin delta` first"
        );

        let mut child = Command::new(&delta_bin)
            .arg("--no-gitconfig")
            .arg("--paging=always")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn delta");

        child.stdin.take().unwrap().write_all(b"").unwrap();
        let output = child.wait_with_output().unwrap();

        // --paging=always still starts the pager even when stdout is piped.
        // The test verifies this doesn't hang and exits cleanly.
        assert!(
            output.status.success(),
            "delta --paging=always exited with error on empty stdin: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
