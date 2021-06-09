use std::collections::HashMap;
use std::path::PathBuf;
use std::process;

use regex::Regex;
use structopt::clap;
use syntect::highlighting::Style as SyntectStyle;
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;

use crate::bat_utils::output::PagingMode;
use crate::cli;
use crate::color;
use crate::delta::State;
use crate::env;
use crate::features::navigate;
use crate::features::side_by_side;
use crate::git_config::GitConfigEntry;
use crate::style::{self, Style};

pub struct Config {
    pub available_terminal_width: usize,
    pub background_color_extends_to_terminal_width: bool,
    pub commit_style: Style,
    pub color_only: bool,
    pub commit_regex: Regex,
    pub cwd_relative_to_repo_root: Option<String>,
    pub decorations_width: cli::Width,
    pub diff_stat_align_width: usize,
    pub error_exit_code: i32,
    pub file_added_label: String,
    pub file_copied_label: String,
    pub file_modified_label: String,
    pub file_removed_label: String,
    pub file_renamed_label: String,
    pub file_style: Style,
    pub git_config_entries: HashMap<String, GitConfigEntry>,
    pub hunk_header_file_style: Style,
    pub hunk_header_line_number_style: Style,
    pub hunk_header_style: Style,
    pub hunk_header_style_include_file_path: bool,
    pub hunk_header_style_include_line_number: bool,
    pub hyperlinks: bool,
    pub hyperlinks_commit_link_format: Option<String>,
    pub hyperlinks_file_link_format: String,
    pub inspect_raw_lines: cli::InspectRawLines,
    pub keep_plus_minus_markers: bool,
    pub line_numbers: bool,
    pub line_numbers_left_format: String,
    pub line_numbers_left_style: Style,
    pub line_numbers_minus_style: Style,
    pub line_numbers_plus_style: Style,
    pub line_numbers_right_format: String,
    pub line_numbers_right_style: Style,
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
    pub zero_style: Style,
}

impl Config {
    pub fn get_style(&self, state: &State) -> &Style {
        match state {
            State::HunkMinus(_) => &self.minus_style,
            State::HunkPlus(_) => &self.plus_style,
            State::CommitMeta => &self.commit_style,
            State::FileMeta => &self.file_style,
            State::HunkHeader => &self.hunk_header_style,
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
            eprintln!(
                "Invalid commit-regex: {}. \
                 The value must be a valid Rust regular expression. \
                 See https://docs.rs/regex.",
                opt.commit_regex
            );
            process::exit(1);
        });

        let tokenization_regex = Regex::new(&opt.tokenization_regex).unwrap_or_else(|_| {
            eprintln!(
                "Invalid word-diff-regex: {}. \
                 The value must be a valid Rust regular expression. \
                 See https://docs.rs/regex.",
                opt.tokenization_regex
            );
            process::exit(1);
        });

        let side_by_side_data = side_by_side::SideBySideData::new(
            &opt.computed.decorations_width,
            &opt.computed.available_terminal_width,
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

        let navigate_regexp = if opt.navigate || opt.show_themes {
            Some(navigate::make_navigate_regexp(
                opt.show_themes,
                &file_modified_label,
                &file_added_label,
                &file_removed_label,
                &file_renamed_label,
            ))
        } else {
            None
        };

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
            diff_stat_align_width: opt.diff_stat_align_width,
            error_exit_code: 2, // Use 2 for error because diff uses 0 and 1 for non-error.
            file_added_label,
            file_copied_label,
            file_modified_label,
            file_removed_label,
            file_renamed_label,
            file_style,
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
            keep_plus_minus_markers: opt.keep_plus_minus_markers,
            line_numbers: opt.line_numbers,
            line_numbers_left_format: opt.line_numbers_left_format,
            line_numbers_left_style,
            line_numbers_minus_style,
            line_numbers_plus_style,
            line_numbers_right_format: opt.line_numbers_right_format,
            line_numbers_right_style,
            line_numbers_zero_style,
            line_buffer_size: opt.line_buffer_size,
            max_line_distance: opt.max_line_distance,
            max_line_distance_for_naively_paired_lines,
            max_line_length: opt.max_line_length,
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
            truncation_symbol: "→".to_string(),
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
    let error_exit_code = 2; // This is also stored in Config.
    eprintln!(
        "{} This should not be possible. \
         Please report the bug at https://github.com/dandavison/delta/issues.",
        message
    );
    process::exit(error_exit_code);
}
