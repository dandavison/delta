// https://github.com/sharkdp/bat a1b9334a44a2c652f52dddaa83dbacba57372468
// src/output.rs
// See src/bat/LICENSE
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use shell_words;

use super::less::retrieve_less_version;

use crate::config;
use crate::env;
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
        pager: Option<&str>,
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
        pager_from_config: Option<&str>,
        config: &config::Config,
    ) -> Result<Self> {
        let mut replace_arguments_to_less = false;

        let pager_from_env = match (
            env::get_env_var("DELTA_PAGER"),
            env::get_env_var("BAT_PAGER"),
            env::get_env_var("PAGER"),
        ) {
            (Some(delta_pager), _, _) => Some(delta_pager),
            (None, Some(bat_pager), _) => Some(bat_pager),
            (None, None, Some(pager)) => {
                // less needs to be called with the '-R' option in order to properly interpret ANSI
                // color sequences. If someone has set PAGER="less -F", we therefore need to
                // overwrite the arguments and add '-R'.
                // We only do this for PAGER, since it is used in other contexts.
                replace_arguments_to_less = true;
                Some(pager)
            }
            _ => None,
        };

        let pager_from_config = pager_from_config.map(|p| p.to_string());

        if pager_from_config.is_some() {
            replace_arguments_to_less = false;
        }

        let pager = pager_from_config
            .or(pager_from_env)
            .unwrap_or_else(|| String::from("less"));

        let pagerflags =
            shell_words::split(&pager).chain_err(|| "Could not parse pager command.")?;

        match pagerflags.split_first() {
            Some((pager_name, args)) => {
                let pager_path = PathBuf::from(pager_name);

                let is_less = pager_path.file_stem() == Some(&OsString::from("less"));

                let mut process = if is_less {
                    let mut p = Command::new(&pager_path);
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
                            Some(version)
                                if (version < 530 || (cfg!(windows) && version < 558)) =>
                            {
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
                    p
                } else {
                    let mut p = Command::new(&pager_path);
                    p.args(args);
                    p
                };
                if config.navigate {
                    process.args(&["--pattern", &navigate::make_navigate_regexp(&config)]);
                }
                Ok(process
                    .env("LESSANSIENDCHARS", "mK")
                    .stdin(Stdio::piped())
                    .spawn()
                    .map(OutputType::Pager)
                    .unwrap_or_else(|_| OutputType::stdout()))
            }
            None => Ok(OutputType::stdout()),
        }
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

impl Drop for OutputType {
    fn drop(&mut self) {
        if let OutputType::Pager(ref mut command) = *self {
            let _ = command.wait();
        }
    }
}
