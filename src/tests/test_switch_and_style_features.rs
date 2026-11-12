//! How delta combines a builtin feature turned on by an option of the same name with the user's
//! own settings.
//!
//! A switch feature (side-by-side, line-numbers, keep-plus-minus-markers, navigate, hyperlinks)
//! turns on one capability of delta; its other values are only defaults for how that capability
//! looks. A style feature (raw, color-only, diff-highlight, diff-so-fancy) is a set of values for
//! how delta looks.
//!
//! The requirements on delta's TUI (toggle keys, re-rendering on resize) are not tested here.

use std::sync::atomic::{AtomicUsize, Ordering};

use clap::CommandFactory;

use crate::ansi;
use crate::cli;
use crate::config::Config;
use crate::env::DeltaEnv;
use crate::tests::integration_test_utils;

const SWITCH_FEATURES: [&str; 5] = [
    "side-by-side",
    "line-numbers",
    "keep-plus-minus-markers",
    "navigate",
    "hyperlinks",
];

/// The user's own feature. It sets what the builtin side-by-side, line-numbers and navigate
/// features also set.
const USER_FEATURE: &str = r#"
[delta "mine"]
    line-numbers-left-format = "{nm:>1}L"
    line-numbers-right-format = "{np:>1}R"
    line-numbers-left-style = red
    line-numbers-right-style = red
    line-numbers-minus-style = red
    line-numbers-zero-style = red
    line-numbers-plus-style = red
    file-modified-label = M
    hunk-label = H
"#;

/// A theme that sets what each style feature sets.
const THEME: &str = r#"
[delta "t"]
    dark = true
    commit-style = red
    file-style = red
    file-decoration-style = blue box
    hunk-header-style = red
    hunk-header-decoration-style = blue box
    minus-style = red
    plus-style = green
"#;

const DIFF: &str = "\
commit 1111111111111111111111111111111111111111
Author: A <a@example.com>

    Change a

diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,3 +1,3 @@ fn f() {
 a
-b
+c
 d
";

/// git config with the user's feature turned on, and `extra` in the [delta] section.
fn user_config(extra: &str) -> String {
    format!("[delta]\n    features = mine\n{extra}{USER_FEATURE}")
}

/// delta's options for `args`, `DELTA_FEATURES` and `git_config`, or why delta rejects `args`.
fn options(
    args: &[&str],
    delta_features: Option<&str>,
    git_config: &str,
) -> Result<cli::Opt, String> {
    // Parse first, so that an argument delta rejects fails this case instead of exiting.
    let argv: Vec<&str> = ["/dev/null", "/dev/null"]
        .iter()
        .chain(args)
        .copied()
        .collect();
    cli::Opt::command()
        .try_get_matches_from(&argv)
        .map_err(|err| format!("delta rejects {}: {:?}", args.join(" "), err.kind()))?;

    static N: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "delta__test_switch_and_style_features_{}_{}.gitconfig",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let path = path.to_str().unwrap();
    let opt = integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
        DeltaEnv {
            columns: Some("100".to_string()),
            features: delta_features.map(str::to_string),
            ..DeltaEnv::default()
        },
        args,
        Some(git_config.as_bytes()),
        Some(path),
    );
    std::fs::remove_file(path).unwrap();
    Ok(opt)
}

/// delta's output for `args`, `DELTA_FEATURES` and `git_config`, or why delta rejects `args`.
fn output(args: &[&str], delta_features: Option<&str>, git_config: &str) -> Result<String, String> {
    options(args, delta_features, git_config)
        .map(|opt| integration_test_utils::run_delta(DIFF, &Config::from(opt)))
}

/// Why `actual` is not `expected`, or None if it is.
fn difference(
    expected: &Result<String, String>,
    actual: &Result<String, String>,
) -> Option<String> {
    match (expected, actual) {
        (Ok(expected), Ok(actual)) if expected == actual => None,
        (Ok(expected), Ok(actual)) => {
            let line = expected
                .lines()
                .zip(actual.lines())
                .find(|(e, a)| e != a)
                .map(|(e, a)| {
                    format!(
                        "expected line {:?}, got {:?}",
                        ansi::strip_ansi_codes(e),
                        ansi::strip_ansi_codes(a)
                    )
                })
                .unwrap_or_else(|| "the outputs differ in length".to_string());
            if ansi::strip_ansi_codes(expected) == ansi::strip_ansi_codes(actual) {
                return Some(format!("the outputs differ only in colour: {line}"));
            }
            Some(line)
        }
        (Err(err), _) | (_, Err(err)) => Some(err.clone()),
    }
}

fn assert_no_failures(failures: Vec<String>) {
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// A switch feature gives the same output however it is turned on: in [delta], in a feature,
/// `--name`, `DELTA_FEATURES=+name`, or added to the list in `--features`.
#[test]
fn a_switch_feature_gives_the_same_output_however_it_is_turned_on() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let expected = output(&[], None, &user_config(&format!("    {switch} = true\n")));
        let flag = format!("--{switch}");
        let added = format!("+{switch}");
        let features = format!("--features=mine {switch}");
        let in_feature = format!("{}    {switch} = true\n", user_config(""));
        let cases: [(&str, &[&str], Option<&str>, &str); 4] = [
            ("in a feature", &[], None, &in_feature),
            (&flag, &[&flag], None, &user_config("")),
            ("DELTA_FEATURES", &[], Some(&added), &user_config("")),
            (&features, &[&features], None, &user_config("")),
        ];
        for (case, args, delta_features, git_config) in cases {
            let actual = output(args, delta_features, git_config);
            if let Some(difference) = difference(&expected, &actual) {
                failures.push(format!(
                    "{switch} turned on {case} is not as {switch} = true in [delta]: {difference}"
                ));
            }
        }
    }
    assert_no_failures(failures);
}

/// `--name` is the same as adding `name` to the feature list, and `--name=false` the same as
/// removing it.
#[test]
fn a_switch_feature_flag_is_the_same_as_adding_or_removing_the_feature() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let on = format!("--{switch}");
        let added = format!("+{switch}");
        let expected = output(&[], Some(&added), &user_config(""));
        if let Some(difference) = difference(&expected, &output(&[&on], None, &user_config(""))) {
            failures.push(format!(
                "{on} is not as DELTA_FEATURES={added}: {difference}"
            ));
        }

        let off = format!("--{switch}=false");
        let removed = format!("+-{switch}");
        let configured = user_config(&format!("    {switch} = true\n"));
        let expected = output(&[], Some(&removed), &configured);
        if let Some(difference) = difference(&expected, &output(&[&off], None, &configured)) {
            failures.push(format!(
                "{off} is not as DELTA_FEATURES={removed}: {difference}"
            ));
        }
    }
    assert_no_failures(failures);
}

/// What a switch feature sets besides its own option are defaults: the user's own settings win,
/// however the switch feature was turned on.
#[test]
fn a_switch_feature_keeps_what_the_user_sets() {
    let mut failures = Vec::new();
    let mut check = |case: &str, opt: Result<cli::Opt, String>, field: &str, expected: &str| {
        let actual = opt.map(|opt| match field {
            "line-numbers-left-format" => opt.line_numbers_left_format,
            "line-numbers-right-format" => opt.line_numbers_right_format,
            "line-numbers-left-style" => opt.line_numbers_left_style,
            "file-modified-label" => opt.file_modified_label,
            "hunk-label" => opt.hunk_label,
            _ => unreachable!(),
        });
        match actual {
            Ok(actual) if actual == expected => {}
            Ok(actual) => failures.push(format!("{case}: {field} is {actual:?}, not {expected:?}")),
            Err(err) => failures.push(format!("{case}: {err}")),
        }
    };
    let ways = |switch: &str| -> [(String, Vec<String>, Option<String>, String); 3] {
        [
            (
                format!("{switch} = true in [delta]"),
                vec![],
                None,
                user_config(&format!("    {switch} = true\n")),
            ),
            (
                format!("--{switch}"),
                vec![format!("--{switch}")],
                None,
                user_config(""),
            ),
            (
                format!("DELTA_FEATURES=+{switch}"),
                vec![],
                Some(format!("+{switch}")),
                user_config(""),
            ),
        ]
    };
    let expectations: [(&str, &[(&str, &str)]); 3] = [
        (
            "side-by-side",
            &[
                ("line-numbers-left-format", "{nm:>1}L"),
                ("line-numbers-right-format", "{np:>1}R"),
                ("line-numbers-left-style", "red"),
            ],
        ),
        ("line-numbers", &[("line-numbers-left-style", "red")]),
        (
            "navigate",
            &[("file-modified-label", "M"), ("hunk-label", "H")],
        ),
    ];
    for (switch, fields) in expectations {
        for (case, args, delta_features, git_config) in ways(switch) {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            for (field, expected) in fields.iter() {
                check(
                    &case,
                    options(&args, delta_features.as_deref(), &git_config),
                    field,
                    expected,
                );
            }
        }
    }

    // A setting in [delta] wins too.
    let git_config = "[delta]\n    line-numbers-left-format = \"{nm:>1}D\"\n";
    check(
        "--side-by-side with line-numbers-left-format in [delta]",
        options(&["--side-by-side"], None, git_config),
        "line-numbers-left-format",
        "{nm:>1}D",
    );
    assert_no_failures(failures);
}

/// Line-number formats the user sets, other than in a section with the same name as side-by-side,
/// apply to both the unified and the side-by-side view.
#[test]
fn the_users_line_number_formats_apply_to_both_views() {
    let mut failures = Vec::new();
    let git_config = user_config("    line-numbers = true\n");
    let cases: [(&str, &[&str]); 3] = [
        ("unified", &[]),
        ("--side-by-side", &["--side-by-side"]),
        ("--side-by-side=false", &["--side-by-side=false"]),
    ];
    for (case, args) in cases {
        match options(args, None, &git_config) {
            Ok(opt) => {
                if (
                    opt.line_numbers_left_format.as_str(),
                    opt.line_numbers_right_format.as_str(),
                ) != ("{nm:>1}L", "{np:>1}R")
                {
                    failures.push(format!(
                        "{case}: formats are {:?} and {:?}",
                        opt.line_numbers_left_format, opt.line_numbers_right_format
                    ));
                }
            }
            Err(err) => failures.push(format!("{case}: {err}")),
        }
    }
    let side_by_side = user_config("    line-numbers = true\n    side-by-side = true\n");
    match options(&[], None, &side_by_side) {
        Ok(opt) if opt.line_numbers_left_format == "{nm:>1}L" => {}
        Ok(opt) => failures.push(format!(
            "side-by-side = true: format is {:?}",
            opt.line_numbers_left_format
        )),
        Err(err) => failures.push(err),
    }
    assert_no_failures(failures);
}

/// A section with the same name as a builtin feature adds to it: its settings apply whenever the
/// builtin is on, however it was turned on, and beat both the builtin's own settings and the
/// user's other features.
#[test]
fn a_section_with_the_same_name_as_a_builtin_applies_whenever_the_builtin_is_on() {
    let sections = r#"
[delta "side-by-side"]
    line-numbers-left-format = "{nm:>1}S"
[delta "navigate"]
    file-modified-label = N
"#;
    let mut failures = Vec::new();
    let mut check = |case: String, opt: Result<cli::Opt, String>, field: &str, expected: &str| {
        let actual = opt.map(|opt| match field {
            "line-numbers-left-format" => opt.line_numbers_left_format,
            "file-modified-label" => opt.file_modified_label,
            _ => unreachable!(),
        });
        match actual {
            Ok(actual) if actual == expected => {}
            Ok(actual) => failures.push(format!("{case}: {field} is {actual:?}, not {expected:?}")),
            Err(err) => failures.push(format!("{case}: {err}")),
        }
    };
    let cases: [(&str, &str, &str); 2] = [
        ("side-by-side", "line-numbers-left-format", "{nm:>1}S"),
        ("navigate", "file-modified-label", "N"),
    ];
    for (builtin, field, expected) in cases {
        let flag = format!("--{builtin}");
        let added = format!("+{builtin}");
        let features = format!("--features=mine {builtin}");
        let with_user_feature = |extra: &str| format!("{}{sections}", user_config(extra));
        let ways: [(String, Vec<&str>, Option<&str>, String); 5] = [
            (
                format!("{builtin} = true in [delta]"),
                vec![],
                None,
                with_user_feature(&format!("    {builtin} = true\n")),
            ),
            (
                format!("{builtin} = true in a feature"),
                vec![],
                None,
                format!("{}    {builtin} = true\n{sections}", user_config("")),
            ),
            (flag.clone(), vec![&flag], None, with_user_feature("")),
            (
                format!("DELTA_FEATURES={added}"),
                vec![],
                Some(&added),
                with_user_feature(""),
            ),
            (
                features.clone(),
                vec![&features],
                None,
                with_user_feature(""),
            ),
        ];
        for (case, args, delta_features, git_config) in ways {
            check(
                format!("{case}, with the user's other feature"),
                options(&args, delta_features, &git_config),
                field,
                expected,
            );
        }
        // Without the user's other feature, the section beats the builtin's own settings.
        check(
            format!("{flag}, without the user's other feature"),
            options(&[&flag], None, &format!("[delta]\n{sections}")),
            field,
            expected,
        );
    }

    // When the builtin is off, the section does not apply.
    let unified = format!("{}{sections}", user_config("    line-numbers = true\n"));
    check(
        "unified".to_string(),
        options(&[], None, &unified),
        "line-numbers-left-format",
        "{nm:>1}L",
    );
    check(
        "navigate off".to_string(),
        options(&[], None, &unified),
        "file-modified-label",
        "M",
    );

    // A setting in [delta] itself still beats every feature.
    let in_delta = format!(
        "[delta]\n    side-by-side = true\n    line-numbers-left-format = \"{{nm:>1}}D\"\n{sections}"
    );
    check(
        "line-numbers-left-format in [delta]".to_string(),
        options(&[], None, &in_delta),
        "line-numbers-left-format",
        "{nm:>1}D",
    );
    assert_no_failures(failures);
}

/// Turning a switch feature off removes everything it brought in: the output is as if the switch
/// feature had never been turned on.
#[test]
fn turning_a_switch_feature_off_removes_everything_it_brought_in() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let expected = output(&[], None, &user_config(""));
        let in_delta = user_config(&format!("    {switch} = true\n"));
        let in_feature = format!("{}    {switch} = true\n", user_config(""));
        let off = format!("--{switch}=false");
        let removed = format!("+-{switch}");
        for (on, git_config) in [("in [delta]", &in_delta), ("in a feature", &in_feature)] {
            let offs: [(&str, &[&str], Option<&str>); 2] = [
                (&off, &[&off], None),
                ("DELTA_FEATURES", &[], Some(&removed)),
            ];
            for (how, args, delta_features) in offs {
                let actual = output(args, delta_features, git_config);
                if let Some(difference) = difference(&expected, &actual) {
                    failures.push(format!(
                        "{switch} turned on {on} and off by {how} leaves something: {difference}"
                    ));
                }
            }
        }
    }
    assert_no_failures(failures);
}

/// A switch feature turned off in [delta] is off however a feature turns it on, and brings in
/// nothing: [delta] has priority over every feature.
#[test]
fn a_switch_feature_turned_off_in_the_delta_section_brings_in_nothing() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        // With the user's feature, which sets some of what the switch feature sets, and without.
        let with_user_feature = (
            user_config(""),
            format!("[delta]\n    features = mine {switch}\n    {switch} = false\n{USER_FEATURE}"),
            format!(
                "{}    {switch} = true\n",
                user_config(&format!("    {switch} = false\n"))
            ),
        );
        let without = (
            "[delta]\n".to_string(),
            format!("[delta]\n    features = {switch}\n    {switch} = false\n"),
            format!(
                "[delta]\n    features = other\n    {switch} = false\n[delta \"other\"]\n    {switch} = true\n"
            ),
        );
        for (base, listed, in_feature) in [with_user_feature, without] {
            let expected = output(&[], None, &base);
            for (on, git_config) in [
                ("listed in features", &listed),
                ("in a feature", &in_feature),
            ] {
                let actual = output(&[], None, git_config);
                if let Some(difference) = difference(&expected, &actual) {
                    failures.push(format!(
                        "{switch} turned on {on} and off in [delta] leaves something: {difference}"
                    ));
                }
            }
        }
    }
    assert_no_failures(failures);
}

/// The command line beats git config: `--name=false` turns off a switch feature git config turns
/// on.
#[test]
fn a_switch_feature_turned_off_on_the_command_line_beats_git_config() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let off = format!("--{switch}=false");
        match options(
            &[&off],
            None,
            &user_config(&format!("    {switch} = true\n")),
        ) {
            Ok(opt) => {
                let on = match switch {
                    "side-by-side" => opt.computed.side_by_side,
                    "line-numbers" => opt.line_numbers,
                    "keep-plus-minus-markers" => opt.keep_plus_minus_markers,
                    "navigate" => opt.navigate,
                    "hyperlinks" => opt.hyperlinks,
                    _ => unreachable!(),
                };
                if on {
                    failures.push(format!("{off}: {switch} is still on"));
                }
            }
            Err(err) => failures.push(err),
        }
    }
    assert_no_failures(failures);
}

/// The command line beats git config: a style feature asked for on the command line beats the
/// user's theme.
#[test]
fn a_style_feature_on_the_command_line_beats_the_users_theme() {
    type Field = (&'static str, fn(&cli::Opt) -> &str);
    let commit: Field = ("commit-style", |opt| &opt.commit_style);
    let file: Field = ("file-style", |opt| &opt.file_style);
    let file_decoration: Field = ("file-decoration-style", |opt| &opt.file_decoration_style);
    let hunk_header: Field = ("hunk-header-style", |opt| &opt.hunk_header_style);
    let minus: Field = ("minus-style", |opt| &opt.minus_style);
    let plus: Field = ("plus-style", |opt| &opt.plus_style);
    let styles: [(&str, &[Field]); 4] = [
        (
            "raw",
            &[commit, file, file_decoration, hunk_header, minus, plus],
        ),
        // color-only is raw without raw's minus and plus styles.
        ("color-only", &[commit, file, file_decoration, hunk_header]),
        // diff-highlight is raw with its own minus and plus styles, and diff-so-fancy is
        // diff-highlight with its own file and hunk header styles.
        (
            "diff-highlight",
            &[commit, file, file_decoration, hunk_header, minus, plus],
        ),
        (
            "diff-so-fancy",
            &[commit, file, file_decoration, hunk_header, minus, plus],
        ),
    ];
    let mut failures = Vec::new();
    let themed = format!("[delta]\n    features = t\n{THEME}");
    for (style, fields) in styles {
        let flag = format!("--{style}");
        match (
            options(&[&flag], None, "[delta]\n"),
            options(&[&flag], None, &themed),
        ) {
            (Ok(expected), Ok(actual)) => {
                for (field, value) in fields.iter() {
                    if value(&expected) != value(&actual) {
                        failures.push(format!(
                            "{flag} with a theme: {field} is {:?}, not {:?}",
                            value(&actual),
                            value(&expected)
                        ));
                    }
                }
            }
            (Err(err), _) | (_, Err(err)) => failures.push(err),
        }
    }
    assert_no_failures(failures);
}

/// The command line beats git config: a feature added on the command line beats the features in
/// git config.
#[test]
fn a_feature_added_on_the_command_line_beats_the_features_in_git_config() {
    let git_config = format!(
        "{}[delta \"other\"]\n    file-modified-label = O\n",
        user_config("")
    );
    let opt = options(&[], Some("+other"), &git_config).unwrap();
    assert_eq!(opt.file_modified_label, "O");
}

/// Backward compatibility: a switch feature's builtin defaults are used where the user sets
/// nothing, however the switch feature is turned on.
#[test]
fn a_switch_feature_uses_its_builtin_defaults_where_the_user_sets_nothing() {
    let mut failures = Vec::new();
    let cases: [(&str, &[&str], &str); 4] = [
        (
            "side-by-side = true",
            &[],
            "[delta]\n    side-by-side = true\n",
        ),
        ("--side-by-side", &["--side-by-side"], "[delta]\n"),
        ("navigate = true", &[], "[delta]\n    navigate = true\n"),
        ("--navigate", &["--navigate"], "[delta]\n"),
    ];
    for (case, args, git_config) in cases {
        match options(args, None, git_config) {
            Ok(opt) if case.contains("side-by-side") => {
                if opt.line_numbers_left_format != "│{nm:^4}│" {
                    failures.push(format!(
                        "{case}: format is {:?}",
                        opt.line_numbers_left_format
                    ));
                }
            }
            Ok(opt) => {
                if opt.file_modified_label != "Δ" {
                    failures.push(format!("{case}: label is {:?}", opt.file_modified_label));
                }
            }
            Err(err) => failures.push(err),
        }
    }
    assert_no_failures(failures);
}

/// Backward compatibility: in git config, a theme beats a style feature set in [delta].
#[test]
fn in_git_config_a_theme_beats_a_style_feature_set_in_the_delta_section() {
    let mut failures = Vec::new();
    for style in ["raw", "diff-so-fancy"] {
        let git_config = format!("[delta]\n    features = t\n    {style} = true\n{THEME}");
        let opt = options(&[], None, &git_config).unwrap();
        if opt.file_decoration_style != "blue box" {
            failures.push(format!(
                "{style} = true: file-decoration-style is {:?}",
                opt.file_decoration_style
            ));
        }
    }
    assert_no_failures(failures);
}

/// Backward compatibility: a bare switch feature flag means on and does not take the files after
/// it, and `-n` still combines with `-s`.
#[test]
fn a_bare_switch_feature_flag_means_on_and_leaves_the_files_after_it() {
    let mut failures = Vec::new();
    for flag in [
        "-n",
        "--line-numbers",
        "--keep-plus-minus-markers",
        "--navigate",
        "--hyperlinks",
        "-s",
        "--side-by-side",
    ] {
        let opt = cli::Opt::from_iter_and_git_config(
            &DeltaEnv::default(),
            ["delta", "--no-gitconfig", flag, "a", "b"],
            None,
        );
        let on = match flag {
            "-n" | "--line-numbers" => opt.line_numbers,
            "--keep-plus-minus-markers" => opt.keep_plus_minus_markers,
            "--navigate" => opt.navigate,
            "--hyperlinks" => opt.hyperlinks,
            _ => opt.computed.side_by_side,
        };
        if !on {
            failures.push(format!("{flag} is not on"));
        }
        if (opt.minus_file.as_deref(), opt.plus_file.as_deref())
            != (Some("a".as_ref()), Some("b".as_ref()))
        {
            failures.push(format!("{flag} takes a file"));
        }
    }
    for cluster in ["-ns", "-sn"] {
        let opt = cli::Opt::from_iter_and_git_config(
            &DeltaEnv::default(),
            ["delta", "--no-gitconfig", cluster],
            None,
        );
        if !(opt.line_numbers && opt.computed.side_by_side) {
            failures.push(format!("{cluster} is not -n -s"));
        }
    }
    assert_no_failures(failures);
}

/// The command line beats DELTA_NAVIGATE: `--navigate=false` turns navigate off.
#[test]
fn navigate_turned_off_on_the_command_line_beats_delta_navigate() {
    let opt = integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
        DeltaEnv {
            navigate: Some("1".to_string()),
            ..DeltaEnv::default()
        },
        &["--navigate=false"],
        None,
        None,
    );
    assert!(!opt.navigate);
}

/// `--name` is the same as adding `name` to the feature list, so both beat `name = false` in
/// [delta].
#[test]
fn a_switch_feature_added_to_the_feature_list_beats_it_turned_off_in_git_config() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let git_config = user_config(&format!("    {switch} = false\n"));
        let flag = format!("--{switch}");
        let added = format!("+{switch}");
        let features = format!("--features=mine {switch}");
        let expected = output(&[&flag], None, &git_config);
        let cases: [(&str, &[&str], Option<&str>); 2] = [
            ("DELTA_FEATURES", &[], Some(&added)),
            (&features, &[&features], None),
        ];
        for (case, args, delta_features) in cases {
            let actual = output(args, delta_features, &git_config);
            if let Some(difference) = difference(&expected, &actual) {
                failures.push(format!(
                    "{switch} added by {case} is not as {flag}, with {switch} = false in [delta]: {difference}"
                ));
            }
        }
    }
    assert_no_failures(failures);
}

/// When the sections with the names of two builtin features in effect set the same option, the
/// result does not change between runs: side-by-side's section beats the section of the
/// line-numbers that side-by-side brings in.
#[test]
fn two_sections_with_builtin_names_give_the_same_result_on_each_run() {
    let git_config = format!(
        "{}[delta \"line-numbers\"]\n    line-numbers-left-format = L\n[delta \"side-by-side\"]\n    line-numbers-left-format = S\n",
        user_config("    side-by-side = true\n    line-numbers = true\n")
    );
    let formats: std::collections::BTreeSet<String> = (0..20)
        .map(|_| {
            options(&[], None, &git_config)
                .unwrap()
                .line_numbers_left_format
        })
        .collect();
    assert_eq!(
        formats,
        ["S".to_string()].into(),
        "line-numbers-left-format over 20 runs"
    );
}

/// A switch feature that a feature or theme of the user's turns off is not in effect, and brings
/// in nothing.
#[test]
fn a_switch_feature_turned_off_by_a_users_feature_brings_in_nothing() {
    let mut failures = Vec::new();
    for switch in SWITCH_FEATURES {
        let expected = output(&[], None, "[delta]\n");
        let git_config = format!(
            "[delta]\n    features = {switch} off\n[delta \"off\"]\n    {switch} = false\n"
        );
        let actual = output(&[], None, &git_config);
        if let Some(difference) = difference(&expected, &actual) {
            failures.push(format!(
                "{switch} listed and turned off by a later feature leaves something: {difference}"
            ));
        }
    }
    assert_no_failures(failures);
}

/// A switch feature's own option keeps the priority of where it is set; only the other values
/// of the switch feature are defaults. Of two features, the later in the list has priority.
#[test]
fn a_switch_features_own_option_keeps_its_place_in_the_feature_list() {
    let mut failures = Vec::new();
    for (switch, on) in [
        (
            "line-numbers",
            (|opt: &cli::Opt| opt.line_numbers) as fn(&cli::Opt) -> bool,
        ),
        ("keep-plus-minus-markers", |opt: &cli::Opt| {
            opt.keep_plus_minus_markers
        }),
    ] {
        let off = format!("[delta \"off\"]\n    {switch} = false\n");
        for (list, expected) in [
            (format!("off {switch}"), true),
            (format!("{switch} off"), false),
        ] {
            let git_config = format!("[delta]\n    features = {list}\n{off}");
            match options(&[], None, &git_config) {
                Ok(opt) if on(&opt) == expected => {}
                Ok(_) => failures.push(format!("features = {list}: {switch} is not {expected}")),
                Err(err) => failures.push(err),
            }
        }
    }
    assert_no_failures(failures);
}

/// The command line beats a section with the name of a builtin feature: that section beats only
/// the features in git config.
#[test]
fn the_command_line_beats_a_section_with_the_name_of_a_builtin_feature() {
    let mut failures = Vec::new();
    let git_config =
        "[delta]\n    line-numbers = true\n[delta \"line-numbers\"]\n    file-style = blue box\n";
    match options(&["--raw"], None, git_config) {
        Ok(opt) if opt.file_style == "raw" => {}
        Ok(opt) => failures.push(format!("--raw: file-style is {:?}", opt.file_style)),
        Err(err) => failures.push(err),
    }
    let git_config = "[delta]\n    navigate = true\n[delta \"navigate\"]\n    file-modified-label = S\n[delta \"o\"]\n    file-modified-label = O\n";
    match options(&[], Some("+o"), git_config) {
        Ok(opt) if opt.file_modified_label == "O" => {}
        Ok(opt) => failures.push(format!(
            "DELTA_FEATURES=+o: file-modified-label is {:?}",
            opt.file_modified_label
        )),
        Err(err) => failures.push(err),
    }
    assert_no_failures(failures);
}

/// A section with the name of a builtin feature adds to it, including the features it lists, however
/// the builtin feature is turned on.
#[test]
fn a_section_with_the_name_of_a_builtin_feature_brings_in_its_features_however_it_is_turned_on() {
    let mut failures = Vec::new();
    let sections =
        "[delta \"navigate\"]\n    features = x\n[delta \"line-numbers\"]\n    features = y\n[delta \"x\"]\n    file-modified-label = X\n[delta \"y\"]\n    line-numbers-left-style = green\n";
    let label: fn(&cli::Opt) -> &str = |opt| &opt.file_modified_label;
    let style: fn(&cli::Opt) -> &str = |opt| &opt.line_numbers_left_style;
    // A case: its name, the arguments, DELTA_FEATURES, what [delta] adds, the value to check,
    // and what it should be.
    type Case<'a> = (
        &'a str,
        &'a [&'a str],
        Option<&'a str>,
        &'a str,
        fn(&cli::Opt) -> &str,
        &'a str,
    );
    let cases: [Case; 5] = [
        ("--navigate", &["--navigate"], None, "", label, "X"),
        (
            "--features=navigate",
            &["--features=navigate"],
            None,
            "",
            label,
            "X",
        ),
        (
            "DELTA_FEATURES=+navigate",
            &[],
            Some("+navigate"),
            "",
            label,
            "X",
        ),
        (
            "navigate = true",
            &[],
            None,
            "    navigate = true\n",
            label,
            "X",
        ),
        (
            "line-numbers brought in by side-by-side",
            &[],
            None,
            "    side-by-side = true\n",
            style,
            "green",
        ),
    ];
    for (case, args, delta_features, in_delta, value, expected) in cases {
        let git_config = format!("[delta]\n{in_delta}{sections}");
        match options(args, delta_features, &git_config) {
            Ok(opt) if value(&opt) == expected => {}
            Ok(opt) => failures.push(format!("{case}: {:?}, not {expected:?}", value(&opt))),
            Err(err) => failures.push(err),
        }
    }
    assert_no_failures(failures);
}

/// With side-by-side = hunk, the hunks drawn in the unified layout use the formats of
/// [delta "line-numbers"] when line-numbers is in effect, as side-by-side brings it in.
#[test]
fn unified_hunks_use_the_line_numbers_section_when_side_by_side_brings_in_line_numbers() {
    let git_config = "[delta]\n    side-by-side = hunk\n[delta \"line-numbers\"]\n    line-numbers-left-format = L{nm}\n";
    let opt = options(&[], None, git_config).unwrap();
    assert_eq!(opt.computed.unified_line_numbers_left_format, "L{nm}");
}

/// A builtin feature named in a features list in git config is in effect, as with its flag: a
/// style feature does not set an option of its own name, and is not left out for that.
#[test]
fn a_builtin_feature_named_in_a_features_list_is_in_effect() {
    let mut failures = Vec::new();
    for feature in [
        "raw",
        "color-only",
        "diff-highlight",
        "diff-so-fancy",
        "side-by-side",
        "line-numbers",
        "keep-plus-minus-markers",
        "navigate",
        "hyperlinks",
    ] {
        let flag = format!("--{feature}");
        let expected = output(&[&flag], None, "[delta]\n");
        let actual = output(&[], None, &format!("[delta]\n    features = {feature}\n"));
        if let Some(difference) = difference(&expected, &actual) {
            failures.push(format!(
                "features = {feature} is not as {flag}: {difference}"
            ));
        }
    }
    assert_no_failures(failures);
}
