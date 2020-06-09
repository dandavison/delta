use std::collections::VecDeque;

use structopt::clap;

use crate::cli;
use crate::config;
use crate::features;
use crate::git_config;

macro_rules! set_options {
	([$( ($option_name:expr, $field_ident:ident) ),* ],
     $opt:expr, $builtin_features:expr, $git_config:expr, $arg_matches:expr) => {
        $(
            if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                if let Some(value) = $crate::get_option_value::get_option_value($option_name, &$builtin_features, $opt, $git_config) {
                    $opt.$field_ident = value;
                }
            };
        )*
	};
}

pub fn set_options(
    opt: &mut cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
    arg_matches: &clap::ArgMatches,
) {
    if let Some(git_config) = git_config {
        if opt.no_gitconfig {
            git_config.enabled = false;
        }
    }
    let builtin_features = features::make_builtin_features();
    opt.features = gather_features(
        opt,
        builtin_features.keys().into_iter().collect(),
        git_config,
    );

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
            ("color-only", color_only),
            ("commit-decoration-style", commit_decoration_style),
            ("commit-style", commit_style),
            ("dark", dark),
            ("file-added-label", file_added_label),
            ("file-decoration-style", file_decoration_style),
            ("file-modified-label", file_modified_label),
            ("file-removed-label", file_removed_label),
            ("file-renamed-label", file_renamed_label),
            ("file-style", file_style),
            ("hunk-header-decoration-style", hunk_header_decoration_style),
            ("hunk-header-style", hunk_header_style),
            ("keep-plus-minus-markers", keep_plus_minus_markers),
            ("light", light),
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
            ("number", show_line_numbers),
            ("number-left-format", number_left_format),
            ("number-left-format-style", number_left_format_style),
            ("number-minus-style", number_minus_style),
            ("number-plus-style", number_plus_style),
            ("number-right-format", number_right_format),
            ("number-right-format-style", number_right_format_style),
            ("number-zero-style", number_zero_style),
            ("paging-mode", paging_mode),
            // Hack: plus-style must come before plus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("plus-style", plus_style),
            ("plus-emph-style", plus_emph_style),
            ("plus-empty-line-marker-style", plus_empty_line_marker_style),
            ("plus-non-emph-style", plus_non_emph_style),
            ("syntax-theme", syntax_theme),
            ("tabs", tab_width),
            ("true-color", true_color),
            ("whitespace-error-style", whitespace_error_style),
            ("width", width),
            ("word-diff-regex", tokenization_regex),
            ("zero-style", zero_style)
        ],
        opt,
        builtin_features,
        git_config,
        arg_matches
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
/// 4. Suppose the main [delta] section has `diff-highlight = true` followed by `color-only = true`.
///    Then
///    - `diff-highlight`
///    - `color-only`
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
    builtin_feature_names: Vec<&String>,
    git_config: &Option<git_config::GitConfig>,
) -> String {
    let mut features = VecDeque::new();

    // Gather features from command line.
    if let Some(git_config) = git_config {
        for feature in split_feature_string(&opt.features.to_lowercase()) {
            gather_features_recursively(feature, &mut features, &builtin_feature_names, git_config);
        }
    } else {
        for feature in split_feature_string(&opt.features.to_lowercase()) {
            features.push_front(feature.to_string());
        }
    }

    // Gather builtin feature flags supplied on command line.
    // TODO: Iterate over programatically-obtained names of builtin features.
    if opt.color_only {
        features.push_front("color-only".to_string());
    }
    if opt.diff_highlight {
        features.push_front("diff-highlight".to_string());
    }
    if opt.diff_so_fancy {
        features.push_front("diff-so-fancy".to_string());
    }
    if opt.navigate {
        features.push_front("navigate".to_string());
    }

    if let Some(git_config) = git_config {
        // Gather features from [delta] section if --features was not passed.
        if opt.features.is_empty() {
            if let Some(feature_string) = git_config.get::<String>(&format!("delta.features")) {
                for feature in split_feature_string(&feature_string.to_lowercase()) {
                    gather_features_recursively(
                        feature,
                        &mut features,
                        &builtin_feature_names,
                        git_config,
                    )
                }
            }
        }
        // Always gather builtin feature flags from [delta] section.
        gather_builtin_features("delta", &mut features, &builtin_feature_names, git_config);
    }

    Vec::<String>::from(features).join(" ")
}

fn gather_features_recursively<'a>(
    feature: &str,
    features: &mut VecDeque<String>,
    builtin_feature_names: &Vec<&String>,
    git_config: &git_config::GitConfig,
) {
    features.push_front(feature.to_string());
    if let Some(child_features) = git_config.get::<String>(&format!("delta.{}.features", feature)) {
        for child_feature in split_feature_string(&child_features) {
            if !features.contains(&child_feature.to_string()) {
                gather_features_recursively(
                    child_feature,
                    features,
                    builtin_feature_names,
                    git_config,
                )
            }
        }
    }
    gather_builtin_features(
        &format!("delta.{}", feature),
        features,
        builtin_feature_names,
        git_config,
    );
}

fn gather_builtin_features<'a>(
    git_config_key: &str,
    features: &mut VecDeque<String>,
    builtin_feature_names: &Vec<&String>,
    git_config: &git_config::GitConfig,
) {
    for feature in builtin_feature_names {
        if let Some(value) = git_config.get::<bool>(&format!("{}.{}", git_config_key, feature)) {
            if value {
                features.push_front(feature.to_string());
            }
        }
    }
}

fn split_feature_string(features: &str) -> impl Iterator<Item = &str> {
    features.split_whitespace().rev()
}
