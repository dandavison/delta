//! `delta --tui` (alias `--gitu`): gitu, an interactive git client and pager,
//! with delta as its renderer.

use std::ffi::OsString;
use std::sync::Arc;
use std::{backtrace::Backtrace, env, panic};

use ::gitu::cli::Args;
use ::gitu::config::{self, ColorMode, Config, EnabledFeatures, RendererGitConfig};
use ::gitu::{error::Error, term, Res};
use clap::{CommandFactory, FromArgMatches};

use crate::git_config::GitConfig;

const ABOUT: &str = "An interactive git client and pager, with delta as its renderer.

Based on gitu by altsem (https://github.com/altsem/gitu), a git client inspired \
by Magit, embedded via the delta-gitu fork (https://github.com/dandavison/gitu).";

/// Where delta's own options go, for a user who gave them to `delta --tui`.
const DELTA_OPTIONS_GO_IN_GIT_CONFIG: &str = "delta's own options cannot be given with --tui. \
Set them in git config's [delta] section, or turn delta features on and off in the TUI with S, L and H.";

/// Run gitu with `args`, which start after `--tui` or `--gitu`, returning the
/// exit code.
pub fn main(args: impl IntoIterator<Item = OsString>) -> i32 {
    let args = match parse(args) {
        Ok(args) => args,
        Err(err) => {
            let message = error_message(&err);
            if err.use_stderr() {
                eprint!("{message}");
            } else {
                print!("{message}");
            }
            return err.exit_code();
        }
    };
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
    }
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, clap::Error> {
    let matches = Args::command()
        .name("delta --tui")
        .bin_name("delta --tui")
        .about(ABOUT)
        .try_get_matches_from(std::iter::once("delta --tui".into()).chain(args))?;
    Args::from_arg_matches(&matches)
}

/// What clap says of `err`, and, for an argument gitu does not take, where
/// delta's options go: that argument is most likely one of them.
fn error_message(err: &clap::Error) -> String {
    let mut message = err.render().to_string();
    if err.kind() == clap::error::ErrorKind::UnknownArgument {
        message.push_str(&format!("\n{DELTA_OPTIONS_GO_IN_GIT_CONFIG}\n"));
    }
    message
}

/// What to tell a user whose `--tui` is not delta's first argument (`args`
/// include the program), where delta would take it as one of its own options
/// and go on to read a diff that is not coming.
pub fn misplaced_tui(args: impl IntoIterator<Item = OsString>) -> Option<String> {
    let mut args = args.into_iter().skip(1);
    if args.next().is_some_and(|arg| is_tui(&arg)) {
        return None;
    }
    args.any(|arg| is_tui(&arg)).then(|| {
        format!("--tui must be delta's first argument, as in `delta --tui`. {DELTA_OPTIONS_GO_IN_GIT_CONFIG}")
    })
}

pub fn is_tui(arg: &OsString) -> bool {
    arg == "--tui" || arg == "--gitu"
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
/// gitu can offer the themes for that mode first, how to ask which features the
/// user's config enables, so gitu can show them as on and turn them off, and how
/// delta opens git config, so that gitu offers themes delta can find.
fn tell_gitu_about_delta(config: &mut Config, opt: &crate::cli::Opt) {
    let renderer = &mut config.general.diff_renderer;
    renderer.color_mode = Some(color_mode(opt));
    renderer.enabled_features = EnabledFeatures::new(|dir| {
        let env = crate::env::DeltaEnv {
            current_dir: Some(dir.to_path_buf()),
            ..crate::env::DeltaEnv::init()
        };
        enabled_features(&env, GitConfig::try_create(&env))
    });
    renderer.git_config = RendererGitConfig::new(|dir| crate::git_config::open(Some(dir)).1);
}

/// The features that `git_config`, as it is now, enables, by delta's name: those it names, not
/// those a builtin feature brings in as its defaults. gitu asks this
/// during the session, so the user's git config is read again each time. A feature
/// enabled with a minimum width, such as side-by-side = 140, counts whatever the
/// terminal's width: the user's rule decides whether it is in effect.
fn enabled_features(env: &crate::env::DeltaEnv, git_config: Option<GitConfig>) -> Vec<String> {
    let env = crate::env::DeltaEnv {
        // Wider than any minimum width a config gives, so that every one is met. delta only
        // settles its options here; nothing is rendered at this width.
        columns: Some(u32::MAX.to_string()),
        ..env.clone()
    };
    // gitu has the terminal, so delta must not ask it its background.
    let args = ["delta", "--detect-dark-light=never"];
    let opt = crate::cli::Opt::from_iter_and_git_config(&env, args, git_config);
    // What a builtin feature brings in as a default, such as side-by-side's line numbers, is not
    // enabled by the user's config.
    opt.features
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .filter(|feature| opt.computed.features_named_by_user.contains(*feature))
        .map(String::from)
        .collect()
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

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn test_deltas_options_given_to_delta_tui_say_where_they_go() {
        let err = parse(os_args(&["--side-by-side"])).unwrap_err();
        let message = error_message(&err);
        assert!(message.contains("'--side-by-side'"), "{}", message);
        assert!(message.contains("[delta]"), "{}", message);
    }

    #[test]
    fn test_tui_after_other_arguments_says_it_must_come_first() {
        let message = misplaced_tui(os_args(&["delta", "--side-by-side", "--tui"])).unwrap();
        assert!(message.contains("first"), "{}", message);
        assert!(message.contains("[delta]"), "{}", message);
        assert_eq!(misplaced_tui(os_args(&["delta", "--tui"])), None);
        assert_eq!(misplaced_tui(os_args(&["delta", "a", "b"])), None);
    }

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

    /// gitu offers themes from the git config delta reads, so that a theme it offers is one delta
    /// can find.
    #[test]
    fn test_gitu_reads_the_git_config_delta_reads() {
        let repo = std::env::temp_dir().join(format!(
            "delta-gitu-test-repo-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let git_repo = git2::Repository::init(&repo).unwrap();
        git_repo
            .config()
            .unwrap()
            .set_bool("delta.mine.dark", true)
            .unwrap();

        let mut config = configure("");
        tell_gitu_about_delta(&mut config, &options_with("[delta]\n"));
        let git_config = config.general.diff_renderer.git_config.open(&repo);
        fs::remove_dir_all(&repo).unwrap();

        let git_config = git_config
            .expect("delta says how it opens git config")
            .unwrap();
        assert_eq!(git_config.get_bool("delta.mine.dark").ok(), Some(true));
    }

    /// gitu is told the features the user's config enables, so that it shows them
    /// as on and turns them off when toggled.
    #[test]
    fn test_gitu_is_told_the_features_delta_uses() {
        let in_force = enabled_with(
            "[delta]\n    side-by-side = true\n    features = mine\n[delta \"mine\"]\n    navigate = true\n",
        );
        for feature in ["side-by-side", "mine", "navigate"] {
            assert!(
                in_force.iter().any(|f| f == feature),
                "{}: {:?}",
                feature,
                in_force
            );
        }
        // side-by-side brings in line numbers as one of its defaults: the user's config does not
        // enable them.
        for feature in ["line-numbers", "keep-plus-minus-markers"] {
            assert!(
                !in_force.iter().any(|f| f == feature),
                "{}: {:?}",
                feature,
                in_force
            );
        }
    }

    /// A key toggles a feature between enabled and disabled, starting from whether the user's
    /// config enables it. That is decided as delta decides it, for any name: a builtin switch
    /// feature, a builtin style feature, the user's own feature, or a builtin name the user also
    /// has a section for. A switch feature enabled with a minimum width is enabled whatever the
    /// width: the user's rule decides whether it is in effect.
    #[test]
    fn test_gitu_is_told_each_feature_the_users_config_enables_whatever_the_width() {
        let cases: [(&str, &str, bool); 19] = [
            // What a builtin feature brings in as a default is not enabled by the user's config,
            // unless the user's config also names it.
            ("[delta]\n    side-by-side = true\n", "line-numbers", false),
            ("[delta]\n    side-by-side = 99999\n", "line-numbers", false),
            ("[delta]\n    raw = true\n", "keep-plus-minus-markers", false),
            (
                "[delta]\n    side-by-side = true\n    line-numbers = true\n",
                "line-numbers",
                true,
            ),
            (
                "[delta]\n    features = side-by-side line-numbers\n",
                "line-numbers",
                true,
            ),
            (
                "[delta]\n    side-by-side = true\n    features = mine\n[delta \"mine\"]\n    line-numbers = true\n",
                "line-numbers",
                true,
            ),
            ("[delta]\n    side-by-side = 99999\n", "side-by-side", true),
            ("[delta]\n    side-by-side = hunk,99999\n", "side-by-side", true),
            ("[delta]\n    features = side-by-side\n", "side-by-side", true),
            (
                "[delta]\n    features = mine\n[delta \"mine\"]\n    side-by-side = 99999\n",
                "side-by-side",
                true,
            ),
            (
                "[delta]\n    side-by-side = false\n    features = side-by-side\n",
                "side-by-side",
                false,
            ),
            (
                "[delta \"side-by-side\"]\n    line-numbers-left-format = x\n",
                "side-by-side",
                false,
            ),
            (
                "[delta]\n    side-by-side = true\n[delta \"side-by-side\"]\n    line-numbers-left-format = x\n",
                "side-by-side",
                true,
            ),
            ("[delta]\n    raw = true\n", "raw", true),
            ("[delta \"raw\"]\n    file-style = red\n", "raw", false),
            (
                "[delta]\n    features = raw\n[delta \"raw\"]\n    file-style = red\n",
                "raw",
                true,
            ),
            ("[delta \"mine\"]\n    file-style = red\n", "mine", false),
            (
                "[delta]\n    features = mine\n[delta \"mine\"]\n    file-style = red\n",
                "mine",
                true,
            ),
            (
                "[delta]\n    features = mine\n[delta \"mine\"]\n    features = child\n[delta \"child\"]\n    file-style = red\n",
                "child",
                true,
            ),
        ];
        let failures: Vec<String> = cases
            .iter()
            .filter_map(|(git_config, feature, expected)| {
                let in_force = enabled_with(git_config);
                let enabled = in_force.iter().any(|f| f == feature);
                (enabled != *expected).then(|| {
                    format!(
                        "{feature} should be {} with {git_config:?}: {in_force:?}",
                        if *expected { "enabled" } else { "disabled" }
                    )
                })
            })
            .collect();
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
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

    /// The features gitu is told `git_config` enables, when it asks.
    fn enabled_with(git_config: &str) -> Vec<String> {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-enabled-{}-{:?}.gitconfig",
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
        let enabled = enabled_features(&env, Some(git_config));
        fs::remove_file(path).unwrap();
        enabled
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
