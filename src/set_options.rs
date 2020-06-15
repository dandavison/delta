use std::collections::HashMap;
use structopt::clap;

use crate::cli;
use crate::git_config::{git_config_get, GitConfigGet};
use crate::preset::{self, GetValueFunctionFromBuiltinPreset};

// A type T implementing this trait gains a static method allowing an option value of type T to be
// sought, obeying delta's standard rules for looking up option values. It is implemented for T in
// {String, bool, i64}.
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
        git_config: &mut Option<git2::Config>,
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
            if let Some(value) =
                git_config_get::<Self>(&format!("delta.{}", option_name), git_config)
            {
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
        git_config: &mut Option<git2::Config>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: GetValueFunctionFromBuiltinPreset,
    {
        if let Some(git_config) = git_config {
            if let Some(value) =
                git_config_get::<Self>(&format!("delta.{}.{}", preset, option_name), &git_config)
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
impl GetOptionValue for bool {}
impl GetOptionValue for i64 {}

#[macro_use]
mod set_options {
    // set_options<T> implementations

    macro_rules! set_options__string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value;
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__option_string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = Some(value);
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__bool {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = bool::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value;
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__f64 {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        if let Some(value) = value.parse::<f64>().ok(){
                            $opt.$field_ident = value;
                        }
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__usize {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = i64::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value as usize;
                    }
                };
            )*
	    };
    }
}

pub fn set_options(
    opt: &mut cli::Opt,
    git_config: &mut Option<git2::Config>,
    arg_matches: &clap::ArgMatches,
) {
    if opt.no_gitconfig {
        return;
    }
    // --presets must be set first
    set_options__option_string!([("presets", presets)], opt, arg_matches, git_config);
    set_options__bool!(
        [
            ("light", light),
            ("dark", dark),
            ("navigate", navigate),
            ("color-only", color_only),
            ("keep-plus-minus-markers", keep_plus_minus_markers),
            ("number", show_line_numbers)
        ],
        opt,
        arg_matches,
        git_config
    );
    set_options__f64!(
        [("max-line-distance", max_line_distance)],
        opt,
        arg_matches,
        git_config
    );
    set_options__string!(
        [
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
            // Hack: minus-style must come before minus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("minus-style", minus_style),
            ("minus-emph-style", minus_emph_style),
            ("minus-non-emph-style", minus_non_emph_style),
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
            ("plus-non-emph-style", plus_non_emph_style),
            ("true-color", true_color),
            ("word-diff-regex", tokenization_regex),
            ("zero-style", zero_style)
        ],
        opt,
        arg_matches,
        git_config
    );
    set_options__option_string!(
        [("syntax_theme", syntax_theme), ("width", width)],
        opt,
        arg_matches,
        git_config
    );
    set_options__usize!([("tabs", tab_width)], opt, arg_matches, git_config);
}
