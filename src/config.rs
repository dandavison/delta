use std::collections::HashMap;
use std::path::PathBuf;

use regex::Regex;
use structopt::clap;
use syntect::highlighting::Style as SyntectStyle;
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::bat_utils::output::PagingMode;
use crate::cli;
use crate::color;
use crate::delta::State;
use crate::env;
use crate::fatal;
use crate::features::navigate;
use crate::features::side_by_side::{self, ansifill, LeftRight};
use crate::git_config::{GitConfig, GitConfigEntry};
use crate::minusplus::MinusPlus;
use crate::paint::BgFillMethod;
use crate::style::{self, Style};
use crate::syntect_utils::FromDeltaStyle;
use crate::tests::TESTING;
use crate::wrapping::WrapConfig;

pub const INLINE_SYMBOL_WIDTH_1: usize = 1;

fn remove_percent_suffix(arg: &str) -> &str {
    match &arg.strip_suffix('%') {
        Some(s) => s,
        None => arg,
    }
}

fn ensure_display_width_1(what: &str, arg: String) -> String {
    match arg.grapheme_indices(true).count() {
        INLINE_SYMBOL_WIDTH_1 => arg,
        width => fatal(format!(
            "Invalid value for {}, display width of \"{}\" must be {} but is {}",
            what, arg, INLINE_SYMBOL_WIDTH_1, width
        )),
    }
}

fn adapt_wrap_max_lines_argument(arg: String) -> usize {
    if arg == "∞" || arg == "unlimited" || arg.starts_with("inf") {
        0
    } else {
        arg.parse::<usize>()
            .unwrap_or_else(|err| fatal(format!("Invalid wrap-max-lines argument: {}", err)))
            + 1
    }
}

pub struct Config {
    pub available_terminal_width: usize,
    pub background_color_extends_to_terminal_width: bool,
    pub commit_style: Style,
    pub color_only: bool,
    pub commit_regex: Regex,
    pub cwd_relative_to_repo_root: Option<String>,
    pub decorations_width: cli::Width,
    pub default_language: Option<String>,
    pub diff_stat_align_width: usize,
    pub error_exit_code: i32,
    pub file_added_label: String,
    pub file_copied_label: String,
    pub file_modified_label: String,
    pub file_removed_label: String,
    pub file_renamed_label: String,
    pub hunk_label: String,
    pub file_style: Style,
    pub git_config: Option<GitConfig>,
    pub git_config_entries: HashMap<String, GitConfigEntry>,
    pub hunk_header_file_style: Style,
    pub hunk_header_line_number_style: Style,
    pub hunk_header_style: Style,
    pub hunk_header_style_include_file_path: bool,
    pub hunk_header_style_include_line_number: bool,
    pub hyperlinks: bool,
    pub hyperlinks_commit_link_format: Option<String>,
    pub hyperlinks_file_link_format: String,
    pub inline_hint_style: Style,
    pub inspect_raw_lines: cli::InspectRawLines,
    pub keep_plus_minus_markers: bool,
    pub line_fill_method: BgFillMethod,
    pub line_numbers: bool,
    pub line_numbers_format: LeftRight<String>,
    pub line_numbers_style_leftright: LeftRight<Style>,
    pub line_numbers_style_minusplus: MinusPlus<Style>,
    pub line_numbers_zero_style: Style,
    pub line_buffer_size: usize,
    pub max_line_distance: f64,
    pub max_line_distance_for_naively_paired_lines: f64,
    pub max_line_length: usize,
    pub minus_emph_style: Style,
    pub minus_empty_line_marker_style: Style,
    pub minus_file: Option<PathBuf>,
    pub minus_non_emph_style: Style,
    pub minus_style: Style,
    pub navigate: bool,
    pub navigate_regexp: Option<String>,
    pub null_style: Style,
    pub null_syntect_style: SyntectStyle,
    pub pager: Option<String>,
    pub paging_mode: PagingMode,
    pub plus_emph_style: Style,
    pub plus_empty_line_marker_style: Style,
    pub plus_file: Option<PathBuf>,
    pub plus_non_emph_style: Style,
    pub plus_style: Style,
    pub git_minus_style: Style,
    pub git_plus_style: Style,
    pub relative_paths: bool,
    pub show_themes: bool,
    pub side_by_side: bool,
    pub side_by_side_data: side_by_side::SideBySideData,
    pub syntax_dummy_theme: SyntaxTheme,
    pub syntax_set: SyntaxSet,
    pub syntax_theme: Option<SyntaxTheme>,
    pub tab_width: usize,
    pub tokenization_regex: Regex,
    pub true_color: bool,
    pub truncation_symbol: String,
    pub whitespace_error_style: Style,
    pub wrap_config: WrapConfig,
    pub zero_style: Style,
}

impl Config {
    pub fn get_style(&self, state: &State) -> &Style {
        match state {
            State::HunkMinus(_) => &self.minus_style,
            State::HunkPlus(_) => &self.plus_style,
            State::CommitMeta => &self.commit_style,
            State::FileMeta => &self.file_style,
            State::HunkHeader(_, _) => &self.hunk_header_style,
            State::SubmoduleLog => &self.file_style,
            _ => delta_unreachable("Unreachable code reached in get_style."),
        }
    }
}

impl From<cli::Opt> for Config {
    fn from(opt: cli::Opt) -> Self {
        let (
            minus_style,
            minus_emph_style,
            minus_non_emph_style,
            minus_empty_line_marker_style,
            zero_style,
            plus_style,
            plus_emph_style,
            plus_non_emph_style,
            plus_empty_line_marker_style,
            whitespace_error_style,
        ) = make_hunk_styles(&opt);

        let (
            commit_style,
            file_style,
            hunk_header_style,
            hunk_header_file_style,
            hunk_header_line_number_style,
        ) = make_commit_file_hunk_header_styles(&opt);

        let (
            line_numbers_minus_style,
            line_numbers_zero_style,
            line_numbers_plus_style,
            line_numbers_left_style,
            line_numbers_right_style,
        ) = make_line_number_styles(&opt);

        let max_line_distance_for_naively_paired_lines =
            env::get_env_var("DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES")
                .map(|s| s.parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0);

        let commit_regex = Regex::new(&opt.commit_regex).unwrap_or_else(|_| {
            fatal(format!(
                "Invalid commit-regex: {}. \
                 The value must be a valid Rust regular expression. \
                 See https://docs.rs/regex.",
                opt.commit_regex
            ));
        });

        let tokenization_regex = Regex::new(&opt.tokenization_regex).unwrap_or_else(|_| {
            fatal(format!(
                "Invalid word-diff-regex: {}. \
                 The value must be a valid Rust regular expression. \
                 See https://docs.rs/regex.",
                opt.tokenization_regex
            ));
        });

        let inline_hint_style = Style::from_str(
            &opt.inline_hint_style,
            None,
            None,
            opt.computed.true_color,
            false,
        );
        let git_minus_style = match opt.git_config_entries.get("color.diff.old") {
            Some(GitConfigEntry::Style(s)) => Style::from_git_str(s),
            _ => *style::GIT_DEFAULT_MINUS_STYLE,
        };
        let git_plus_style = match opt.git_config_entries.get("color.diff.new") {
            Some(GitConfigEntry::Style(s)) => Style::from_git_str(s),
            _ => *style::GIT_DEFAULT_PLUS_STYLE,
        };

        let file_added_label = opt.file_added_label;
        let file_copied_label = opt.file_copied_label;
        let file_modified_label = opt.file_modified_label;
        let file_removed_label = opt.file_removed_label;
        let file_renamed_label = opt.file_renamed_label;
        let hunk_label = opt.hunk_label;

        let line_fill_method = match opt.line_fill_method.as_deref() {
            // Note that "default" is not documented
            Some("ansi") | Some("default") | None => BgFillMethod::TryAnsiSequence,
            Some("spaces") => BgFillMethod::Spaces,
            _ => fatal("Invalid option for line-fill-method: Expected \"ansi\" or \"spaces\"."),
        };

        let side_by_side_data = side_by_side::SideBySideData::new_sbs(
            &opt.computed.decorations_width,
            &opt.computed.available_terminal_width,
        );
        let side_by_side_data = ansifill::UseFullPanelWidth::sbs_odd_fix(
            &opt.computed.decorations_width,
            &line_fill_method,
            side_by_side_data,
        );

        let navigate_regexp = if opt.navigate || opt.show_themes {
            Some(navigate::make_navigate_regexp(
                opt.show_themes,
                &file_modified_label,
                &file_added_label,
                &file_removed_label,
                &file_renamed_label,
                &hunk_label,
            ))
        } else {
            None
        };

        let wrap_max_lines_plus1 = adapt_wrap_max_lines_argument(opt.wrap_max_lines);

        Self {
            available_terminal_width: opt.computed.available_terminal_width,
            background_color_extends_to_terminal_width: opt
                .computed
                .background_color_extends_to_terminal_width,
            commit_style,
            color_only: opt.color_only,
            commit_regex,
            cwd_relative_to_repo_root: std::env::var("GIT_PREFIX").ok(),
            decorations_width: opt.computed.decorations_width,
            default_language: opt.default_language,
            diff_stat_align_width: opt.diff_stat_align_width,
            error_exit_code: 2, // Use 2 for error because diff uses 0 and 1 for non-error.
            file_added_label,
            file_copied_label,
            file_modified_label,
            file_removed_label,
            file_renamed_label,
            hunk_label,
            file_style,
            git_config: opt.git_config,
            git_config_entries: opt.git_config_entries,
            hunk_header_file_style,
            hunk_header_line_number_style,
            hunk_header_style,
            hunk_header_style_include_file_path: opt
                .hunk_header_style
                .split(' ')
                .any(|s| s == "file"),
            hunk_header_style_include_line_number: opt
                .hunk_header_style
                .split(' ')
                .any(|s| s == "line-number"),
            hyperlinks: opt.hyperlinks,
            hyperlinks_commit_link_format: opt.hyperlinks_commit_link_format,
            hyperlinks_file_link_format: opt.hyperlinks_file_link_format,
            inspect_raw_lines: opt.computed.inspect_raw_lines,
            inline_hint_style,
            keep_plus_minus_markers: opt.keep_plus_minus_markers,
            line_fill_method: if !opt.computed.stdout_is_term && !TESTING {
                // Don't write ANSI sequences (which rely on the width of the
                // current terminal) into a file. Also see UseFullPanelWidth.
                // But when testing always use given value.
                BgFillMethod::Spaces
            } else {
                line_fill_method
            },
            line_numbers: opt.line_numbers,
            line_numbers_format: LeftRight::new(
                opt.line_numbers_left_format,
                opt.line_numbers_right_format,
            ),
            line_numbers_style_leftright: LeftRight::new(
                line_numbers_left_style,
                line_numbers_right_style,
            ),
            line_numbers_style_minusplus: MinusPlus::new(
                line_numbers_minus_style,
                line_numbers_plus_style,
            ),
            line_numbers_zero_style,
            line_buffer_size: opt.line_buffer_size,
            max_line_distance: opt.max_line_distance,
            max_line_distance_for_naively_paired_lines,
            max_line_length: match (opt.side_by_side, wrap_max_lines_plus1) {
                (false, _) | (true, 1) => opt.max_line_length,
                // Ensure there is enough text to wrap, either don't truncate the input at all (0)
                // or ensure there is enough for the requested number of lines.
                // The input can contain ANSI sequences, so round up a bit. This is enough for
                // normal `git diff`, but might not be with ANSI heavy input.
                (true, 0) => 0,
                (true, wrap_max_lines) => {
                    let single_pane_width = opt.computed.available_terminal_width / 2;
                    let add_25_percent_or_term_width =
                        |x| x + std::cmp::max((x * 250) / 1000, single_pane_width) as usize;
                    std::cmp::max(
                        opt.max_line_length,
                        add_25_percent_or_term_width(single_pane_width * wrap_max_lines),
                    )
                }
            },
            minus_emph_style,
            minus_empty_line_marker_style,
            minus_file: opt.minus_file,
            minus_non_emph_style,
            minus_style,
            navigate: opt.navigate,
            navigate_regexp,
            null_style: Style::new(),
            null_syntect_style: SyntectStyle::default(),
            pager: opt.pager,
            paging_mode: opt.computed.paging_mode,
            plus_emph_style,
            plus_empty_line_marker_style,
            plus_file: opt.plus_file,
            plus_non_emph_style,
            plus_style,
            git_minus_style,
            git_plus_style,
            relative_paths: opt.relative_paths,
            show_themes: opt.show_themes,
            side_by_side: opt.side_by_side,
            side_by_side_data,
            syntax_dummy_theme: SyntaxTheme::default(),
            syntax_set: opt.computed.syntax_set,
            syntax_theme: opt.computed.syntax_theme,
            tab_width: opt.tab_width,
            tokenization_regex,
            true_color: opt.computed.true_color,
            truncation_symbol: format!("{}→{}", ansi::ANSI_SGR_REVERSE, ansi::ANSI_SGR_RESET),
            wrap_config: WrapConfig {
                left_symbol: ensure_display_width_1("wrap-left-symbol", opt.wrap_left_symbol),
                right_symbol: ensure_display_width_1("wrap-right-symbol", opt.wrap_right_symbol),
                right_prefix_symbol: ensure_display_width_1(
                    "wrap-right-prefix-symbol",
                    opt.wrap_right_prefix_symbol,
                ),
                use_wrap_right_permille: {
                    let arg = &opt.wrap_right_percent;
                    let percent = remove_percent_suffix(arg)
                        .parse::<f64>()
                        .unwrap_or_else(|err| {
                            fatal(format!(
                                "Could not parse wrap-right-percent argument {}: {}.",
                                &arg, err
                            ))
                        });
                    if percent.is_finite() && percent > 0.0 && percent < 100.0 {
                        (percent * 10.0).round() as usize
                    } else {
                        fatal("Invalid value for wrap-right-percent, not between 0 and 100.")
                    }
                },
                max_lines: wrap_max_lines_plus1,
                inline_hint_syntect_style: SyntectStyle::from_delta_style(inline_hint_style),
            },
            whitespace_error_style,
            zero_style,
        }
    }
}

fn make_hunk_styles(
    opt: &cli::Opt,
) -> (
    Style,
    Style,
    Style,
    Style,
    Style,
    Style,
    Style,
    Style,
    Style,
    Style,
) {
    let is_light_mode = opt.computed.is_light_mode;
    let true_color = opt.computed.true_color;
    let minus_style = Style::from_str(
        &opt.minus_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        false,
    );

    let minus_emph_style = Style::from_str(
        &opt.minus_emph_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_emph_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        true,
    );

    let minus_non_emph_style = Style::from_str(
        &opt.minus_non_emph_style,
        Some(minus_style),
        None,
        true_color,
        false,
    );

    // The style used to highlight a removed empty line when otherwise it would be invisible due to
    // lack of background color in minus-style.
    let minus_empty_line_marker_style = Style::from_str(
        &opt.minus_empty_line_marker_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        false,
    );

    let zero_style = Style::from_str(&opt.zero_style, None, None, true_color, false);

    let plus_style = Style::from_str(
        &opt.plus_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        false,
    );

    let plus_emph_style = Style::from_str(
        &opt.plus_emph_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_emph_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        true,
    );

    let plus_non_emph_style = Style::from_str(
        &opt.plus_non_emph_style,
        Some(plus_style),
        None,
        true_color,
        false,
    );

    // The style used to highlight an added empty line when otherwise it would be invisible due to
    // lack of background color in plus-style.
    let plus_empty_line_marker_style = Style::from_str(
        &opt.plus_empty_line_marker_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
        false,
    );

    let whitespace_error_style =
        Style::from_str(&opt.whitespace_error_style, None, None, true_color, false);

    (
        minus_style,
        minus_emph_style,
        minus_non_emph_style,
        minus_empty_line_marker_style,
        zero_style,
        plus_style,
        plus_emph_style,
        plus_non_emph_style,
        plus_empty_line_marker_style,
        whitespace_error_style,
    )
}

fn make_line_number_styles(opt: &cli::Opt) -> (Style, Style, Style, Style, Style) {
    let true_color = opt.computed.true_color;
    let line_numbers_left_style =
        Style::from_str(&opt.line_numbers_left_style, None, None, true_color, false);

    let line_numbers_minus_style =
        Style::from_str(&opt.line_numbers_minus_style, None, None, true_color, false);

    let line_numbers_zero_style =
        Style::from_str(&opt.line_numbers_zero_style, None, None, true_color, false);

    let line_numbers_plus_style =
        Style::from_str(&opt.line_numbers_plus_style, None, None, true_color, false);

    let line_numbers_right_style =
        Style::from_str(&opt.line_numbers_right_style, None, None, true_color, false);

    (
        line_numbers_minus_style,
        line_numbers_zero_style,
        line_numbers_plus_style,
        line_numbers_left_style,
        line_numbers_right_style,
    )
}

fn make_commit_file_hunk_header_styles(opt: &cli::Opt) -> (Style, Style, Style, Style, Style) {
    let true_color = opt.computed.true_color;
    (
        Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
            &opt.commit_style,
            None,
            Some(&opt.commit_decoration_style),
            opt.deprecated_commit_color.as_deref(),
            true_color,
            false,
        ),
        Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
            &opt.file_style,
            None,
            Some(&opt.file_decoration_style),
            opt.deprecated_file_color.as_deref(),
            true_color,
            false,
        ),
        Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
            &opt.hunk_header_style,
            None,
            Some(&opt.hunk_header_decoration_style),
            opt.deprecated_hunk_color.as_deref(),
            true_color,
            false,
        ),
        Style::from_str_with_handling_of_special_decoration_attributes(
            &opt.hunk_header_file_style,
            None,
            None,
            true_color,
            false,
        ),
        Style::from_str_with_handling_of_special_decoration_attributes(
            &opt.hunk_header_line_number_style,
            None,
            None,
            true_color,
            false,
        ),
    )
}

/// Did the user supply `option` on the command line?
pub fn user_supplied_option(option: &str, arg_matches: &clap::ArgMatches) -> bool {
    arg_matches.occurrences_of(option) > 0
}

pub fn delta_unreachable(message: &str) -> ! {
    fatal(format!(
        "{} This should not be possible. \
         Please report the bug at https://github.com/dandavison/delta/issues.",
        message
    ));
}

#[cfg(test)]
// Usual length of the header returned by `run_delta()`, often `skip()`-ed.
pub const HEADER_LEN: usize = 7;

#[cfg(test)]
pub mod tests {
    use crate::bat_utils::output::PagingMode;
    use crate::cli;
    use crate::tests::integration_test_utils;
    use std::fs::remove_file;

    #[test]
    fn test_get_computed_values_from_config() {
        let git_config_contents = b"
[delta]
    true-color = never
    width = 100
    inspect-raw-lines = true
    paging = never
    syntax-theme = None
";
        let git_config_path = "delta__test_get_true_color_from_config.gitconfig";
        let config = integration_test_utils::make_config_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(config.true_color, false);
        assert_eq!(config.decorations_width, cli::Width::Fixed(100));
        assert_eq!(config.background_color_extends_to_terminal_width, true);
        assert_eq!(config.inspect_raw_lines, cli::InspectRawLines::True);
        assert_eq!(config.paging_mode, PagingMode::Never);
        assert!(config.syntax_theme.is_none());
        // syntax_set doesn't depend on gitconfig.
        remove_file(git_config_path).unwrap();
    }
}
