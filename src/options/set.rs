use std::cmp::min;
use std::collections::{HashMap, HashSet, VecDeque};
use std::process;

use console::Term;
use structopt::clap;

use crate::bat::assets::HighlightingAssets;
use crate::bat::output::PagingMode;
use crate::cli;
use crate::config;
use crate::env;
use crate::features;
use crate::git_config;
use crate::options::option_value::{OptionValue, ProvenancedOptionValue};
use crate::options::theme;

macro_rules! set_options {
	([$( ($option_name:expr, $field_ident:ident) ),* ],
     $opt:expr, $builtin_features:expr, $git_config:expr, $arg_matches:expr, $check_names:expr) => {
        let mut option_names = HashSet::new();
        $(
            if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                if let Some(value) = $crate::options::get::get_option_value(
                    $option_name,
                    &$builtin_features,
                    $opt,
                    $git_config
                ) {
                    $opt.$field_ident = value;
                }
            }
            if $check_names {
                option_names.insert($option_name);
            }
        )*
        if $check_names {
            option_names.extend(&[
                "diff-highlight", // Does not exist as a flag on config
                "diff-so-fancy", // Does not exist as a flag on config
                "features",  // Processed differently
                // Set prior to the rest
                "no-gitconfig",
                "dark",
                "light",
                "syntax-theme",
            ]);
            let expected_option_names = $crate::cli::Opt::get_option_or_flag_names();
            if option_names != expected_option_names {
                $crate::config::delta_unreachable(
                    &format!("Error processing options.\nUnhandled names: {:?}\nInvalid names: {:?}.\n",
                             &expected_option_names - &option_names,
                             &option_names - &expected_option_names));
            }
        }
	}
}

pub fn set_options(
    opt: &mut cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
    arg_matches: &clap::ArgMatches,
    assets: HighlightingAssets,
) {
    if let Some(git_config) = git_config {
        if opt.no_gitconfig {
            git_config.enabled = false;
        }
    }

    set_paging_mode(opt);
    set_widths(opt);

    // Set light, dark, and syntax-theme.
    set_true_color(opt);
    set__light__dark__syntax_theme__options(opt, git_config, arg_matches);
    theme::set__is_light_mode__syntax_theme__syntax_set(opt, assets);

    let builtin_features = features::make_builtin_features();
    // Set features
    let features = gather_features(opt, &builtin_features, git_config);
    opt.features = features.join(" ");

    // HACK: make minus-line styles have syntax-highlighting iff side-by-side.
    if features.contains(&"side-by-side".to_string()) {
        let prefix = "normal ";
        if !config::user_supplied_option("minus-style", arg_matches) {
            if opt.minus_style.starts_with(prefix) {
                opt.minus_style = format!("syntax {}", &opt.minus_style[prefix.len()..]);
            }
        }
        if !config::user_supplied_option("minus-emph-style", arg_matches) {
            if opt.minus_emph_style.starts_with(prefix) {
                opt.minus_emph_style = format!("syntax {}", &opt.minus_emph_style[prefix.len()..]);
            }
        }
    }

    // Handle options which default to an arbitrary git config value.
    // TODO: incorporate this logic into the set_options macro.
    if !config::user_supplied_option("whitespace-error-style", arg_matches) {
        opt.whitespace_error_style = if let Some(git_config) = git_config {
            git_config.get::<String>("color.diff.whitespace")
        } else {
            None
        }
        .unwrap_or_else(|| "magenta reverse".to_string())
    }

    set_options!(
        [
            ("24-bit-color", true_color),
            ("color-only", color_only),
            ("commit-decoration-style", commit_decoration_style),
            ("commit-style", commit_style),
            ("file-added-label", file_added_label),
            ("file-decoration-style", file_decoration_style),
            ("file-modified-label", file_modified_label),
            ("file-removed-label", file_removed_label),
            ("file-renamed-label", file_renamed_label),
            ("file-style", file_style),
            ("hunk-header-decoration-style", hunk_header_decoration_style),
            ("hunk-header-style", hunk_header_style),
            ("keep-plus-minus-markers", keep_plus_minus_markers),
            ("max-line-distance", max_line_distance),
            // Hack: minus-style must come before minus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("minus-style", minus_style),
            ("minus-emph-style", minus_emph_style),
            (
                "minus-empty-line-marker-style",
                minus_empty_line_marker_style
            ),
            ("minus-non-emph-style", minus_non_emph_style),
            ("minus-non-emph-style", minus_non_emph_style),
            ("navigate", navigate),
            ("line-numbers", line_numbers),
            ("line-numbers-left-format", line_numbers_left_format),
            ("line-numbers-left-style", line_numbers_left_style),
            ("line-numbers-minus-style", line_numbers_minus_style),
            ("line-numbers-plus-style", line_numbers_plus_style),
            ("line-numbers-right-format", line_numbers_right_format),
            ("line-numbers-right-style", line_numbers_right_style),
            ("line-numbers-zero-style", line_numbers_zero_style),
            ("paging", paging_mode),
            // Hack: plus-style must come before plus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("plus-style", plus_style),
            ("plus-emph-style", plus_emph_style),
            ("plus-empty-line-marker-style", plus_empty_line_marker_style),
            ("plus-non-emph-style", plus_non_emph_style),
            ("raw", raw),
            ("side-by-side", side_by_side),
            ("tabs", tab_width),
            ("whitespace-error-style", whitespace_error_style),
            ("width", width),
            ("word-diff-regex", tokenization_regex),
            ("zero-style", zero_style)
        ],
        opt,
        builtin_features,
        git_config,
        arg_matches,
        true
    );
}

#[allow(non_snake_case)]
fn set__light__dark__syntax_theme__options(
    opt: &mut cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
    arg_matches: &clap::ArgMatches,
) {
    let validate_light_and_dark = |opt: &cli::Opt| {
        if opt.light && opt.dark {
            eprintln!("--light and --dark cannot be used together.");
            process::exit(1);
        }
    };
    let empty_builtin_features = HashMap::new();
    validate_light_and_dark(&opt);
    if !(opt.light || opt.dark) {
        set_options!(
            [("dark", dark), ("light", light)],
            opt,
            &empty_builtin_features,
            git_config,
            arg_matches,
            false
        );
    }
    validate_light_and_dark(&opt);
    set_options!(
        [("syntax-theme", syntax_theme)],
        opt,
        &empty_builtin_features,
        git_config,
        arg_matches,
        false
    );
}

/// Features are processed differently from all other options. The role of this function is to
/// collect all configuration related to features and summarize it as a single list
/// (space-separated string) of enabled features. The list is arranged in order of increasing
/// priority in the sense that, when searching for a option value, one starts at the right-hand end
/// and moves leftward, examining each feature in turn until a feature that associates a value with
/// the option name is encountered. This search is documented in
/// `get_option_value::get_option_value`.
///
/// The feature list comprises features deriving from the following sources, listed in order of
/// decreasing priority:
///
/// 1. Suppose the command-line has `--features "a b"`. Then
///    - `b`, followed by b's "ordered descendents"
///    - `a`, followed by a's "ordered descendents"
///
/// 2. Suppose the command line enables two builtin features via `--navigate --diff-so-fancy`. Then
///    - `diff-so-fancy`
///    - `navigate`
///
/// 3. Suppose the main [delta] section has `features = d e`. Then
///    - `e`, followed by e's "ordered descendents"
///    - `d`, followed by d's "ordered descendents"
///
/// 4. Suppose the main [delta] section has `diff-highlight = true` followed by `raw = true`.
///    Then
///    - `diff-highlight`
///    - `raw`
///
/// The "ordered descendents" of a feature `f` is a list of features obtained via a pre-order
/// traversal of the feature tree rooted at `f`. This tree arises because it is allowed for a
/// feature to contain a (key, value) pair that itself enables features.
///
/// If a feature has already been included at higher priority, and is encountered again, it is
/// ignored.
///
/// Thus, for example:
///
/// delta --features "my-navigate-settings" --navigate   =>   "navigate my-navigate-settings"
///
/// In the following configuration, the feature names indicate their priority, with `a` having
/// highest priority:
///
/// delta --g --features "d a"
///
/// [delta "a"]
///     features = c b
///
/// [delta "d"]
///     features = f e
fn gather_features<'a>(
    opt: &cli::Opt,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    git_config: &Option<git_config::GitConfig>,
) -> Vec<String> {
    let mut features = VecDeque::new();

    // Gather features from command line.
    if let Some(git_config) = git_config {
        for feature in split_feature_string(&opt.features.to_lowercase()) {
            gather_features_recursively(feature, &mut features, &builtin_features, opt, git_config);
        }
    } else {
        for feature in split_feature_string(&opt.features.to_lowercase()) {
            features.push_front(feature.to_string());
        }
    }

    // Gather builtin feature flags supplied on command line.
    // TODO: Iterate over programatically-obtained names of builtin features.
    if opt.raw {
        gather_builtin_features_recursively("raw", &mut features, &builtin_features, opt);
    }
    if opt.color_only {
        gather_builtin_features_recursively("color-only", &mut features, &builtin_features, opt);
    }
    if opt.diff_highlight {
        gather_builtin_features_recursively(
            "diff-highlight",
            &mut features,
            &builtin_features,
            opt,
        );
    }
    if opt.diff_so_fancy {
        gather_builtin_features_recursively("diff-so-fancy", &mut features, &builtin_features, opt);
    }
    if opt.line_numbers {
        gather_builtin_features_recursively("line-numbers", &mut features, &builtin_features, opt);
    }
    if opt.navigate {
        gather_builtin_features_recursively("navigate", &mut features, &builtin_features, opt);
    }
    if opt.side_by_side {
        gather_builtin_features_recursively("side-by-side", &mut features, &builtin_features, opt);
    }

    if let Some(git_config) = git_config {
        // Gather features from [delta] section if --features was not passed.
        if opt.features.is_empty() {
            if let Some(feature_string) = git_config.get::<String>(&format!("delta.features")) {
                for feature in split_feature_string(&feature_string.to_lowercase()) {
                    gather_features_recursively(
                        feature,
                        &mut features,
                        &builtin_features,
                        opt,
                        git_config,
                    )
                }
            }
        }
        // Always gather builtin feature flags from [delta] section.
        gather_builtin_features_from_flags_in_gitconfig(
            "delta",
            &mut features,
            &builtin_features,
            opt,
            git_config,
        );
    }

    Vec::<String>::from(features)
}

/// Add to feature list `features` all features in the tree rooted at `feature`.
fn gather_features_recursively<'a>(
    feature: &str,
    features: &mut VecDeque<String>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
    git_config: &git_config::GitConfig,
) {
    if builtin_features.contains_key(feature) {
        gather_builtin_features_recursively(feature, features, builtin_features, opt);
    } else {
        features.push_front(feature.to_string());
    }
    if let Some(child_features) = git_config.get::<String>(&format!("delta.{}.features", feature)) {
        for child_feature in split_feature_string(&child_features) {
            if !features.contains(&child_feature.to_string()) {
                gather_features_recursively(
                    child_feature,
                    features,
                    builtin_features,
                    opt,
                    git_config,
                )
            }
        }
    }
    gather_builtin_features_from_flags_in_gitconfig(
        &format!("delta.{}", feature),
        features,
        builtin_features,
        opt,
        git_config,
    );
}

/// Look for builtin features requested via boolean feature flags (as opposed to via a "features"
/// list) in a custom feature section in git config and add them to the features list.
fn gather_builtin_features_from_flags_in_gitconfig<'a>(
    git_config_key: &str,
    features: &mut VecDeque<String>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
    git_config: &git_config::GitConfig,
) {
    for child_feature in builtin_features.keys() {
        if let Some(value) =
            git_config.get::<bool>(&format!("{}.{}", git_config_key, child_feature))
        {
            if value {
                gather_builtin_features_recursively(child_feature, features, builtin_features, opt);
            }
        }
    }
}

/// Add to feature list `features` all builtin features in the tree rooted at `builtin_feature`. A
/// builtin feature is a named collection of (option-name, value) pairs. This tree arises because
/// those option names might include (a) a "features" list, and (b) boolean feature flags. I.e. the
/// children of a node in the tree are features in (a) and (b). (In both cases the features
/// referenced will be other builtin features, since a builtin feature is determined at compile
/// time and therefore cannot know of the existence of a non-builtin custom features in gitconfig).
fn gather_builtin_features_recursively<'a>(
    feature: &str,
    features: &mut VecDeque<String>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
) {
    let feature_string = feature.to_string();
    if features.contains(&feature_string) {
        return;
    }
    features.push_front(feature_string);
    let feature_data = builtin_features.get(feature).unwrap();
    if let Some(child_features_fn) = feature_data.get("features") {
        if let ProvenancedOptionValue::DefaultValue(OptionValue::String(features_string)) =
            child_features_fn(opt, &None)
        {
            for child_feature in split_feature_string(&features_string) {
                gather_builtin_features_recursively(child_feature, features, builtin_features, opt);
            }
        }
    }
    for child_feature in builtin_features.keys() {
        if let Some(child_features_fn) = feature_data.get(child_feature) {
            if let ProvenancedOptionValue::DefaultValue(OptionValue::Boolean(value)) =
                child_features_fn(opt, &None)
            {
                if value {
                    gather_builtin_features_recursively(
                        child_feature,
                        features,
                        builtin_features,
                        opt,
                    );
                }
            }
        }
    }
}

fn split_feature_string(features: &str) -> impl Iterator<Item = &str> {
    features.split_whitespace().rev()
}

fn set_true_color(opt: &mut cli::Opt) {
    opt.computed.true_color = match opt.true_color.as_ref() {
        "always" => true,
        "never" => false,
        "auto" => is_truecolor_terminal(),
        _ => {
            eprintln!(
                "Invalid value for --24-bit-color option: {} (valid values are \"always\", \"never\", and \"auto\")",
                opt.true_color
            );
            process::exit(1);
        }
    };
}

fn is_truecolor_terminal() -> bool {
    env::get_env_var("COLORTERM")
        .map(|colorterm| colorterm == "truecolor" || colorterm == "24bit")
        .unwrap_or(false)
}

fn set_paging_mode(opt: &mut cli::Opt) {
    opt.computed.paging_mode = match opt.paging_mode.as_ref() {
        "always" => PagingMode::Always,
        "never" => PagingMode::Never,
        "auto" => PagingMode::QuitIfOneScreen,
        _ => {
            eprintln!(
                "Invalid value for --paging option: {} (valid values are \"always\", \"never\", and \"auto\")",
                opt.paging_mode
            );
            process::exit(1);
        }
    };
}

fn set_widths(opt: &mut cli::Opt) {
    // Allow one character in case e.g. `less --status-column` is in effect. See #41 and #10.
    opt.computed.available_terminal_width = (Term::stdout().size().1 - 1) as usize;
    let (decorations_width, background_color_extends_to_terminal_width) = match opt.width.as_deref()
    {
        Some("variable") => (cli::Width::Variable, false),
        Some(width) => {
            let width = width.parse().unwrap_or_else(|_| {
                eprintln!("Could not parse width as a positive integer: {:?}", width);
                process::exit(1);
            });
            (
                cli::Width::Fixed(min(width, opt.computed.available_terminal_width)),
                true,
            )
        }
        None => (
            cli::Width::Fixed(opt.computed.available_terminal_width),
            true,
        ),
    };
    opt.computed.decorations_width = decorations_width;
    opt.computed.background_color_extends_to_terminal_width =
        background_color_extends_to_terminal_width;
}
