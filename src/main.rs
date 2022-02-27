extern crate bitflags;

#[macro_use]
extern crate error_chain;

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

use std::io::{self, ErrorKind};
use std::process;

use bytelines::ByteLinesReader;

use crate::delta::delta;
use crate::utils::bat::assets::list_languages;
use crate::utils::bat::output::OutputType;

pub fn fatal<T>(errmsg: T) -> !
where
    T: AsRef<str> + std::fmt::Display,
{
    #[cfg(not(test))]
    {
        eprintln!("{}", errmsg);
        // As in Config::error_exit_code: use 2 for error
        // because diff uses 0 and 1 for non-error.
        process::exit(2);
    }
    #[cfg(test)]
    panic!("{}\n", errmsg);
}

pub mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
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
        .unwrap_or_else(|err| eprintln!("Failed to set ctrl-c handler: {}", err));
    let exit_code = run_app()?;
    // when you call process::exit, no destructors are called, so we want to do it only once, here
    process::exit(exit_code);
}

#[cfg(not(tarpaulin_include))]
// An Ok result contains the desired process exit code. Note that 1 is used to
// report that two files differ when delta is called with two positional
// arguments and without standard input; 2 is used to report a real problem.
fn run_app() -> std::io::Result<i32> {
    let assets = utils::bat::assets::load_highlighting_assets();
    let opt = cli::Opt::from_args_and_git_config(git_config::GitConfig::try_create(), assets);

    let subcommand_result = if opt.list_languages {
        Some(list_languages())
    } else if opt.list_syntax_themes {
        Some(subcommands::list_syntax_themes::list_syntax_themes())
    } else if opt.show_syntax_themes {
        Some(subcommands::show_syntax_themes::show_syntax_themes())
    } else if opt.show_themes {
        Some(subcommands::show_themes::show_themes(
            opt.dark,
            opt.light,
            opt.computed.is_light_mode,
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
                _ => fatal(format!("{}", error)),
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

    let mut output_type =
        OutputType::from_mode(config.paging_mode, config.pager.clone(), &config).unwrap();
    let mut writer = output_type.handle().unwrap();

    if atty::is(atty::Stream::Stdin) {
        let exit_code = subcommands::diff::diff(
            config.minus_file.as_ref(),
            config.plus_file.as_ref(),
            &config,
            &mut writer,
        );
        return Ok(exit_code);
    }

    if let Err(error) = delta(io::stdin().lock().byte_lines(), &mut writer, &config) {
        match error.kind() {
            ErrorKind::BrokenPipe => return Ok(0),
            _ => eprintln!("{}", error),
        }
    };
    Ok(0)
}
