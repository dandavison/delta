// https://github.com/sharkdp/bat a1b9334a44a2c652f52dddaa83dbacba57372468
// src/output.rs
// See src/bat_utils/LICENSE
use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use super::less::retrieve_less_version;

use crate::config;
use crate::features::navigate;

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PagingMode {
    Always,
    QuitIfOneScreen,
    Never,
}
use crate::errors::*;

pub enum OutputType {
    Pager(Child),
    Stdout(io::Stdout),
}

impl OutputType {
    pub fn from_mode(
        mode: PagingMode,
        pager: Option<String>,
        config: &config::Config,
    ) -> Result<Self> {
        use self::PagingMode::*;
        Ok(match mode {
            Always => OutputType::try_pager(false, pager, config)?,
            QuitIfOneScreen => OutputType::try_pager(true, pager, config)?,
            _ => OutputType::stdout(),
        })
    }

    /// Try to launch the pager. Fall back to stdout in case of errors.
    fn try_pager(
        quit_if_one_screen: bool,
        pager_from_config: Option<String>,
        config: &config::Config,
    ) -> Result<Self> {
        let mut replace_arguments_to_less = false;

        let pager_from_env = match (
            env::var("DELTA_PAGER"),
            env::var("BAT_PAGER"),
            env::var("PAGER"),
        ) {
            (Ok(delta_pager), _, _) => Some(delta_pager),
            (_, Ok(bat_pager), _) => Some(bat_pager),
            (_, _, Ok(pager)) => {
                // less needs to be called with the '-R' option in order to properly interpret ANSI
                // color sequences. If someone has set PAGER="less -F", we therefore need to
                // overwrite the arguments and add '-R'.
                // We only do this for PAGER, since it is used in other contexts.
                replace_arguments_to_less = true;
                Some(pager)
            }
            _ => None,
        };

        if pager_from_config.is_some() {
            replace_arguments_to_less = false;
        }

        let pager = pager_from_config
            .or(pager_from_env)
            .unwrap_or_else(|| String::from("less"));

        let pagerflags =
            shell_words::split(&pager).chain_err(|| "Could not parse pager command.")?;

        Ok(match pagerflags.split_first() {
            Some((pager_name, args)) => {
                let pager_path = PathBuf::from(pager_name);

                let is_less = pager_path.file_stem() == Some(&OsString::from("less"));

                let process = if is_less {
                    _make_process_from_less_path(
                        pager_path,
                        args,
                        replace_arguments_to_less,
                        quit_if_one_screen,
                        config,
                    )
                } else {
                    _make_process_from_pager_path(pager_path, args)
                };
                if let Some(mut process) = process {
                    process
                        .stdin(Stdio::piped())
                        .spawn()
                        .map(OutputType::Pager)
                        .unwrap_or_else(|_| OutputType::stdout())
                } else {
                    OutputType::stdout()
                }
            }
            None => OutputType::stdout(),
        })
    }

    fn stdout() -> Self {
        OutputType::Stdout(io::stdout())
    }

    pub fn handle(&mut self) -> Result<&mut dyn Write> {
        Ok(match *self {
            OutputType::Pager(ref mut command) => command
                .stdin
                .as_mut()
                .chain_err(|| "Could not open stdin for pager")?,
            OutputType::Stdout(ref mut handle) => handle,
        })
    }
}

fn _make_process_from_less_path(
    less_path: PathBuf,
    args: &[String],
    replace_arguments_to_less: bool,
    quit_if_one_screen: bool,
    config: &config::Config,
) -> Option<Command> {
    if let Ok(less_path) = grep_cli::resolve_binary(less_path) {
        let mut p = Command::new(&less_path);
        if args.is_empty() || replace_arguments_to_less {
            p.args(vec!["--RAW-CONTROL-CHARS"]);

            // Passing '--no-init' fixes a bug with '--quit-if-one-screen' in older
            // versions of 'less'. Unfortunately, it also breaks mouse-wheel support.
            //
            // See: http://www.greenwoodsoftware.com/less/news.530.html
            //
            // For newer versions (530 or 558 on Windows), we omit '--no-init' as it
            // is not needed anymore.
            match retrieve_less_version() {
                None => {
                    p.arg("--no-init");
                }
                Some(version) if (version < 530 || (cfg!(windows) && version < 558)) => {
                    p.arg("--no-init");
                }
                _ => {}
            }

            if quit_if_one_screen {
                p.arg("--quit-if-one-screen");
            }
        } else {
            p.args(args);
        }
        p.env("LESSCHARSET", "UTF-8");
        p.env("LESSANSIENDCHARS", "mK");
        if config.navigate {
            if let Ok(hist_file) = navigate::copy_less_hist_file_and_append_navigate_regexp(config)
            {
                p.env("LESSHISTFILE", hist_file);
                if config.show_themes {
                    p.arg("+n");
                }
            }
        }
        Some(p)
    } else {
        None
    }
}

fn _make_process_from_pager_path(pager_path: PathBuf, args: &[String]) -> Option<Command> {
    if pager_path.file_stem() == Some(&OsString::from("delta")) {
        eprintln!(
            "\
It looks like you have set delta as the value of $PAGER. \
This would result in a non-terminating recursion. \
delta is not an appropriate value for $PAGER \
(but it is an appropriate value for $GIT_PAGER)."
        );
        std::process::exit(1);
    }
    if let Ok(pager_path) = grep_cli::resolve_binary(pager_path) {
        let mut p = Command::new(&pager_path);
        p.args(args);
        Some(p)
    } else {
        None
    }
}

impl Drop for OutputType {
    fn drop(&mut self) {
        if let OutputType::Pager(ref mut command) = *self {
            let _ = command.wait();
        }
    }
}
