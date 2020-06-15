use std::collections::HashMap;
use structopt::clap;

use crate::cli;
use crate::git_config::{self, GitConfigGet};
use crate::preset::{self, GetValueFunctionFromBuiltinPreset};

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
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: GetValueFunctionFromBuiltinPreset,
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
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: GetValueFunctionFromBuiltinPreset,
    {
        if let Some(git_config) = git_config {
            if let Some(value) =
                git_config.get::<Self>(&format!("delta.{}.{}", preset, option_name))
            {
                return Some(value);
            }
        }
        if let Some(builtin_preset) = builtin_presets.get(preset) {
            if let Some(value_function) =
                Self::get_value_function_from_builtin_preset(option_name, builtin_preset)
            {
                return Some(value_function(opt, &git_config));
            }
        }
        return None;
    }
}

impl GetOptionValue for String {}

impl GetOptionValue for Option<String> {
    fn get_option_value(
        option_name: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self> {
        match get_option_value::<String>(option_name, builtin_presets, opt, git_config) {
            Some(value) => Some(Some(value)),
            None => None,
        }
    }
}

impl GetOptionValue for bool {}

impl GetOptionValue for i64 {}

impl GetOptionValue for usize {
    fn get_option_value(
        option_name: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self> {
        match get_option_value::<i64>(option_name, builtin_presets, opt, git_config) {
            Some(value) => Some(value as usize),
            None => None,
        }
    }
}

impl GetOptionValue for f64 {
    fn get_option_value(
        option_name: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self> {
        match get_option_value::<String>(option_name, builtin_presets, opt, git_config) {
            Some(value) => value.parse::<f64>().ok(),
            None => None,
        }
    }
}

fn get_option_value<T>(
    option_name: &str,
    builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
    opt: &cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
) -> Option<T>
where
    T: GitConfigGet,
    T: GetOptionValue,
    T: GetValueFunctionFromBuiltinPreset,
{
    T::get_option_value(option_name, builtin_presets, opt, git_config)
}

macro_rules! set_options {
	([$( ($option_name:expr, $type:ty, $field_ident:ident) ),* ],
     $opt:expr, $builtin_presets:expr, $git_config:expr, $arg_matches:expr) => {
        $(
            if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                if let Some(value) = get_option_value::<$type>($option_name, &$builtin_presets, $opt, $git_config) {
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
    set_options!(
        [
            // --presets must be set first
            ("presets", Option<String>, presets),
            ("color-only", bool, color_only),
            ("commit-decoration-style", String, commit_decoration_style),
            ("commit-style", String, commit_style),
            ("dark", bool, dark),
            ("file-added-label", String, file_added_label),
            ("file-decoration-style", String, file_decoration_style),
            ("file-modified-label", String, file_modified_label),
            ("file-removed-label", String, file_removed_label),
            ("file-renamed-label", String, file_renamed_label),
            ("file-style", String, file_style),
            (
                "hunk-header-decoration-style",
                String,
                hunk_header_decoration_style
            ),
            ("hunk-header-style", String, hunk_header_style),
            ("keep-plus-minus-markers", bool, keep_plus_minus_markers),
            ("light", bool, light),
            ("max-line-distance", f64, max_line_distance),
            // Hack: minus-style must come before minus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("minus-style", String, minus_style),
            ("minus-emph-style", String, minus_emph_style),
            ("minus-non-emph-style", String, minus_non_emph_style),
            ("navigate", bool, navigate),
            ("number", bool, show_line_numbers),
            ("number-minus-format", String, number_minus_format),
            (
                "number-minus-format-style",
                String,
                number_minus_format_style
            ),
            ("number-minus-style", String, number_minus_style),
            ("number-plus-format", String, number_plus_format),
            ("number-plus-format-style", String, number_plus_format_style),
            ("number-plus-style", String, number_plus_style),
            ("paging-mode", String, paging_mode),
            // Hack: plus-style must come before plus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("plus-style", String, plus_style),
            ("plus-emph-style", String, plus_emph_style),
            ("plus-non-emph-style", String, plus_non_emph_style),
            ("syntax_theme", Option<String>, syntax_theme),
            ("tabs", usize, tab_width),
            ("true-color", String, true_color),
            ("width", Option<String>, width),
            ("word-diff-regex", String, tokenization_regex),
            ("zero-style", String, zero_style)
        ],
        opt,
        preset::make_builtin_presets(),
        git_config,
        arg_matches
    );
}
