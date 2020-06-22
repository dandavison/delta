use std::collections::HashMap;
use std::collections::HashSet;
use structopt::clap;

use itertools::Itertools;

use crate::cli;
use crate::config;
use crate::features;
use crate::get_option_value::get_option_value;
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
    let builtin_features = features::make_builtin_features();
    set_features(opt, git_config, &builtin_features);
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
            ("navigate", navigate),
            ("number", show_line_numbers),
            ("number-minus-format", number_minus_format),
            ("number-minus-format-style", number_minus_format_style),
            ("number-minus-style", number_minus_style),
            ("number-plus-format", number_plus_format),
            ("number-plus-format-style", number_plus_format_style),
            ("number-plus-style", number_plus_style),
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

fn set_features(
    opt: &mut cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
) {
    if opt.color_only {
        opt.features = format!("{} color-only", opt.features);
    }
    if opt.diff_highlight {
        opt.features = format!("{} diff-highlight", opt.features);
    }
    if opt.diff_so_fancy {
        opt.features = format!("{} diff-so-fancy", opt.features);
    }
    if opt.navigate {
        opt.features = format!("{} navigate", opt.features);
    }

    if let Some(more_features) =
        get_option_value::<String>("features", builtin_features, opt, git_config)
    {
        opt.features = append_features(&opt.features, &more_features);
    }
}

fn append_features(features: &str, more_features: &str) -> String {
    let feature_set: HashSet<_> = features.split_whitespace().collect();

    let more_features = more_features
        .to_lowercase()
        .split_whitespace()
        .filter(|s| !feature_set.contains(s))
        .join(" ");

    [features, &more_features].join(" ")
}
