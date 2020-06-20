use std::collections::HashMap;
use structopt::clap;

use crate::cli;
use crate::config;
use crate::git_config::{self, GitConfigGet};
use crate::preset;

// A type T implementing this trait gains a static method allowing an option value of type T to be
// looked up, implementing delta's rules for looking up option values.
trait GetOptionValue {
    // If the value for option name n was not supplied on the command line, then a search is performed
    // as follows. The first value encountered is used:
    //
    // 1. For each preset p (moving right to left through the listed presets):
    //    1.1 The value of n under p interpreted as a user-supplied preset (i.e. git config value
    //        delta.$p.$n)
    //    1.2 The value for n under p interpreted as a builtin preset
    // 3. The value for n in the main git config section for delta (i.e. git config value delta.$n)
    fn get_option_value(
        option_name: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: From<preset::OptionValue>,
    {
        if let Some(presets) = &opt.presets {
            for preset in presets.to_lowercase().split_whitespace().rev() {
                if let Some(value) = Self::get_option_value_for_preset(
                    option_name,
                    &preset,
                    &builtin_presets,
                    opt,
                    git_config,
                ) {
                    return Some(value);
                }
            }
        }
        if let Some(git_config) = git_config {
            if let Some(value) = git_config.get::<Self>(&format!("delta.{}", option_name)) {
                return Some(value);
            }
        }
        None
    }

    fn get_option_value_for_preset(
        option_name: &str,
        preset: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: From<preset::OptionValue>,
    {
        if let Some(git_config) = git_config {
            if let Some(value) =
                git_config.get::<Self>(&format!("delta.{}.{}", preset, option_name))
            {
                return Some(value);
            }
        }
        if let Some(builtin_preset) = builtin_presets.get(preset) {
            if let Some(value_function) = builtin_preset.get(option_name) {
                return Some(value_function(opt, &git_config).into());
            }
        }
        return None;
    }
}

impl GetOptionValue for Option<String> {}
impl GetOptionValue for String {}
impl GetOptionValue for bool {}
impl GetOptionValue for f64 {}
impl GetOptionValue for usize {}

fn get_option_value<T>(
    option_name: &str,
    builtin_presets: &HashMap<String, preset::BuiltinPreset>,
    opt: &cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
) -> Option<T>
where
    T: GitConfigGet,
    T: GetOptionValue,
    T: From<preset::OptionValue>,
{
    T::get_option_value(option_name, builtin_presets, opt, git_config)
}

macro_rules! set_options {
	([$( ($option_name:expr, $field_ident:ident) ),* ],
     $opt:expr, $builtin_presets:expr, $git_config:expr, $arg_matches:expr) => {
        $(
            if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                if let Some(value) = get_option_value($option_name, &$builtin_presets, $opt, $git_config) {
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
    if opt.no_gitconfig {
        return;
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
            // --presets must be set first
            ("presets", presets),
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
        preset::make_builtin_presets(),
        git_config,
        arg_matches
    );
}
