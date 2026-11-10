//! `delta --tui` (alias `--gitu`): gitu, an interactive git client and pager,
//! with delta as its renderer.

use std::ffi::OsString;
use std::sync::Arc;
use std::{backtrace::Backtrace, env, panic};

use ::gitu::cli::Args;
use ::gitu::config::{self, ColorMode, Config};
use ::gitu::{error::Error, term, Res};
use clap::{CommandFactory, FromArgMatches};

use crate::git_config::GitConfig;

const ABOUT: &str = "An interactive git client and pager, with delta as its renderer.

Based on gitu by altsem (https://github.com/altsem/gitu), a git client inspired \
by Magit, embedded via the delta-gitu fork (https://github.com/dandavison/gitu).";

/// Run gitu with `args`, which start after `--tui` or `--gitu`, returning the
/// exit code.
pub fn main(args: impl Iterator<Item = OsString>) -> i32 {
    let matches = Args::command()
        .name("delta --tui")
        .bin_name("delta --tui")
        .about(ABOUT)
        .get_matches_from(std::iter::once("delta --tui".into()).chain(args));
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
    if args.log {
        simple_logging::log_to_file(::gitu::LOG_FILE_NAME, log::LevelFilter::Debug)
            .map_err(Error::OpenLogFile)?;
    }

    let mut config = config::init_config(args.config.clone())?;
    let delta = env::current_exe().map_err(Error::Term)?;
    use_delta_as_renderer(&mut config, &delta.to_string_lossy());
    let env = crate::env::DeltaEnv::init();
    let opt = delta_options(&env, GitConfig::try_create(&env));
    tell_gitu_about_delta(&mut config, &opt);
    // The deltas gitu runs write to pipes, so they cannot ask the terminal its
    // background; this one asked, unless git config sets light or dark, before
    // gitu takes the terminal over.
    if let Some(mode) = opt.computed.terminal_color_mode {
        let mode = match mode {
            crate::color::ColorMode::Dark => "dark",
            crate::color::ColorMode::Light => "light",
        };
        env::set_var(crate::env::DELTA_TERMINAL_COLOR_MODE, mode);
    }
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

/// Tell gitu what delta does with the user's config: the mode it renders in, so
/// gitu can offer the themes for that mode first, and the features it uses, so
/// gitu can show them as on and turn them off.
fn tell_gitu_about_delta(config: &mut Config, opt: &crate::cli::Opt) {
    let renderer = &mut config.general.diff_renderer;
    renderer.color_mode = Some(color_mode(opt));
    renderer.features_in_force = opt
        .features
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect();
}

/// delta's options as `git_config` and the terminal settle them. They are settled
/// before gitu takes over the terminal, so the terminal can still be asked.
fn delta_options(env: &crate::env::DeltaEnv, git_config: Option<GitConfig>) -> crate::cli::Opt {
    crate::cli::Opt::from_iter_and_git_config(env, ["delta"], git_config)
}

/// The mode delta renders in, light or dark: from git config and, unless that
/// says, the terminal's background.
fn color_mode(opt: &crate::cli::Opt) -> ColorMode {
    match opt.computed.color_mode {
        crate::color::ColorMode::Dark => ColorMode::Dark,
        crate::color::ColorMode::Light => ColorMode::Light,
    }
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

    /// gitu is told the mode delta renders in, so that it can offer the themes
    /// for that mode first.
    #[test]
    fn test_gitu_is_told_the_mode_delta_renders_in() {
        assert_eq!(
            color_mode_with("[delta]\n    light = true\n"),
            ColorMode::Light
        );
        assert_eq!(
            color_mode_with("[delta]\n    dark = true\n"),
            ColorMode::Dark
        );
    }

    #[test]
    fn test_a_feature_in_git_config_can_set_the_mode() {
        assert_eq!(
            color_mode_with("[delta]\n    features = mine\n[delta \"mine\"]\n    light = true\n"),
            ColorMode::Light
        );
    }

    fn color_mode_with(git_config: &str) -> ColorMode {
        color_mode(&options_with(git_config))
    }

    /// gitu is told the features delta uses with the user's config, so that it
    /// shows them as on and turns them off when toggled.
    #[test]
    fn test_gitu_is_told_the_features_delta_uses() {
        let config = configure_with_git_config(
            "[delta]\n    side-by-side = true\n    features = mine\n[delta \"mine\"]\n    navigate = true\n",
        );
        let in_force = &config.general.diff_renderer.features_in_force;
        for feature in ["side-by-side", "line-numbers", "mine", "navigate"] {
            assert!(
                in_force.iter().any(|f| f == feature),
                "{}: {:?}",
                feature,
                in_force
            );
        }
        assert!(!in_force.iter().any(|f| f == "keep-plus-minus-markers"));
    }

    fn options_with(git_config: &str) -> crate::cli::Opt {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-{}-{:?}.gitconfig",
            std::process::id(),
            std::thread::current().id()
        ));
        let env = crate::env::DeltaEnv::default();
        let git_config = crate::tests::integration_test_utils::make_git_config(
            &env,
            git_config.as_bytes(),
            path.to_str().unwrap(),
            false,
        );
        let opt = delta_options(&env, Some(git_config));
        fs::remove_file(path).unwrap();
        opt
    }

    /// gitu's config as `delta --tui` gives it, with `git_config` as the user's.
    fn configure_with_git_config(git_config: &str) -> Config {
        let mut config = configure("");
        tell_gitu_about_delta(&mut config, &options_with(git_config));
        config
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
