use std::path::PathBuf;
use std::process;

use regex::Regex;
use structopt::clap;
use syntect::highlighting::Style as SyntectStyle;
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;

use crate::bat::output::PagingMode;
use crate::cli;
use crate::color;
use crate::delta::State;
use crate::env;
use crate::features::side_by_side;
use crate::style::Style;

pub struct Config {
    pub available_terminal_width: usize,
    pub background_color_extends_to_terminal_width: bool,
    pub commit_style: Style,
    pub decorations_width: cli::Width,
    pub file_added_label: String,
    pub file_modified_label: String,
    pub file_removed_label: String,
    pub file_renamed_label: String,
    pub file_style: Style,
    pub keep_plus_minus_markers: bool,
    pub hunk_header_style: Style,
    pub max_buffered_lines: usize,
    pub max_line_distance: f64,
    pub max_line_distance_for_naively_paired_lines: f64,
    pub minus_emph_style: Style,
    pub minus_empty_line_marker_style: Style,
    pub minus_file: Option<PathBuf>,
    pub minus_non_emph_style: Style,
    pub minus_style: Style,
    pub navigate: bool,
    pub null_style: Style,
    pub null_syntect_style: SyntectStyle,
    pub line_numbers_left_format: String,
    pub line_numbers_left_style: Style,
    pub line_numbers_minus_style: Style,
    pub line_numbers_plus_style: Style,
    pub line_numbers_right_format: String,
    pub line_numbers_right_style: Style,
    pub line_numbers_zero_style: Style,
    pub paging_mode: PagingMode,
    pub plus_emph_style: Style,
    pub plus_empty_line_marker_style: Style,
    pub plus_file: Option<PathBuf>,
    pub plus_non_emph_style: Style,
    pub plus_style: Style,
    pub line_numbers: bool,
    pub side_by_side: bool,
    pub side_by_side_data: side_by_side::SideBySideData,
    pub syntax_dummy_theme: SyntaxTheme,
    pub syntax_set: SyntaxSet,
    pub syntax_theme: Option<SyntaxTheme>,
    pub tab_width: usize,
    pub true_color: bool,
    pub truncation_symbol: String,
    pub tokenization_regex: Regex,
    pub whitespace_error_style: Style,
    pub zero_style: Style,
}

impl Config {
    pub fn get_style(&self, state: &State) -> &Style {
        match state {
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

        let (commit_style, file_style, hunk_header_style) =
            make_commit_file_hunk_header_styles(&opt);

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

        Self {
            available_terminal_width: opt.computed.available_terminal_width,
            background_color_extends_to_terminal_width: opt
                .computed
                .background_color_extends_to_terminal_width,
            commit_style,
            decorations_width: opt.computed.decorations_width,
            file_added_label: opt.file_added_label,
            file_modified_label: opt.file_modified_label,
            file_removed_label: opt.file_removed_label,
            file_renamed_label: opt.file_renamed_label,
            file_style,
            keep_plus_minus_markers: opt.keep_plus_minus_markers,
            hunk_header_style,
            max_buffered_lines: 32,
            max_line_distance: opt.max_line_distance,
            max_line_distance_for_naively_paired_lines,
            minus_emph_style,
            minus_empty_line_marker_style,
            minus_file: opt.minus_file.map(|s| s.clone()),
            minus_non_emph_style,
            minus_style,
            navigate: opt.navigate,
            null_style: Style::new(),
            null_syntect_style: SyntectStyle::default(),
            line_numbers_left_format: opt.line_numbers_left_format,
            line_numbers_left_style,
            line_numbers_minus_style,
            line_numbers_plus_style,
            line_numbers_right_format: opt.line_numbers_right_format,
            line_numbers_right_style,
            line_numbers_zero_style,
            paging_mode: opt.computed.paging_mode,
            plus_emph_style,
            plus_empty_line_marker_style,
            plus_file: opt.plus_file.map(|s| s.clone()),
            plus_non_emph_style,
            plus_style,
            line_numbers: opt.line_numbers,
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

fn make_hunk_styles<'a>(
    opt: &'a cli::Opt,
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

fn make_line_number_styles<'a>(opt: &'a cli::Opt) -> (Style, Style, Style, Style, Style) {
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

fn make_commit_file_hunk_header_styles(opt: &cli::Opt) -> (Style, Style, Style) {
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
    )
}

/// Did the user supply `option` on the command line?
pub fn user_supplied_option(option: &str, arg_matches: &clap::ArgMatches) -> bool {
    arg_matches.occurrences_of(option) > 0
}

pub fn delta_unreachable(message: &str) -> ! {
    eprintln!(
        "{} This should not be possible. \
         Please report the bug at https://github.com/dandavison/delta/issues.",
        message
    );
    process::exit(1);
}
