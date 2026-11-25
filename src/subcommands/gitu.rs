//! `delta tui`: gitu, an interactive git client and pager, with delta as its
//! renderer. `delta --tui` and `delta --gitu`, as delta 0.22.0 ran it, still do.

use std::ffi::OsString;
use std::sync::Arc;
use std::{backtrace::Backtrace, env, panic};

use ::gitu::cli::Args;
use ::gitu::config::{
    self, ColorMode, Config, EnabledFeatures, RendererGitConfig, RendererOptions,
    RendererSideBySide,
};
use ::gitu::{error::Error, term, Res};
use clap::{CommandFactory, FromArgMatches};

use crate::git_config::GitConfig;

const ABOUT: &str = "An interactive git client and pager, with delta as its renderer.

Based on gitu by altsem (https://github.com/altsem/gitu), a git client inspired \
by Magit, embedded via the delta-gitu fork (https://github.com/dandavison/gitu).";

/// Where delta's own options go, for a user who gave them to `delta tui`.
const DELTA_OPTIONS_GO_IN_GIT_CONFIG: &str = "delta's own options cannot be given with delta tui. \
Set them in git config's [delta] section, or turn delta features on and off in the TUI with S, L and H.";

/// Run gitu with `args`, which start after `tui`, `--tui` or `--gitu`,
/// returning the exit code.
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

/// Whether this delta-gitu has `--agent-instructions`.
pub fn has_agent_instructions() -> bool {
    Args::command()
        .get_arguments()
        .any(|arg| arg.get_long() == Some("agent-instructions"))
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, clap::Error> {
    let matches = Args::command()
        .name("delta tui")
        .bin_name("delta tui")
        .about(ABOUT)
        .try_get_matches_from(std::iter::once("delta tui".into()).chain(args))?;
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
    args.any(|arg| is_tui_option(&arg)).then(|| {
        format!("To run the TUI, tui must be delta's first argument, as in `delta tui`. {DELTA_OPTIONS_GO_IN_GIT_CONFIG}")
    })
}

/// Whether `arg`, as delta's first argument, runs the TUI: `tui`, as `git` and
/// `rg` run those programs, or `--tui`, or its alias `--gitu`.
pub fn is_tui(arg: &OsString) -> bool {
    arg == "tui" || is_tui_option(arg)
}

/// Whether `arg` is `--tui` or its alias `--gitu`, which delta would take as one
/// of its own options anywhere but first.
fn is_tui_option(arg: &OsString) -> bool {
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
/// user's config enables, so gitu can show them as on and turn them off, how
/// delta opens git config, so that gitu offers themes delta can find, and how to
/// ask what the user's config sets options gitu draws with to, such as
/// `comment-style`, so they are set as delta's own are, and how to ask whether
/// delta draws side by side at a width, so that gitu lays out difftastic's view
/// as delta draws it.
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
    renderer.options = RendererOptions::new(
        |dir, features, names| {
            let env = options_env(dir, features);
            option_values(&env, GitConfig::try_create(&env), names)
        },
        |dir, features, names| {
            let env = options_env(dir, features);
            option_lists(&env, GitConfig::try_create(&env), names)
        },
    );
    renderer.side_by_side = RendererSideBySide::new(|dir, features, columns| {
        let env = crate::env::DeltaEnv {
            columns: Some(columns.to_string()),
            ..options_env(dir, features)
        };
        draws_side_by_side(&env, GitConfig::try_create(&env))
    });
}

/// Whether delta draws side by side in `env`, whose `COLUMNS` is the width it draws at: a
/// minimum width such as `side-by-side = 120` must be met.
fn draws_side_by_side(env: &crate::env::DeltaEnv, git_config: Option<GitConfig>) -> bool {
    // gitu has the terminal, so delta must not ask it its background.
    let args = ["delta", "--detect-dark-light=never"];
    crate::cli::Opt::from_iter_and_git_config(env, args, git_config)
        .computed
        .side_by_side
}

/// delta's environment in `dir`, with `features` in force as `DELTA_FEATURES` gives them.
fn options_env(dir: &std::path::Path, features: Option<&str>) -> crate::env::DeltaEnv {
    let env = crate::env::DeltaEnv::init();
    crate::env::DeltaEnv {
        current_dir: Some(dir.to_path_buf()),
        // gitu sets DELTA_FEATURES for the deltas it runs only when it turns features on
        // or off; otherwise they take it from the environment, as this does.
        features: features.map(String::from).or(env.features),
        ..env
    }
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

/// What `git_config` sets each of `names` to, as delta takes its own options from git config
/// and the features in force, with `env`'s features (`DELTA_FEATURES`) in force too: `None` for
/// one not set. The names are of options gitu draws with and delta does not use, such as
/// `comment-style`; delta reads nothing but these values for them.
fn option_values(
    env: &crate::env::DeltaEnv,
    git_config: Option<GitConfig>,
    names: &[&str],
) -> Vec<Option<String>> {
    // gitu has the terminal, so delta must not ask it its background.
    let args = ["delta", "--detect-dark-light=never"];
    let mut opt = crate::cli::Opt::from_iter_and_git_config(env, args, git_config);
    let mut git_config = opt.git_config.take();
    let builtin_features = crate::features::make_builtin_features();
    names
        .iter()
        .map(|name| {
            crate::options::get::get_option_value::<String>(
                name,
                &builtin_features,
                &opt,
                &mut git_config,
            )
        })
        .collect()
}

/// The items `git_config` gives each of `names`, a list gitu takes from delta's config such as
/// `command`, lowest precedence first: those of each feature in force in turn, those
/// `DELTA_FEATURES` or the command line turns on after those git config does, then those of
/// `[delta]`. This is the order in which delta takes its own options, but simpler: it does not
/// give a builtin feature's section, such as `[delta "side-by-side"]`, precedence over other
/// features, as delta does for a single value.
fn option_lists(
    env: &crate::env::DeltaEnv,
    git_config: Option<GitConfig>,
    names: &[&str],
) -> Vec<Vec<String>> {
    // gitu has the terminal, so delta must not ask it its background.
    let args = ["delta", "--detect-dark-light=never"];
    let mut opt = crate::cli::Opt::from_iter_and_git_config(env, args, git_config);
    let Some(git_config) = opt.git_config.take() else {
        return vec![vec![]; names.len()];
    };
    let features: Vec<&str> = opt
        .features
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    let from_command_line =
        |feature: &&&str| opt.computed.command_line_features.contains(**feature);
    let sections: Vec<String> = features
        .iter()
        .filter(|feature| !from_command_line(feature))
        .chain(features.iter().filter(from_command_line))
        .map(|feature| format!("delta.{feature}"))
        .chain(["delta".to_owned()])
        .collect();
    names
        .iter()
        .map(|name| {
            let mut items = vec![];
            for section in &sections {
                let regex = format!("^{}\\.{}$", regex::escape(section), regex::escape(name));
                git_config.for_each(&regex, |_, value| items.extend(value.map(String::from)));
            }
            items
        })
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

    /// `delta tui` is `delta --tui`, as `delta git` and `delta rg` run git and rg: as delta's first
    /// argument only, so that a file named tui after another is a file to diff.
    #[test]
    fn test_tui_as_the_first_argument_is_the_tui() {
        assert!(is_tui(&"tui".into()));
        // As delta 0.22.0 ran it.
        assert!(is_tui(&"--tui".into()));
        assert!(is_tui(&"--gitu".into()));
        assert_eq!(misplaced_tui(os_args(&["delta", "tui"])), None);
        assert_eq!(misplaced_tui(os_args(&["delta", "a", "tui"])), None);
    }

    #[test]
    fn test_the_tuis_help_says_delta_tui() {
        let help = parse(os_args(&["--help"]))
            .unwrap_err()
            .render()
            .to_string();
        assert!(help.contains("Usage: delta tui"), "{help}");
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

    /// gitu is told whether delta draws side by side at a width, with features in force as
    /// `DELTA_FEATURES` gives them, so that difftastic's view is laid out as delta draws it.
    #[test]
    fn test_gitu_is_told_whether_delta_draws_side_by_side_at_a_width() {
        let cases = [
            ("[delta]\n    side-by-side = 120\n", None, 119, false),
            ("[delta]\n    side-by-side = 120\n", None, 120, true),
            (
                "[delta]\n    side-by-side = 120\n",
                Some("-side-by-side"),
                200,
                false,
            ),
            ("[delta]\n", Some("+side-by-side"), 80, true),
            ("[delta]\n", None, 200, false),
        ];
        for (git_config, features, columns, expected) in cases {
            assert_eq!(
                side_by_side_with(git_config, features, columns),
                expected,
                "{git_config:?} {features:?} {columns}"
            );
        }
    }

    /// Whether gitu is told delta draws side by side with `git_config` and `features` in force as
    /// `DELTA_FEATURES` gives them, at a width of `columns`.
    fn side_by_side_with(git_config: &str, features: Option<&str>, columns: usize) -> bool {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-side-by-side-{}-{:?}.gitconfig",
            std::process::id(),
            std::thread::current().id()
        ));
        let env = crate::env::DeltaEnv {
            features: features.map(String::from),
            columns: Some(columns.to_string()),
            ..crate::env::DeltaEnv::default()
        };
        let git_config = crate::tests::integration_test_utils::make_git_config(
            &env,
            git_config.as_bytes(),
            path.to_str().unwrap(),
            false,
        );
        let side_by_side = draws_side_by_side(&env, Some(git_config));
        fs::remove_file(path).unwrap();
        side_by_side
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

    /// What gitu is told `git_config` sets `names` to, with `features` in force as
    /// `DELTA_FEATURES` gives them.
    fn values_with(
        git_config: &str,
        features: Option<&str>,
        names: &[&str],
    ) -> Vec<Option<String>> {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-values-{}-{:?}.gitconfig",
            std::process::id(),
            std::thread::current().id()
        ));
        let env = crate::env::DeltaEnv {
            features: features.map(String::from),
            ..crate::env::DeltaEnv::default()
        };
        let git_config = crate::tests::integration_test_utils::make_git_config(
            &env,
            git_config.as_bytes(),
            path.to_str().unwrap(),
            false,
        );
        let values = option_values(&env, Some(git_config), names);
        fs::remove_file(path).unwrap();
        values
    }

    #[test]
    fn test_gitu_is_told_an_option_set_in_the_delta_section() {
        assert_eq!(
            values_with(
                "[delta]\n\tcomment-style = red bold\n",
                None,
                &["comment-style", "comment-author-style"],
            ),
            [Some("red bold".to_owned()), None]
        );
    }

    #[test]
    fn test_gitu_is_told_an_option_set_by_a_feature_git_config_enables() {
        let git_config =
            "[delta]\n\tfeatures = my-theme\n[delta \"my-theme\"]\n\tcomment-style = red\n";
        assert_eq!(
            values_with(git_config, None, &["comment-style"]),
            [Some("red".to_owned())]
        );
    }

    #[test]
    fn test_gitu_is_told_an_option_set_by_a_feature_it_turns_on() {
        let git_config = "[delta \"my-theme\"]\n\tcomment-style = red\n";
        assert_eq!(values_with(git_config, None, &["comment-style"]), [None]);
        assert_eq!(
            values_with(git_config, Some("+my-theme"), &["comment-style"]),
            [Some("red".to_owned())]
        );
    }

    /// The items gitu is told `git_config` gives the lists `names`, with `features` in force as
    /// `DELTA_FEATURES` gives them.
    fn lists_with(git_config: &str, features: Option<&str>, names: &[&str]) -> Vec<Vec<String>> {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-lists-{}-{:?}.gitconfig",
            std::process::id(),
            std::thread::current().id()
        ));
        let env = crate::env::DeltaEnv {
            features: features.map(String::from),
            ..crate::env::DeltaEnv::default()
        };
        let git_config = crate::tests::integration_test_utils::make_git_config(
            &env,
            git_config.as_bytes(),
            path.to_str().unwrap(),
            false,
        );
        let lists = option_lists(&env, Some(git_config), names);
        fs::remove_file(path).unwrap();
        lists
    }

    #[test]
    fn test_gitu_is_told_a_lists_items_in_the_delta_section_and_each_feature_in_force() {
        let git_config = "[delta]\n\tfeatures = a b\n\tcommand = x delta\n\
                          [delta \"a\"]\n\tcommand = x a\n\tcommand = y a\n\
                          [delta \"b\"]\n\tcommand = x b\n\
                          [delta \"off\"]\n\tcommand = x off\n";
        assert_eq!(
            lists_with(git_config, None, &["command", "view"]),
            [vec!["x a", "y a", "x b", "x delta"], vec![]]
        );
    }

    #[test]
    fn test_gitu_is_told_a_lists_items_in_a_feature_it_turns_on_after_git_configs() {
        let git_config = "[delta]\n\tfeatures = b\n\
                          [delta \"a\"]\n\tcommand = x a\n\
                          [delta \"b\"]\n\tcommand = x b\n";
        assert_eq!(
            lists_with(git_config, Some("+a"), &["command"]),
            [vec!["x b", "x a"]]
        );
    }

    /// gitu's settings are options in delta's config that delta does not use, so none can have
    /// the name of one delta does.
    #[test]
    fn test_no_setting_gitu_takes_from_deltas_config_is_a_delta_option() {
        let deltas: Vec<String> = crate::cli::Opt::command()
            .get_arguments()
            .filter_map(|argument| argument.get_long().map(String::from))
            .collect();
        let taken: Vec<String> = config::delta_option_names()
            .into_iter()
            .filter(|name| deltas.contains(name))
            .collect();
        assert!(taken.is_empty(), "{:?}", taken);
    }

    /// The TUI's config as gitu resolves it from `git_config` through delta's own resolution of
    /// options, with `features` in force as `DELTA_FEATURES` gives them.
    fn tui_config_with(git_config: &str, features: Option<&str>) -> Config {
        let path = std::env::temp_dir().join(format!(
            "delta-gitu-test-tui-config-{}-{:?}.gitconfig",
            std::process::id(),
            std::thread::current().id()
        ));
        crate::tests::integration_test_utils::make_git_config(
            &crate::env::DeltaEnv::default(),
            git_config.as_bytes(),
            path.to_str().unwrap(),
            false,
        );
        let env_with = |features: Option<&str>| crate::env::DeltaEnv {
            features: features.map(String::from),
            ..crate::env::DeltaEnv::default()
        };
        let git_config_at = |path: &std::path::Path| {
            Some(GitConfig::from_path(
                &crate::env::DeltaEnv::default(),
                path,
                false,
            ))
        };
        let mut config = configure("");
        let (values_path, lists_path) = (path.clone(), path.clone());
        config.general.diff_renderer.options = RendererOptions::new(
            move |_, features, names| {
                option_values(&env_with(features), git_config_at(&values_path), names)
            },
            move |_, features, names| {
                option_lists(&env_with(features), git_config_at(&lists_path), names)
            },
        );
        let resolved = config.resolved(&std::env::temp_dir(), features).unwrap();
        fs::remove_file(path).unwrap();
        resolved
    }

    /// A feature holding TUI settings: a setting, a style, a command and the command's key.
    const TUI_FEATURE: &str = "[delta \"tui\"]\n\
                               \tstart-folded = true\n\
                               \thash-style = red\n\
                               \tcommand = hello echo hello\n\
                               \tfeature-key = hello t\n";

    /// gitu's style types are its own, so the style is read from how it prints.
    fn is_red(style: &config::StyleConfigEntry) -> bool {
        format!("{style:?}").contains("fg: Some(Red)")
    }

    fn holds_the_tui_feature(config: &Config) -> bool {
        config.general.start_folded
            && is_red(&config.style.hash)
            && config.general.commands.get("hello").map(String::as_str) == Some("echo hello")
            && config.general.diff_renderer.feature_keys.get("hello") == Some(&'t')
    }

    #[test]
    fn test_the_tuis_settings_in_a_feature_git_config_enables_are_in_force() {
        let git_config = format!("[delta]\n\tfeatures = tui\n{TUI_FEATURE}");
        assert!(holds_the_tui_feature(&tui_config_with(&git_config, None)));
    }

    #[test]
    fn test_the_tuis_settings_in_a_feature_turned_on_by_delta_features_are_in_force() {
        assert!(holds_the_tui_feature(&tui_config_with(
            TUI_FEATURE,
            Some("+tui")
        )));
    }

    #[test]
    fn test_the_tuis_settings_in_a_feature_not_in_force_are_not() {
        let config = tui_config_with(TUI_FEATURE, None);
        assert!(!config.general.start_folded);
        assert!(!is_red(&config.style.hash));
        assert!(config.general.commands.is_empty());
        assert!(!config
            .general
            .diff_renderer
            .feature_keys
            .contains_key("hello"));
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
