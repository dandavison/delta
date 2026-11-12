//! `dark-features` and `light-features` in [delta] list features that apply only when delta renders
//! for a dark or a light background. delta decides the mode as it always has, from everything but
//! these two lists, and a feature in them cannot change the mode that selected it.

use crate::cli;
use crate::color::ColorMode;
use crate::env::DeltaEnv;
use crate::tests::integration_test_utils;

/// Each per-mode feature sets `plus-style` to its own name, to show which applied.
const PER_MODE: &str = r#"
    dark-features = my-dark
    light-features = my-light
[delta "my-dark"]
    plus-style = my-dark
[delta "my-light"]
    plus-style = my-light
"#;

fn options(args: &[&str], delta_features: Option<&str>, git_config: &str) -> cli::Opt {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "delta__test_per_mode_features_{}_{}.gitconfig",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let path = path.to_str().unwrap();
    let opt = integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
        DeltaEnv {
            features: delta_features.map(str::to_string),
            ..DeltaEnv::default()
        },
        args,
        Some(git_config.as_bytes()),
        Some(path),
    );
    std::fs::remove_file(path).unwrap();
    opt
}

/// git config with `extra` in the [delta] section, before the per-mode lists.
fn with_per_mode(extra: &str) -> String {
    format!("[delta]\n{extra}{PER_MODE}")
}

#[test]
fn the_features_for_the_mode_apply() {
    let cases: [(&str, &[&str], &str, &str); 6] = [
        ("--dark", &["--dark"], "", "my-dark"),
        ("--light", &["--light"], "", "my-light"),
        ("dark = true", &[], "    dark = true\n", "my-dark"),
        ("light = true", &[], "    light = true\n", "my-light"),
        (
            "a light syntax theme",
            &[],
            "    syntax-theme = GitHub\n",
            "my-light",
        ),
        // In tests delta cannot ask the terminal, and with nothing else to go on it renders for
        // a dark background.
        ("nothing", &[], "", "my-dark"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(case, args, extra, expected)| {
            let opt = options(args, None, &with_per_mode(extra));
            (opt.plus_style != *expected).then(|| {
                format!(
                    "{case}: plus-style is {:?}, not {expected:?}",
                    opt.plus_style
                )
            })
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// A per-mode feature may set `light`, `dark` or a syntax theme for the other mode; the mode that
/// selected it holds, and its syntax theme is used.
#[test]
fn a_per_mode_feature_cannot_change_the_mode() {
    let git_config = r#"
[delta]
    features = base
    dark-features = d
    light-features = l
[delta "base"]
    syntax-theme = Nord
[delta "d"]
    light = true
    syntax-theme = GitHub
    plus-style = d
[delta "l"]
    plus-style = l
"#;
    let opt = options(&[], None, git_config);
    assert_eq!(opt.computed.color_mode, ColorMode::Dark);
    assert_eq!(opt.plus_style, "d");
    assert_eq!(opt.syntax_theme.as_deref(), Some("GitHub"));
}

#[test]
fn the_features_for_the_mode_rank_above_features() {
    let git_config = format!(
        "{}[delta \"base\"]\n    plus-style = base\n",
        with_per_mode("    features = base\n")
    );
    assert_eq!(
        options(&["--dark"], None, &git_config).plus_style,
        "my-dark"
    );

    // A value in [delta] itself still beats every feature.
    let git_config = with_per_mode("    plus-style = delta\n");
    assert_eq!(options(&["--dark"], None, &git_config).plus_style, "delta");
}

#[test]
fn the_command_line_beats_the_features_for_the_mode() {
    let git_config = format!(
        "{}[delta \"cli\"]\n    plus-style = cli\n",
        with_per_mode("")
    );
    let opt = options(&["--dark"], Some("+cli"), &git_config);
    assert_eq!(opt.plus_style, "cli");

    // A style feature asked for on the command line beats them, as it beats a theme.
    let git_config = format!(
        "{}    file-style = red\n    file-decoration-style = blue box\n",
        with_per_mode("")
    );
    let opt = options(&["--dark", "--raw"], None, &git_config);
    assert_eq!(opt.file_style, "raw");
    assert_eq!(opt.file_decoration_style, "none");
}

/// `--features`, and DELTA_FEATURES without `+`, replace the features in git config, including
/// these; DELTA_FEATURES with `+` adds to them.
#[test]
fn features_on_the_command_line_replace_the_features_for_the_mode() {
    let git_config = format!("{}[delta \"x\"]\n    zero-style = x\n", with_per_mode(""));
    let cases: [(&str, &[&str], Option<&str>, &str); 3] = [
        (
            "--features=x",
            &["--dark", "--features=x"],
            None,
            "syntax auto",
        ),
        ("DELTA_FEATURES=x", &["--dark"], Some("x"), "syntax auto"),
        ("DELTA_FEATURES=+x", &["--dark"], Some("+x"), "my-dark"),
    ];
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(case, args, delta_features, expected)| {
            let opt = options(args, *delta_features, &git_config);
            (opt.plus_style != *expected).then(|| {
                format!(
                    "{case}: plus-style is {:?}, not {expected:?}",
                    opt.plus_style
                )
            })
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn a_feature_for_the_mode_can_be_removed_on_the_command_line() {
    let opt = options(&["--dark"], Some("+-my-dark"), &with_per_mode(""));
    assert_eq!(opt.plus_style, "syntax auto");
}

#[test]
fn a_builtin_feature_can_be_a_feature_for_the_mode() {
    let git_config = "[delta]\n    dark-features = side-by-side\n";
    assert!(options(&["--dark"], None, git_config).computed.side_by_side);
    assert!(
        !options(&["--light"], None, git_config)
            .computed
            .side_by_side
    );
}
