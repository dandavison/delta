use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::result::Result;
use std::str::FromStr;

use bat::assets::HighlightingAssets;
use console::Term;

use crate::cli;
use crate::config;
use crate::env::DeltaEnv;
use crate::errors::*;
use crate::fatal;
use crate::features;
use crate::git_config::{GitConfig, GitConfigEntry};
use crate::options::option_value::{OptionValue, ProvenancedOptionValue};
use crate::options::theme;
use crate::utils::bat::output::PagingMode;

macro_rules! set_options {
    ([$( $field_ident:ident ),* ],
    $opt:expr, $builtin_features:expr, $git_config:expr, $arg_matches:expr, $expected_option_name_map:expr, $check_names:expr) => {
        let mut option_names = HashSet::new();
        $(
            let kebab_case_field_name = stringify!($field_ident).replace("_", "-");
            let option_name = $expected_option_name_map[kebab_case_field_name.as_str()];
            if !$crate::config::user_supplied_option(&kebab_case_field_name, $arg_matches) {
                if let Some(value) = $crate::options::get::get_option_value(
                    option_name,
                    &$builtin_features,
                    $opt,
                    $git_config
                ) {
                    $opt.$field_ident = value;
                }
            }
            if $check_names {
                option_names.insert(option_name);
            }
        )*
        if $check_names {
            option_names.extend(&[
                "24-bit-color",
                "diff-highlight", // Does not exist as a flag on config
                "diff-so-fancy", // Does not exist as a flag on config
                "features",  // Processed differently
                "help", // automatically added by clap
                // Set prior to the rest
                "no-gitconfig",
                "dark",
                "light",
                "syntax-theme",
                "version", // automatically added by clap
            ]);
            let expected_option_names: HashSet<_> = $expected_option_name_map.values().cloned().collect();

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
    git_config: &mut Option<GitConfig>,
    arg_matches: &clap::ArgMatches,
    assets: HighlightingAssets,
) {
    if let Some(git_config) = git_config {
        if opt.no_gitconfig {
            git_config.enabled = false;
        }
        set_git_config_entries(opt, git_config);
    }
    opt.navigate = opt.navigate || opt.env.navigate.is_some();
    if opt.syntax_theme.is_none() {
        opt.syntax_theme = opt.env.bat_theme.clone();
    }

    let option_names = cli::Opt::get_argument_and_option_names();

    // Set features
    let mut builtin_features = features::make_builtin_features();

    // --color-only is used for interactive.diffFilter (git add -p) and side-by-side cannot be used
    // there (does not emit lines in 1-1 correspondence with raw git output). See #274.
    if config::user_supplied_option("color-only", arg_matches) {
        builtin_features.remove("side-by-side");
    }

    let features = gather_features(opt, &builtin_features, git_config);
    opt.features = Some(features.join(" "));

    // Set light, dark, and syntax-theme.
    set__light__dark__syntax_theme__options(opt, git_config, arg_matches, &option_names);

    // HACK: make minus-line styles have syntax-highlighting iff side-by-side.
    if features.contains(&"side-by-side".to_string()) {
        let prefix = "normal ";
        if !config::user_supplied_option("minus-style", arg_matches)
            && opt.minus_style.starts_with(prefix)
        {
            opt.minus_style = format!("syntax {}", &opt.minus_style[prefix.len()..]);
        }
        if !config::user_supplied_option("minus-emph-style", arg_matches)
            && opt.minus_emph_style.starts_with(prefix)
        {
            opt.minus_emph_style = format!("syntax {}", &opt.minus_emph_style[prefix.len()..]);
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
            blame_code_style,
            blame_format,
            blame_separator_format,
            blame_palette,
            blame_separator_style,
            blame_timestamp_format,
            blame_timestamp_output_format,
            color_only,
            commit_decoration_style,
            commit_regex,
            commit_style,
            default_language,
            diff_stat_align_width,
            file_added_label,
            file_copied_label,
            file_decoration_style,
            file_modified_label,
            file_removed_label,
            file_renamed_label,
            file_regex_replacement,
            right_arrow,
            hunk_label,
            file_style,
            grep_context_line_style,
            grep_file_style,
            grep_line_number_style,
            grep_match_line_style,
            grep_match_word_style,
            grep_separator_symbol,
            hunk_header_decoration_style,
            hunk_header_file_style,
            hunk_header_line_number_style,
            hunk_header_style,
            hyperlinks,
            hyperlinks_commit_link_format,
            hyperlinks_file_link_format,
            inline_hint_style,
            inspect_raw_lines,
            keep_plus_minus_markers,
            line_buffer_size,
            map_styles,
            max_line_distance,
            max_line_length,
            // Hack: minus-style must come before minus-*emph-style because the latter default
            // dynamically to the value of the former.
            merge_conflict_begin_symbol,
            merge_conflict_end_symbol,
            merge_conflict_ours_diff_header_decoration_style,
            merge_conflict_ours_diff_header_style,
            merge_conflict_theirs_diff_header_decoration_style,
            merge_conflict_theirs_diff_header_style,
            minus_style,
            minus_emph_style,
            minus_empty_line_marker_style,
            minus_non_emph_style,
            minus_non_emph_style,
            navigate,
            navigate_regex,
            line_fill_method,
            line_numbers,
            line_numbers_left_format,
            line_numbers_left_style,
            line_numbers_minus_style,
            line_numbers_plus_style,
            line_numbers_right_format,
            line_numbers_right_style,
            line_numbers_zero_style,
            pager,
            paging_mode,
            parse_ansi,
            // Hack: plus-style must come before plus-*emph-style because the latter default
            // dynamically to the value of the former.
            plus_style,
            plus_emph_style,
            plus_empty_line_marker_style,
            plus_non_emph_style,
            raw,
            relative_paths,
            show_colors,
            show_themes,
            side_by_side,
            wrap_max_lines,
            wrap_right_prefix_symbol,
            wrap_right_percent,
            wrap_right_symbol,
            wrap_left_symbol,
            tab_width,
            tokenization_regex,
            true_color,
            whitespace_error_style,
            width,
            zero_style
        ],
        opt,
        builtin_features,
        git_config,
        arg_matches,
        &option_names,
        true
    );

    // Setting ComputedValues
    set_widths_and_isatty(opt);
    set_true_color(opt);
    theme::set__is_light_mode__syntax_theme__syntax_set(opt, assets);
    opt.computed.inspect_raw_lines =
        cli::InspectRawLines::from_str(&opt.inspect_raw_lines).unwrap();
    opt.computed.paging_mode = parse_paging_mode(&opt.paging_mode);

    // --color-only is used for interactive.diffFilter (git add -p). side-by-side, and
    // **-decoration-style cannot be used there (does not emit lines in 1-1 correspondence with raw git output).
    // See #274.
    if opt.color_only {
        opt.side_by_side = false;
        opt.file_decoration_style = "none".to_string();
        opt.commit_decoration_style = "none".to_string();
        opt.hunk_header_decoration_style = "none".to_string();
    }
}

#[allow(non_snake_case)]
fn set__light__dark__syntax_theme__options(
    opt: &mut cli::Opt,
    git_config: &mut Option<GitConfig>,
    arg_matches: &clap::ArgMatches,
    option_names: &HashMap<&str, &str>,
) {
    let validate_light_and_dark = |opt: &cli::Opt| {
        if opt.light && opt.dark {
            fatal("--light and --dark cannot be used together.");
        }
    };
    let empty_builtin_features = HashMap::new();
    validate_light_and_dark(opt);
    if !(opt.light || opt.dark) {
        set_options!(
            [dark, light],
            opt,
            &empty_builtin_features,
            git_config,
            arg_matches,
            option_names,
            false
        );
    }
    validate_light_and_dark(opt);
    set_options!(
        [syntax_theme],
        opt,
        &empty_builtin_features,
        git_config,
        arg_matches,
        option_names,
        false
    );
}

// Features are processed differently from all other options. The role of this function is to
// collect all configuration related to features and summarize it as a single list
// (space-separated string) of enabled features. The list is arranged in order of increasing
// priority in the sense that, when searching for a option value, one starts at the right-hand end
// and moves leftward, examining each feature in turn until a feature that associates a value with
// the option name is encountered. This search is documented in
// `get_option_value::get_option_value`.
//
// The feature list comprises features deriving from the following sources, listed in order of
// decreasing priority:
//
// 1. Suppose the command-line has `--features "a b"`. Then
//    - `b`, followed by b's "ordered descendents"
//    - `a`, followed by a's "ordered descendents"
//
// 2. Suppose the command line enables two builtin features via `--navigate --diff-so-fancy`. Then
//    - `diff-so-fancy`
//    - `navigate`
//
// 3. Suppose the main [delta] section has `features = d e`. Then
//    - `e`, followed by e's "ordered descendents"
//    - `d`, followed by d's "ordered descendents"
//
// 4. Suppose the main [delta] section has `diff-highlight = true` followed by `raw = true`.
//    Then
//    - `diff-highlight`
//    - `raw`
//
// The "ordered descendents" of a feature `f` is a list of features obtained via a pre-order
// traversal of the feature tree rooted at `f`. This tree arises because it is allowed for a
// feature to contain a (key, value) pair that itself enables features.
//
// If a feature has already been included at higher priority, and is encountered again, it is
// ignored.
//
// Thus, for example:
//
// delta --features "my-navigate-settings" --navigate   =>   "navigate my-navigate-settings"
//
// In the following configuration, the feature names indicate their priority, with `a` having
// highest priority:
//
// delta --g --features "d a"
//
// [delta "a"]
//     features = c b
//
// [delta "d"]
//     features = f e
fn gather_features(
    opt: &mut cli::Opt,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    git_config: &Option<GitConfig>,
) -> Vec<String> {
    let from_env_var = &opt.env.features;
    let from_args = opt.features.as_deref().unwrap_or("");
    let input_features: Vec<&str> = match from_env_var.as_deref() {
        Some(from_env_var) if from_env_var.starts_with('+') => from_env_var[1..]
            .split_whitespace()
            .chain(split_feature_string(from_args))
            .collect(),
        Some(from_env_var) => {
            opt.features = Some(from_env_var.to_string());
            split_feature_string(from_env_var).collect()
        }
        None => split_feature_string(from_args).collect(),
    };

    let mut features = VecDeque::new();

    // Gather features from command line.
    if let Some(git_config) = git_config {
        for feature in input_features {
            gather_features_recursively(feature, &mut features, builtin_features, opt, git_config);
        }
    } else {
        for feature in input_features {
            features.push_front(feature.to_string());
        }
    }

    // Gather builtin feature flags supplied on command line.
    // TODO: Iterate over programatically-obtained names of builtin features.
    if opt.raw {
        gather_builtin_features_recursively("raw", &mut features, builtin_features, opt);
    }
    if opt.color_only {
        gather_builtin_features_recursively("color-only", &mut features, builtin_features, opt);
    }
    if opt.diff_highlight {
        gather_builtin_features_recursively("diff-highlight", &mut features, builtin_features, opt);
    }
    if opt.diff_so_fancy {
        gather_builtin_features_recursively("diff-so-fancy", &mut features, builtin_features, opt);
    }
    if opt.hyperlinks {
        gather_builtin_features_recursively("hyperlinks", &mut features, builtin_features, opt);
    }
    if opt.line_numbers {
        gather_builtin_features_recursively("line-numbers", &mut features, builtin_features, opt);
    }
    if opt.navigate {
        gather_builtin_features_recursively("navigate", &mut features, builtin_features, opt);
    }
    if opt.side_by_side {
        gather_builtin_features_recursively("side-by-side", &mut features, builtin_features, opt);
    }

    if let Some(git_config) = git_config {
        // Gather features from [delta] section if --features was not passed.
        if opt.features.is_none() {
            if let Some(feature_string) = git_config.get::<String>("delta.features") {
                for feature in split_feature_string(&feature_string) {
                    gather_features_recursively(
                        feature,
                        &mut features,
                        builtin_features,
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
            builtin_features,
            opt,
            git_config,
        );
    }

    Vec::<String>::from(features)
}

/// Add to feature list `features` all features in the tree rooted at `feature`.
fn gather_features_recursively(
    feature: &str,
    features: &mut VecDeque<String>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
    git_config: &GitConfig,
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
fn gather_builtin_features_from_flags_in_gitconfig(
    git_config_key: &str,
    features: &mut VecDeque<String>,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
    git_config: &GitConfig,
) {
    for child_feature in builtin_features.keys() {
        if let Some(true) = git_config.get::<bool>(&format!("{}.{}", git_config_key, child_feature))
        {
            gather_builtin_features_recursively(child_feature, features, builtin_features, opt);
        }
    }
}

/// Add to feature list `features` all builtin features in the tree rooted at `builtin_feature`. A
/// builtin feature is a named collection of (option-name, value) pairs. This tree arises because
/// those option names might include (a) a "features" list, and (b) boolean feature flags. I.e. the
/// children of a node in the tree are features in (a) and (b). (In both cases the features
/// referenced will be other builtin features, since a builtin feature is determined at compile
/// time and therefore cannot know of the existence of a non-builtin custom features in gitconfig).
fn gather_builtin_features_recursively(
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
    if let Some(feature_data) = builtin_features.get(feature) {
        if let Some(child_features_fn) = feature_data.get("features") {
            if let ProvenancedOptionValue::DefaultValue(OptionValue::String(features_string)) =
                child_features_fn(opt, &None)
            {
                for child_feature in split_feature_string(&features_string) {
                    gather_builtin_features_recursively(
                        child_feature,
                        features,
                        builtin_features,
                        opt,
                    );
                }
            }
        }
        for child_feature in builtin_features.keys() {
            if let Some(child_features_fn) = feature_data.get(child_feature) {
                if let ProvenancedOptionValue::DefaultValue(OptionValue::Boolean(true)) =
                    child_features_fn(opt, &None)
                {
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

impl FromStr for cli::InspectRawLines {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            _ => {
                fatal(format!(
                    r#"Invalid value for inspect-raw-lines option: {}. Valid values are "true", and "false"."#,
                    s
                ));
            }
        }
    }
}

fn parse_paging_mode(paging_mode_string: &str) -> PagingMode {
    match paging_mode_string.to_lowercase().as_str() {
        "always" => PagingMode::Always,
        "never" => PagingMode::Never,
        "auto" => PagingMode::QuitIfOneScreen,
        _ => {
            fatal(format!(
                "Invalid value for --paging option: {} (valid values are \"always\", \"never\", and \"auto\")",
                paging_mode_string
            ));
        }
    }
}

fn parse_width_specifier(width_arg: &str, terminal_width: usize) -> Result<usize, String> {
    let width_arg = width_arg.trim();

    let parse = |width: &str, must_be_negative, subexpression| -> Result<isize, String> {
        let remove_spaces = |s: &str| s.chars().filter(|c| c != &' ').collect::<String>();
        match remove_spaces(width).parse() {
            Ok(val) if must_be_negative && val > 0 => Err(()),
            Err(_) => Err(()),
            Ok(ok) => Ok(ok),
        }
        .map_err(|_| {
            let pos = if must_be_negative { " negative" } else { "n" };
            let subexpr = if subexpression {
                format!(" (from {:?})", width_arg)
            } else {
                "".into()
            };
            format!(
                "{:?}{subexpr} is not a{pos} integer",
                width,
                subexpr = subexpr,
                pos = pos
            )
        })
    };

    let width = match width_arg.find('-') {
        None => parse(width_arg, false, false)?.try_into().unwrap(),
        Some(index) if index == 0 => (terminal_width as isize + parse(width_arg, true, false)?)
            .try_into()
            .map_err(|_| {
                format!(
                    "the current terminal width of {} minus {} is negative",
                    terminal_width,
                    &width_arg[1..].trim(),
                )
            })?,
        Some(index) => {
            let a = parse(&width_arg[0..index], false, true)?;
            let b = parse(&width_arg[index..], true, true)?;
            (a + b)
                .try_into()
                .map_err(|_| format!("expression {:?} is not positive", width_arg))?
        }
    };

    Ok(width)
}

fn set_widths_and_isatty(opt: &mut cli::Opt) {
    let term_stdout = Term::stdout();
    opt.computed.stdout_is_term = term_stdout.is_term();

    // If one extra character for e.g. `less --status-column` is required use "-1"
    // as an argument, also see #41, #10, #115 and #727.
    opt.computed.available_terminal_width =
        crate::utils::workarounds::windows_msys2_width_fix(term_stdout.size(), &term_stdout);

    let (decorations_width, background_color_extends_to_terminal_width) = match opt.width.as_deref()
    {
        Some("variable") => (cli::Width::Variable, false),
        Some(width) => {
            let width = parse_width_specifier(width, opt.computed.available_terminal_width)
                .unwrap_or_else(|err| fatal(format!("Invalid value for width: {}", err)));
            (cli::Width::Fixed(width), true)
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

fn set_true_color(opt: &mut cli::Opt) {
    if opt.true_color == "auto" {
        // It's equal to its default, so the user might be using the deprecated
        // --24-bit-color option.
        if let Some(_24_bit_color) = opt._24_bit_color.as_ref() {
            opt.true_color = _24_bit_color.clone();
        }
    }

    opt.computed.true_color = match opt.true_color.as_ref() {
        "always" => true,
        "never" => false,
        "auto" => is_truecolor_terminal(&opt.env),
        _ => {
            fatal(format!(
                "Invalid value for --true-color option: {} (valid values are \"always\", \"never\", and \"auto\")",
                opt.true_color
            ));
        }
    };
}

fn is_truecolor_terminal(env: &DeltaEnv) -> bool {
    env.colorterm
        .as_ref()
        .map(|colorterm| colorterm == "truecolor" || colorterm == "24bit")
        .unwrap_or(false)
}

fn set_git_config_entries(opt: &mut cli::Opt, git_config: &mut GitConfig) {
    for key in &["color.diff.old", "color.diff.new"] {
        if let Some(style_string) = git_config.get::<String>(key) {
            opt.git_config_entries
                .insert(key.to_string(), GitConfigEntry::Style(style_string));
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::fs::remove_file;

    use crate::cli;
    use crate::tests::integration_test_utils;
    use crate::utils::bat::output::PagingMode;

    #[test]
    fn test_options_can_be_set_in_git_config() {
        // In general the values here are not the default values. However there are some exceptions
        // since e.g. color-only = true (non-default) forces side-by-side = false (default).
        let git_config_contents = b"
[delta]
    color-only = false
    commit-decoration-style = black black
    commit-style = black black
    dark = false
    default-language = rs
    diff-highlight = true
    diff-so-fancy = true
    features = xxxyyyzzz
    file-added-label = xxxyyyzzz
    file-decoration-style = black black
    file-modified-label = xxxyyyzzz
    file-removed-label = xxxyyyzzz
    file-renamed-label = xxxyyyzzz
    file-transformation = s/foo/bar/
    right-arrow = xxxyyyzzz
    file-style = black black
    hunk-header-decoration-style = black black
    hunk-header-style = black black
    keep-plus-minus-markers = true
    light = true
    line-numbers = true
    line-numbers-left-format = xxxyyyzzz
    line-numbers-left-style = black black
    line-numbers-minus-style = black black
    line-numbers-plus-style = black black
    line-numbers-right-format = xxxyyyzzz
    line-numbers-right-style = black black
    line-numbers-zero-style = black black
    max-line-distance = 77
    max-line-length = 77
    minus-emph-style = black black
    minus-empty-line-marker-style = black black
    minus-non-emph-style = black black
    minus-style = black black
    navigate = true
    navigate-regex = xxxyyyzzz
    paging = never
    plus-emph-style = black black
    plus-empty-line-marker-style = black black
    plus-non-emph-style = black black
    plus-style = black black
    raw = true
    side-by-side = true
    syntax-theme = xxxyyyzzz
    tabs = 77
    true-color = never
    whitespace-error-style = black black
    width = 77
    word-diff-regex = xxxyyyzzz
    zero-style = black black
    # no-gitconfig
";
        let git_config_path = "delta__test_options_can_be_set_in_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.true_color, "never");
        assert_eq!(opt.color_only, false);
        assert_eq!(opt.commit_decoration_style, "black black");
        assert_eq!(opt.commit_style, "black black");
        assert_eq!(opt.dark, false);
        assert_eq!(opt.default_language, Some("rs".to_owned()));
        // TODO: should set_options not be called on any feature flags?
        // assert_eq!(opt.diff_highlight, true);
        // assert_eq!(opt.diff_so_fancy, true);
        assert!(opt
            .features
            .unwrap()
            .split_whitespace()
            .any(|s| s == "xxxyyyzzz"));
        assert_eq!(opt.file_added_label, "xxxyyyzzz");
        assert_eq!(opt.file_decoration_style, "black black");
        assert_eq!(opt.file_modified_label, "xxxyyyzzz");
        assert_eq!(opt.file_removed_label, "xxxyyyzzz");
        assert_eq!(opt.file_renamed_label, "xxxyyyzzz");
        assert_eq!(opt.right_arrow, "xxxyyyzzz");
        assert_eq!(opt.file_style, "black black");
        assert_eq!(opt.file_regex_replacement, Some("s/foo/bar/".to_string()));
        assert_eq!(opt.hunk_header_decoration_style, "black black");
        assert_eq!(opt.hunk_header_style, "black black");
        assert_eq!(opt.keep_plus_minus_markers, true);
        assert_eq!(opt.light, true);
        assert_eq!(opt.line_numbers, true);
        assert_eq!(opt.line_numbers_left_format, "xxxyyyzzz");
        assert_eq!(opt.line_numbers_left_style, "black black");
        assert_eq!(opt.line_numbers_minus_style, "black black");
        assert_eq!(opt.line_numbers_plus_style, "black black");
        assert_eq!(opt.line_numbers_right_format, "xxxyyyzzz");
        assert_eq!(opt.line_numbers_right_style, "black black");
        assert_eq!(opt.line_numbers_zero_style, "black black");
        assert_eq!(opt.max_line_distance, 77 as f64);
        assert_eq!(opt.max_line_length, 77);
        assert_eq!(opt.minus_emph_style, "black black");
        assert_eq!(opt.minus_empty_line_marker_style, "black black");
        assert_eq!(opt.minus_non_emph_style, "black black");
        assert_eq!(opt.minus_style, "black black");
        assert_eq!(opt.navigate, true);
        assert_eq!(opt.navigate_regex, Some("xxxyyyzzz".to_string()));
        assert_eq!(opt.paging_mode, "never");
        assert_eq!(opt.plus_emph_style, "black black");
        assert_eq!(opt.plus_empty_line_marker_style, "black black");
        assert_eq!(opt.plus_non_emph_style, "black black");
        assert_eq!(opt.plus_style, "black black");
        assert_eq!(opt.raw, true);
        assert_eq!(opt.side_by_side, true);
        assert_eq!(opt.syntax_theme, Some("xxxyyyzzz".to_string()));
        assert_eq!(opt.tab_width, 77);
        assert_eq!(opt.true_color, "never");
        assert_eq!(opt.whitespace_error_style, "black black");
        assert_eq!(opt.width, Some("77".to_string()));
        assert_eq!(opt.tokenization_regex, "xxxyyyzzz");
        assert_eq!(opt.zero_style, "black black");

        assert_eq!(opt.computed.paging_mode, PagingMode::Never);

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_width_in_git_config_is_honored() {
        let git_config_contents = b"
[delta]
    features = my-width-feature

[delta \"my-width-feature\"]
    width = variable
";
        let git_config_path = "delta__test_width_in_git_config_is_honored.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(opt.computed.decorations_width, cli::Width::Variable);

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_parse_width_specifier() {
        use super::parse_width_specifier;
        let term_width = 12;

        let assert_failure_containing = |x, errmsg| {
            assert!(parse_width_specifier(x, term_width)
                .unwrap_err()
                .contains(errmsg));
        };

        assert_failure_containing("", "is not an integer");
        assert_failure_containing("foo", "is not an integer");
        assert_failure_containing("123foo", "is not an integer");
        assert_failure_containing("+12bar", "is not an integer");
        assert_failure_containing("-456bar", "is not a negative integer");

        assert_failure_containing("-13", "minus 13 is negative");
        assert_failure_containing(" -   13 ", "minus 13 is negative");
        assert_failure_containing("12-13", "expression");
        assert_failure_containing(" 12   -   13  ", "expression \"12   -   13\" is not");
        assert_failure_containing("12+foo", "is not an integer");
        assert_failure_containing(
            "  12 -  bar  ",
            "\"-  bar\" (from \"12 -  bar\") is not a negative integer",
        );

        assert_eq!(parse_width_specifier("1", term_width).unwrap(), 1);
        assert_eq!(parse_width_specifier(" 1 ", term_width).unwrap(), 1);
        assert_eq!(parse_width_specifier("-2", term_width).unwrap(), 10);
        assert_eq!(parse_width_specifier(" - 2", term_width).unwrap(), 10);
        assert_eq!(parse_width_specifier("-12", term_width).unwrap(), 0);
        assert_eq!(parse_width_specifier(" - 12 ", term_width).unwrap(), 0);
        assert_eq!(parse_width_specifier(" 2 - 2 ", term_width).unwrap(), 0);
    }
}
