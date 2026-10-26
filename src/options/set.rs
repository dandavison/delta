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
use crate::git_config::GitConfig;
use crate::options::option_value::{OptionValue, ProvenancedOptionValue};
use crate::options::theme;
use crate::utils::bat::output::PagingMode;

macro_rules! set_options {
    ([$( $field_ident:ident ),* ],
    $opt:expr, $builtin_features:expr, $git_config:expr, $arg_matches:expr, $expected_option_name_map:expr, $check_names:expr) => {
        let mut option_names = HashSet::new();
        $(
            let field_name = stringify!($field_ident);
            let option_name = &$expected_option_name_map[field_name];
            if !$crate::config::user_supplied_option(&field_name, $arg_matches) {
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
                option_names.insert(option_name.as_str());
            }
        )*
        if $check_names {
            option_names.extend(&[
                "24-bit-color",
                "diff-highlight", // Does not exist as a flag on config
                "diff-so-fancy", // Does not exist as a flag on config
                "detect-dark-light", // Does not exist as a flag on config
                "features",  // Processed differently
                // Set prior to the rest
                "no-gitconfig",
                "dark",
                "light",
                "syntax-theme",
            ]);
            let expected_option_names: HashSet<_> = $expected_option_name_map
                .values()
                .map(String::as_str)
                .collect();

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
    }
    opt.navigate = opt.navigate || opt.env.navigate.is_some();
    if opt.syntax_theme.is_none() {
        opt.syntax_theme.clone_from(&opt.env.bat_theme);
    }

    let option_names = cli::Opt::get_argument_and_option_names();

    // Set features
    let mut builtin_features = features::make_builtin_features();

    // --color-only is used for interactive.diffFilter (git add -p) and side-by-side cannot be used
    // there (does not emit lines in 1-1 correspondence with raw git output). See #274.
    if config::user_supplied_option("color_only", arg_matches) {
        builtin_features.remove("side-by-side");
    }

    // Needed before gathering features because a side-by-side min width can rule out the
    // side-by-side feature.
    set_available_terminal_width(opt);
    set_side_by_side_width(opt);
    let features = gather_features(opt, &builtin_features, git_config);
    opt.features = Some(features.join(" "));

    // Set light, dark, and syntax-theme.
    set__light__dark__syntax_theme__options(opt, git_config, arg_matches, &option_names);

    // HACK: make minus-line styles have syntax-highlighting iff side-by-side.
    if features.contains(&"side-by-side".to_string()) {
        let prefix = "normal ";
        if !config::user_supplied_option("minus_style", arg_matches)
            && opt.minus_style.starts_with(prefix)
        {
            opt.minus_style = format!("syntax {}", &opt.minus_style[prefix.len()..]);
        }
        if !config::user_supplied_option("minus_emph_style", arg_matches)
            && opt.minus_emph_style.starts_with(prefix)
        {
            opt.minus_emph_style = format!("syntax {}", &opt.minus_emph_style[prefix.len()..]);
        }
    }

    // Handle options which default to an arbitrary git config value.
    // TODO: incorporate this logic into the set_options macro.
    if !config::user_supplied_option("whitespace_error_style", arg_matches) {
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
            config,
            commit_decoration_style,
            commit_regex,
            commit_style,
            default_language,
            diff_args,
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
            grep_header_decoration_style,
            grep_header_file_style,
            grep_output_type,
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
            max_syntax_length,
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
    theme::set__color_mode__syntax_theme__syntax_set(opt, assets);
    opt.computed.inspect_raw_lines =
        cli::InspectRawLines::from_str(&opt.inspect_raw_lines).unwrap();
    opt.computed.paging_mode = parse_paging_mode(&opt.paging_mode);

    opt.computed.side_by_side =
        side_by_side_is_active(opt.side_by_side, opt.computed.side_by_side_width);

    // --color-only is used for interactive.diffFilter (git add -p). side-by-side, and
    // **-decoration-style cannot be used there (does not emit lines in 1-1 correspondence with raw git output).
    // See #274.
    if opt.color_only {
        opt.computed.side_by_side = false;
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
    option_names: &HashMap<String, String>,
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
    // TODO: Iterate over programmatically-obtained names of builtin features.
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
    if opt.keep_plus_minus_markers {
        gather_builtin_features_recursively(
            "keep-plus-minus-markers",
            &mut features,
            builtin_features,
            opt,
        );
    }
    if opt.line_numbers {
        gather_builtin_features_recursively("line-numbers", &mut features, builtin_features, opt);
    }
    if opt.navigate {
        gather_builtin_features_recursively("navigate", &mut features, builtin_features, opt);
    }
    if side_by_side_is_active(opt.side_by_side, opt.computed.side_by_side_width) {
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
    if let Some(child_features) = git_config.get::<String>(&format!("delta.{feature}.features")) {
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
        &format!("delta.{feature}"),
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
        let key = format!("{git_config_key}.{child_feature}");
        let is_active = if child_feature == "side-by-side" {
            git_config
                .get::<cli::SideBySide>(&key)
                .is_some_and(|side_by_side| {
                    side_by_side_is_active(side_by_side, opt.computed.side_by_side_width)
                })
        } else {
            git_config.get::<bool>(&key) == Some(true)
        };
        if is_active {
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

fn side_by_side_is_active(side_by_side: cli::SideBySide, side_by_side_width: usize) -> bool {
    match side_by_side {
        cli::SideBySide::Off => false,
        cli::SideBySide::On => true,
        cli::SideBySide::MinWidth(min_width) => side_by_side_width >= min_width,
    }
}

impl FromStr for cli::SideBySide {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(min_width) = s.parse::<usize>() {
            return Ok(min_width.into());
        }
        match s.to_lowercase().as_str() {
            "true" | "yes" | "on" => Ok(Self::On),
            "false" | "no" | "off" | "" => Ok(Self::Off),
            _ => Err(format!(
                r#"expected "true", "false", or a minimum terminal width (e.g. 80), got "{s}""#
            )),
        }
    }
}

impl From<usize> for cli::SideBySide {
    fn from(min_width: usize) -> Self {
        match min_width {
            0 => Self::Off,
            min_width => Self::MinWidth(min_width),
        }
    }
}

impl From<bool> for cli::SideBySide {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::On
        } else {
            Self::Off
        }
    }
}

impl FromStr for cli::InspectRawLines {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            _ => {
                fatal(format!(
                    r#"Invalid value for inspect-raw-lines option: {s}. Valid values are "true", and "false"."#,
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
                "Invalid value for --paging option: {paging_mode_string} (valid values are \"always\", \"never\", and \"auto\")",
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
                format!(" (from {width_arg:?})")
            } else {
                "".into()
            };
            format!("{width:?}{subexpr} is not a{pos} integer")
        })
    };

    let width = match width_arg.find('-') {
        None => parse(width_arg, false, false)?.try_into().unwrap(),
        Some(0) => (terminal_width as isize + parse(width_arg, true, false)?)
            .try_into()
            .map_err(|_| {
                format!(
                    "the current terminal width of {} minus {} is negative",
                    terminal_width,
                    width_arg[1..].trim(),
                )
            })?,
        Some(index) => {
            let a = parse(&width_arg[0..index], false, true)?;
            let b = parse(&width_arg[index..], true, true)?;
            (a + b)
                .try_into()
                .map_err(|_| format!("expression {width_arg:?} is not positive"))?
        }
    };

    Ok(width)
}

fn set_available_terminal_width(opt: &mut cli::Opt) {
    let term_stdout = Term::stdout();
    // If one extra character for e.g. `less --status-column` is required use "-1"
    // as an argument, also see #41, #10, #115 and #727.
    opt.computed.available_terminal_width =
        width_from_columns_env_var(&opt.env).unwrap_or_else(|| {
            crate::utils::workarounds::windows_msys2_width_fix(term_stdout.size(), &term_stdout)
        });
}

/// Set the width that the side-by-side minimum width applies to: --width if it is a number,
/// otherwise the terminal width. When delta's output is captured, not written to a terminal,
/// --width is the only way to tell delta the width. Only --width from the command line counts,
/// because the side-by-side feature is decided before --width is read from git config.
fn set_side_by_side_width(opt: &mut cli::Opt) {
    let terminal_width = opt.computed.available_terminal_width;
    opt.computed.side_by_side_width = opt
        .width
        .as_deref()
        .and_then(|width| parse_width_specifier(width, terminal_width).ok())
        .unwrap_or(terminal_width);
}

fn set_widths_and_isatty(opt: &mut cli::Opt) {
    opt.computed.stdout_is_term = Term::stdout().is_term();

    let (decorations_width, background_color_extends_to_terminal_width) = match opt.width.as_deref()
    {
        Some("variable") => (cli::Width::Variable, false),
        Some(width) => {
            let width = parse_width_specifier(width, opt.computed.available_terminal_width)
                .unwrap_or_else(|err| fatal(format!("Invalid value for width: {err}")));
            (cli::Width::Fixed(width), true)
        }
        None => {
            #[cfg(test)]
            {
                // instead of passing `--width=..` to all tests, set it here:
                (cli::Width::Fixed(tests::TERMINAL_WIDTH_IN_TESTS), true)
            }
            #[cfg(not(test))]
            {
                (
                    cli::Width::Fixed(opt.computed.available_terminal_width),
                    true,
                )
            }
        }
    };
    opt.computed.decorations_width = decorations_width;
    opt.computed.background_color_extends_to_terminal_width =
        background_color_extends_to_terminal_width;
}

/// The width that $COLUMNS states, if it holds a positive number.
///
/// A caller that runs delta without a terminal on delta's stdout has no other way to say how wide
/// the output should be. Git has the same problem once it has redirected its own stdout to the
/// pager, and it solves it by setting $COLUMNS for the pager to inherit; git's `term_columns()`
/// then reads $COLUMNS ahead of the ioctl. Delta follows git here, so that the two agree on the
/// width when delta renders a diff that git produced.
fn width_from_columns_env_var(env: &DeltaEnv) -> Option<usize> {
    match env.columns.as_deref()?.trim().parse() {
        Ok(width) if width > 0 => Some(width),
        _ => None,
    }
}

fn set_true_color(opt: &mut cli::Opt) {
    if opt.true_color == "auto" {
        // It's equal to its default, so the user might be using the deprecated
        // --24-bit-color option.
        if let Some(_24_bit_color) = opt._24_bit_color.as_ref() {
            opt.true_color.clone_from(_24_bit_color);
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

#[cfg(test)]
pub mod tests {
    use std::fs::remove_file;

    use crate::cli;
    use crate::tests::integration_test_utils;
    use crate::utils::bat::output::PagingMode;

    pub const TERMINAL_WIDTH_IN_TESTS: usize = 43;

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
        assert!(!opt.color_only);
        assert_eq!(opt.commit_decoration_style, "black black");
        assert_eq!(opt.commit_style, "black black");
        assert!(!opt.dark);
        assert_eq!(opt.default_language, "rs".to_owned());
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
        assert!(opt.keep_plus_minus_markers);
        assert!(opt.light);
        assert!(opt.line_numbers);
        assert_eq!(opt.line_numbers_left_format, "xxxyyyzzz");
        assert_eq!(opt.line_numbers_left_style, "black black");
        assert_eq!(opt.line_numbers_minus_style, "black black");
        assert_eq!(opt.line_numbers_plus_style, "black black");
        assert_eq!(opt.line_numbers_right_format, "xxxyyyzzz");
        assert_eq!(opt.line_numbers_right_style, "black black");
        assert_eq!(opt.line_numbers_zero_style, "black black");
        assert_eq!(opt.max_line_distance, 77.0);
        assert_eq!(opt.max_line_length, 77);
        assert_eq!(opt.minus_emph_style, "black black");
        assert_eq!(opt.minus_empty_line_marker_style, "black black");
        assert_eq!(opt.minus_non_emph_style, "black black");
        assert_eq!(opt.minus_style, "black black");
        assert!(opt.navigate);
        assert_eq!(opt.navigate_regex, Some("xxxyyyzzz".to_string()));
        assert_eq!(opt.paging_mode, "never");
        assert_eq!(opt.plus_emph_style, "black black");
        assert_eq!(opt.plus_empty_line_marker_style, "black black");
        assert_eq!(opt.plus_non_emph_style, "black black");
        assert_eq!(opt.plus_style, "black black");
        assert!(opt.raw);
        assert!(opt.computed.side_by_side);
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

    #[test]
    fn test_width_from_columns_env_var() {
        use super::width_from_columns_env_var;
        use crate::env::DeltaEnv;

        let width_for = |columns: Option<&str>| {
            width_from_columns_env_var(&DeltaEnv {
                columns: columns.map(str::to_string),
                ..DeltaEnv::default()
            })
        };

        assert_eq!(width_for(None), None);
        assert_eq!(width_for(Some("")), None);
        assert_eq!(width_for(Some("0")), None);
        assert_eq!(width_for(Some("-80")), None);
        assert_eq!(width_for(Some("wide")), None);
        assert_eq!(width_for(Some("80x24")), None);

        assert_eq!(width_for(Some("80")), Some(80));
        assert_eq!(width_for(Some(" 80 ")), Some(80));
    }

    #[test]
    fn test_side_by_side_min_width_disables_sbs_when_terminal_narrow() {
        // Use a min-width much larger than any reasonable terminal to guarantee it's disabled.
        let opt = integration_test_utils::make_options_from_args(&["--side-by-side=99999"]);
        assert!(!opt.computed.side_by_side);
    }

    #[test]
    fn test_side_by_side_always_enables_without_value() {
        // --side-by-side without a value always sets: enabled=true, min_width=0
        let opt = integration_test_utils::make_options_from_args(&["--side-by-side"]);
        assert!(opt.computed.side_by_side);
    }

    #[test]
    fn test_parse_side_by_side() {
        use cli::SideBySide::*;
        let parse = |s: &str| s.parse::<cli::SideBySide>();
        assert_eq!(parse("true"), Ok(On));
        assert_eq!(parse("yes"), Ok(On));
        assert_eq!(parse("on"), Ok(On));
        assert_eq!(parse("false"), Ok(Off));
        assert_eq!(parse("no"), Ok(Off));
        assert_eq!(parse("off"), Ok(Off));
        assert_eq!(parse("80"), Ok(MinWidth(80)));
        assert_eq!(parse("40"), Ok(MinWidth(40)));
        assert_eq!(parse("0"), Ok(Off));
        assert_eq!(parse("True"), Ok(On));
        assert_eq!(parse(""), Ok(Off));
        assert!(parse("maybe").is_err());
        assert!(parse("-1").is_err());
    }

    #[test]
    fn test_side_by_side_min_width_from_git_config() {
        use std::fs::remove_file;
        let git_config_contents = b"
[delta]
    side-by-side = 99999
";
        let git_config_path = "delta__test_side_by_side_min_width_from_git_config.gitconfig";
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        // min-width of 99999 is larger than any terminal, so side-by-side should be disabled
        assert!(!opt.computed.side_by_side);
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_side_by_side_true_from_git_config() {
        use std::fs::remove_file;
        let git_config_contents = b"
[delta]
    side-by-side = true
";
        let git_config_path = "delta__test_side_by_side_true_from_git_config.gitconfig";
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert!(opt.computed.side_by_side);
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_side_by_side_true_from_git_config_activates_line_numbers() {
        use std::fs::remove_file;
        // side-by-side = true should activate child features (line-numbers)
        let git_config_contents = b"
[delta]
    side-by-side = true
";
        let git_config_path = "delta__test_side_by_side_numeric_activates_line_numbers.gitconfig";
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert!(opt.computed.side_by_side);
        // The side-by-side feature should activate line-numbers as a child feature
        assert!(
            opt.features.as_ref().unwrap().contains("line-numbers"),
            "side-by-side = true should activate line-numbers feature, got: {:?}",
            opt.features
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_side_by_side_numeric_value_from_git_config_activates_line_numbers() {
        use std::fs::remove_file;
        // side-by-side = <number> should activate child features (line-numbers)
        // Using 10 which is smaller than TERMINAL_WIDTH_IN_TESTS (43)
        let git_config_contents = b"
[delta]
    side-by-side = 10
";
        let git_config_path =
            "delta__test_side_by_side_numeric_value_activates_line_numbers.gitconfig";
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert!(opt.computed.side_by_side);
        assert!(
            opt.features.as_ref().unwrap().contains("line-numbers"),
            "side-by-side = 10 should activate line-numbers feature, got: {:?}",
            opt.features
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_side_by_side_zero_from_git_config_disables() {
        use std::fs::remove_file;
        let git_config_contents = b"
[delta]
    side-by-side = 0
";
        let git_config_path = "delta__test_side_by_side_zero_disables.gitconfig";
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert!(!opt.computed.side_by_side);
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_side_by_side_min_width_enables_sbs_when_terminal_wide() {
        // min-width of 1 is smaller than TERMINAL_WIDTH_IN_TESTS (43), so sbs should be enabled.
        let opt = integration_test_utils::make_options_from_args(&["--side-by-side=1"]);
        assert!(opt.computed.side_by_side);
    }

    #[test]
    fn test_side_by_side_min_width_applies_to_width_option() {
        // With --width, the minimum applies to that width, whatever the terminal width is. The
        // side-by-side feature, which brings line numbers, follows the same decision.
        let side_by_side = |args: &[&str]| {
            let opt = integration_test_utils::make_options_from_args(args);
            let features = opt.features.unwrap_or_default();
            assert_eq!(
                opt.computed.side_by_side,
                features.split_whitespace().any(|f| f == "side-by-side"),
                "{args:?}: features {features:?}"
            );
            opt.computed.side_by_side
        };
        assert!(side_by_side(&["--side-by-side=99999", "--width=100000"]));
        assert!(!side_by_side(&["--side-by-side=2", "--width=1"]));
        // --width=variable paints at the terminal width.
        assert!(!side_by_side(&["--side-by-side=99999", "--width=variable"]));
    }

    #[test]
    fn test_side_by_side_min_width_fallback_is_identical_to_unified() {
        let git_config_path =
            "delta__test_side_by_side_min_width_fallback_is_identical_to_unified.gitconfig";
        let unified = run_delta_in_100_columns(&[], None, None, git_config_path);
        let side_by_side =
            run_delta_in_100_columns(&["--side-by-side"], None, None, git_config_path);
        assert_ne!(side_by_side, unified);
        let custom_feature = |line: &str| {
            format!("[delta]\n    features = my-feature\n[delta \"my-feature\"]\n    {line}\n")
        };
        // (args, git config contents, GIT_CONFIG_PARAMETERS, expected output)
        type Case<'a> = (&'a [&'a str], Option<String>, Option<&'a str>, &'a String);
        let cases: &[Case] = &[
            (&["--side-by-side=100"], None, None, &side_by_side),
            (&["--side-by-side=101"], None, None, &unified),
            (
                &[],
                Some("[delta]\n    side-by-side = 100\n".into()),
                None,
                &side_by_side,
            ),
            (
                &[],
                Some("[delta]\n    side-by-side = 101\n".into()),
                None,
                &unified,
            ),
            (
                &[],
                Some(custom_feature("side-by-side = 100")),
                None,
                &side_by_side,
            ),
            (
                &[],
                Some(custom_feature("side-by-side = 101")),
                None,
                &unified,
            ),
            (
                &[],
                Some("".into()),
                Some("'delta.side-by-side=100'"),
                &side_by_side,
            ),
            (
                &[],
                Some("".into()),
                Some("'delta.side-by-side=101'"),
                &unified,
            ),
        ];
        let mismatches: Vec<String> = cases
            .iter()
            .filter(
                |(args, git_config_contents, git_config_parameters, expected)| {
                    run_delta_in_100_columns(
                        args,
                        git_config_contents.as_deref(),
                        *git_config_parameters,
                        git_config_path,
                    ) != **expected
                },
            )
            .map(
                |(args, git_config_contents, git_config_parameters, expected)| {
                    let expected = if *expected == &unified {
                        "unified"
                    } else {
                        "side-by-side"
                    };
                    format!(
                        "args={args:?} git_config={git_config_contents:?} \
                     GIT_CONFIG_PARAMETERS={git_config_parameters:?}: expected {expected} output"
                    )
                },
            )
            .collect();
        remove_file(git_config_path).ok();
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }

    fn run_delta_in_100_columns(
        args: &[&str],
        git_config_contents: Option<&str>,
        git_config_parameters: Option<&str>,
        git_config_path: &str,
    ) -> String {
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var_with_custom_env(
            crate::env::DeltaEnv {
                columns: Some("100".to_string()),
                git_config_parameters: git_config_parameters.map(str::to_string),
                ..crate::env::DeltaEnv::default()
            },
            args,
            git_config_contents.map(str::as_bytes),
            git_config_contents.map(|_| git_config_path),
        );
        integration_test_utils::run_delta(
            "\
diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,2 +1,2 @@
 a
-b
+c
",
            &crate::config::Config::from(opt),
        )
    }

    #[test]
    fn test_side_by_side_git_config_values() {
        // Git boolean spellings are accepted, as they were before side-by-side accepted a min
        // width. Unlike git, negative and suffixed integers (e.g. -1, 1k) are invalid. Invalid
        // values are ignored.
        let cases: &[(&str, bool)] = &[
            ("side-by-side = true", true),
            ("side-by-side = True", true),
            ("side-by-side = TRUE", true),
            ("side-by-side = yes", true),
            ("side-by-side = Yes", true),
            ("side-by-side = on", true),
            ("side-by-side = ON", true),
            ("side-by-side", true),
            ("side-by-side = false", false),
            ("side-by-side = False", false),
            ("side-by-side = no", false),
            ("side-by-side = NO", false),
            ("side-by-side = off", false),
            ("side-by-side = Off", false),
            ("side-by-side =", false),
            ("side-by-side = 0", false),
            ("side-by-side = 1", true),
            ("side-by-side = 100", true),
            ("side-by-side = 101", false),
            ("side-by-side = -1", false),
            ("side-by-side = 1k", false),
            ("side-by-side = maybe", false),
        ];
        assert_side_by_side_cases(
            "delta__test_side_by_side_git_config_values.gitconfig",
            cases,
            |line| format!("[delta]\n    {line}\n"),
            |_| None,
        );
    }

    #[test]
    fn test_side_by_side_git_config_values_in_custom_feature() {
        let cases: &[(&str, bool)] = &[
            ("side-by-side = True", true),
            ("side-by-side", true),
            ("side-by-side = off", false),
            ("side-by-side = 100", true),
            ("side-by-side = 101", false),
        ];
        assert_side_by_side_cases(
            "delta__test_side_by_side_git_config_values_in_custom_feature.gitconfig",
            cases,
            |line| {
                format!("[delta]\n    features = my-feature\n[delta \"my-feature\"]\n    {line}\n")
            },
            |_| None,
        );
    }

    #[test]
    fn test_side_by_side_git_config_parameters_env_var() {
        let cases: &[(&str, bool)] = &[
            ("'delta.side-by-side=true'", true),
            ("'delta.side-by-side=false'", false),
            ("'delta.side-by-side=0'", false),
            ("'delta.side-by-side=100'", true),
            ("'delta.side-by-side=101'", false),
        ];
        assert_side_by_side_cases(
            "delta__test_side_by_side_git_config_parameters_env_var.gitconfig",
            cases,
            |_| "[delta]\n    side-by-side = true\n".to_string(),
            |parameters| Some(parameters.to_string()),
        );
    }

    /// Assert that, in a 100-column terminal, each case produces the same output as
    /// `--side-by-side` if the case's bool is true, and the same output as no side-by-side
    /// option otherwise. All mismatches are reported together.
    fn assert_side_by_side_cases(
        git_config_path: &str,
        cases: &[(&str, bool)],
        make_git_config_contents: impl Fn(&str) -> String,
        make_git_config_parameters: impl Fn(&str) -> Option<String>,
    ) {
        let unified = run_delta_in_100_columns(&[], None, None, git_config_path);
        let side_by_side =
            run_delta_in_100_columns(&["--side-by-side"], None, None, git_config_path);
        let mismatches: Vec<String> = cases
            .iter()
            .filter_map(|&(case, expected_side_by_side)| {
                let contents = make_git_config_contents(case);
                let parameters = make_git_config_parameters(case);
                let output = std::panic::catch_unwind(|| {
                    run_delta_in_100_columns(
                        &[],
                        Some(&contents),
                        parameters.as_deref(),
                        git_config_path,
                    )
                });
                let (expected, expected_name) = if expected_side_by_side {
                    (&side_by_side, "side-by-side")
                } else {
                    (&unified, "unified")
                };
                match output {
                    Ok(output) if output == *expected => None,
                    Ok(_) => Some(format!("{case:?}: expected {expected_name} output")),
                    Err(_) => Some(format!(
                        "{case:?}: expected {expected_name} output, got panic"
                    )),
                }
            })
            .collect();
        remove_file(git_config_path).unwrap();
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }
}
