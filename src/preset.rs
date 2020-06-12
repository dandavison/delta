use std::collections::HashMap;

use crate::cli;

type PresetValueFunction<T> = Box<dyn Fn(&cli::Opt, &Option<git2::Config>) -> T>;
pub type BuiltinPreset<T> = HashMap<String, PresetValueFunction<T>>;

pub trait GetValueFunctionFromBuiltinPreset {
    fn get_value_function_from_builtin_preset<'a>(
        _option_name: &str,
        _builtin_preset: &'a BuiltinPreset<String>,
    ) -> Option<&'a PresetValueFunction<Self>>
    where
        Self: Sized,
    {
        None
    }
}

impl GetValueFunctionFromBuiltinPreset for String {
    fn get_value_function_from_builtin_preset<'a>(
        option_name: &str,
        builtin_preset: &'a BuiltinPreset<String>,
    ) -> Option<&'a PresetValueFunction<String>> {
        builtin_preset.get(option_name)
    }
}

impl GetValueFunctionFromBuiltinPreset for bool {}
impl GetValueFunctionFromBuiltinPreset for i64 {}

// Construct a 2-level hash map: (preset name) -> (option name) -> (value function). A value
// function is a function that takes an Opt struct, and a git Config struct, and returns the value
// for the option.
pub fn make_builtin_presets() -> HashMap<String, BuiltinPreset<String>> {
    vec![
        (
            "diff-highlight".to_string(),
            make_diff_highlight_preset().into_iter().collect(),
        ),
        (
            "diff-so-fancy".to_string(),
            make_diff_so_fancy_preset().into_iter().collect(),
        ),
    ]
    .into_iter()
    .collect()
}

fn _make_diff_highlight_preset<'a>(bold: bool) -> Vec<(String, PresetValueFunction<String>)> {
    vec![
        (
            "minus-style".to_string(),
            Box::new(move |_opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => git_config.get_string("color.diff.old").ok(),
                    None => None,
                }
                .unwrap_or_else(|| (if bold { "bold red" } else { "red" }).to_string())
            }),
        ),
        (
            "minus-non-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => {
                        git_config.get_string("color.diff-highlight.oldNormal").ok()
                    }
                    None => None,
                }
                .unwrap_or_else(|| opt.minus_style.clone())
            }),
        ),
        (
            "minus-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => git_config
                        .get_string("color.diff-highlight.oldHighlight")
                        .ok(),
                    None => None,
                }
                .unwrap_or_else(|| format!("{} reverse", opt.minus_style))
            }),
        ),
        (
            "zero-style".to_string(),
            Box::new(|_opt: &cli::Opt, _git_config: &Option<git2::Config>| "normal".to_string()),
        ),
        (
            "plus-style".to_string(),
            Box::new(move |_opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => git_config.get_string("color.diff.new").ok(),
                    None => None,
                }
                .unwrap_or_else(|| (if bold { "bold green" } else { "green" }).to_string())
            }),
        ),
        (
            "plus-non-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => {
                        git_config.get_string("color.diff-highlight.newNormal").ok()
                    }
                    None => None,
                }
                .unwrap_or_else(|| opt.plus_style.clone())
            }),
        ),
        (
            "plus-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<git2::Config>| {
                match git_config {
                    Some(git_config) => git_config
                        .get_string("color.diff-highlight.newHighlight")
                        .ok(),
                    None => None,
                }
                .unwrap_or_else(|| format!("{} reverse", opt.plus_style))
            }),
        ),
    ]
}

fn make_diff_highlight_preset() -> Vec<(String, PresetValueFunction<String>)> {
    _make_diff_highlight_preset(false)
}

fn make_diff_so_fancy_preset() -> Vec<(String, PresetValueFunction<String>)> {
    let mut preset = _make_diff_highlight_preset(true);
    preset.push((
        "commit-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<git2::Config>| "bold yellow".to_string()),
    ));
    preset.push((
        "commit-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<git2::Config>| "none".to_string()),
    ));
    preset.push((
        "file-style".to_string(),
        Box::new(|_opt: &cli::Opt, git_config: &Option<git2::Config>| {
            match git_config {
                Some(git_config) => git_config.get_string("color.diff.meta").ok(),
                None => None,
            }
            .unwrap_or_else(|| "11".to_string())
        }),
    ));
    preset.push((
        "file-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<git2::Config>| {
            "bold yellow ul ol".to_string()
        }),
    ));
    preset.push((
        "hunk-header-style".to_string(),
        Box::new(|_opt: &cli::Opt, git_config: &Option<git2::Config>| {
            match git_config {
                Some(git_config) => git_config.get_string("color.diff.frag").ok(),
                None => None,
            }
            .unwrap_or_else(|| "bold syntax".to_string())
        }),
    ));
    preset.push((
        "hunk-header-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<git2::Config>| "magenta box".to_string()),
    ));
    preset
}
