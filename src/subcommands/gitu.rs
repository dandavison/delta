//! `delta --gitu`: gitu, an interactive git client and pager, with delta as
//! its renderer.

use std::ffi::OsString;
use std::sync::Arc;
use std::{backtrace::Backtrace, env, panic};

use ::gitu::cli::{Args, Commands};
use ::gitu::config::{self, Config};
use ::gitu::{error::Error, term, Res};
use clap::{CommandFactory, FromArgMatches};

const ABOUT: &str = "An interactive git client and pager, with delta as its renderer.

Based on gitu by altsem (https://github.com/altsem/gitu), a git client inspired \
by Magit, embedded via the delta-gitu fork (https://github.com/dandavison/gitu).";

/// Run gitu with `args`, which start after `--gitu`, returning the exit code.
pub fn main(args: impl Iterator<Item = OsString>) -> i32 {
    let matches = Args::command()
        .name("delta --gitu")
        .bin_name("delta --gitu")
        .about(ABOUT)
        .mut_subcommand("completion", |completion| completion.hide(true))
        .get_matches_from(std::iter::once("delta --gitu".into()).chain(args));
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
    }
}

fn run(args: &Args) -> Res<i32> {
    if args.version {
        println!("delta {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    if let Some(Commands::Completion { .. }) = args.command {
        eprintln!("delta --gitu does not generate completions");
        return Ok(2);
    }
    if args.log {
        simple_logging::log_to_file(::gitu::LOG_FILE_NAME, log::LevelFilter::Debug)
            .map_err(Error::OpenLogFile)?;
    }

    let mut config = config::init_config(args.config.clone())?;
    let delta = env::current_exe().map_err(Error::Term)?;
    use_delta_as_renderer(&mut config, &delta.to_string_lossy());
    let config = Arc::new(config);

    let hook_config = config.clone();
    panic::set_hook(Box::new(move |panic_info| {
        if let Err(err) = term::backend().reset_term(&hook_config) {
            eprintln!("Error: {err}");
        }
        eprintln!("{panic_info}");
        eprintln!("trace: \n{}", Backtrace::force_capture());
    }));

    ::gitu::run(config, args, &mut term::backend())
}

/// Make gitu render diffs with this delta executable, whatever gitu's own config
/// says about the diff renderer, so that host and renderer agree on the protocol
/// version and nothing has to be on `PATH`.
fn use_delta_as_renderer(config: &mut Config, delta: &str) {
    let diff = &mut config.general.diff_renderer;
    diff.command = vec![delta.into()];
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    const DELTA: &str = "/path to/delta";

    #[test]
    fn test_renderers_without_config_file() {
        let config = configure("");
        let diff = &config.general.diff_renderer;
        assert_eq!(diff.command, [DELTA]);
    }

    #[test]
    fn test_diff_renderer_is_always_this_delta() {
        let config = configure(
            r#"
            [general]
            diff_renderer.command = ["delta", "--color-only"]
            "#,
        );
        let diff = &config.general.diff_renderer;
        assert_eq!(diff.command, [DELTA]);
    }

    fn configure(toml: &str) -> Config {
        let path = config_file(toml);
        let mut config = config::init_config(Some(path.clone())).unwrap();
        fs::remove_file(path).unwrap();
        use_delta_as_renderer(&mut config, DELTA);
        config
    }

    fn config_file(toml: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-{}-{:?}.toml",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::write(&path, toml).unwrap();
        path
    }
}
