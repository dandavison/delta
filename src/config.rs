use std::collections::HashMap;
use std::path::PathBuf;

use clap::parser::ValueSource;
use regex::Regex;
use syntect::highlighting::Style as SyntectStyle;
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;

use crate::ansi;
use crate::cli;
use crate::color::{self, ColorMode};
use crate::delta::State;
use crate::fatal;
use crate::features::navigate;
use crate::features::side_by_side::{self, ansifill, LeftRight};
use crate::git_config::GitConfig;
use crate::handlers;
use crate::handlers::blame::parse_blame_line_numbers;
use crate::handlers::blame::BlameLineNumbers;
use crate::minusplus::MinusPlus;
use crate::paint::BgFillMethod;
use crate::parse_styles;
use crate::style;
use crate::style::Style;
use crate::tests::TESTING;
use crate::utils;
use crate::utils::bat::output::PagingMode;
use crate::utils::regex_replacement::RegexReplacement;
use crate::wrapping::WrapConfig;

pub const INLINE_SYMBOL_WIDTH_1: usize = 1;

// Used if an invalid default-language was specified.
pub const SYNTAX_FALLBACK_LANG: &str = "txt";

#[cfg_attr(test, derive(Clone))]
pub struct Config {
    pub available_terminal_width: usize,
    pub background_color_extends_to_terminal_width: bool,
    pub blame_code_style: Option<Style>,
    pub blame_format: String,
    pub blame_separator_format: BlameLineNumbers,
    pub blame_palette: Vec<String>,
    pub blame_separator_style: Option<Style>,
    pub blame_timestamp_format: String,
    pub blame_timestamp_output_format: Option<String>,
    pub color_only: bool,
    pub commit_regex: Regex,
    pub commit_style: Style,
    pub cwd_of_delta_process: Option<PathBuf>,
    pub cwd_of_user_shell_process: Option<PathBuf>,
    pub cwd_relative_to_repo_root: Option<String>,
    pub decorations_width: cli::Width,
    pub default_language: String,
    pub diff_args: String,
    pub diff_stat_align_width: usize,
    pub error_exit_code: i32,
    pub file_added_label: String,
    pub file_copied_label: String,
    pub file_modified_label: String,
    pub file_removed_label: String,
    pub file_renamed_label: String,
    pub file_regex_replacement: Option<RegexReplacement>,
    pub right_arrow: String,
    pub file_style: Style,
    pub git_config: Option<GitConfig>,
    pub git_minus_style: Style,
    pub git_plus_style: Style,
    pub grep_context_line_style: Style,
    pub grep_file_style: Style,
    pub classic_grep_header_file_style: Style,
    pub classic_grep_header_style: Style,
    pub ripgrep_header_style: Style,
    pub grep_line_number_style: Style,
    pub grep_match_line_style: Style,
    pub grep_match_word_style: Style,
    pub grep_output_type: Option<GrepType>,
    pub grep_separator_symbol: String,
    pub handle_merge_conflicts: bool,
    pub hostname: Option<String>,
    pub hunk_header_file_style: Style,
    pub hunk_header_line_number_style: Style,
    pub hunk_header_style_include_file_path: HunkHeaderIncludeFilePath,
    pub hunk_header_style_include_line_number: HunkHeaderIncludeLineNumber,
    pub hunk_header_style_include_code_fragment: HunkHeaderIncludeCodeFragment,
    pub hunk_header_style: Style,
    pub hunk_label: String,
    pub hyperlinks_commit_link_format: Option<String>,
    pub hyperlinks_file_link_format: String,
    pub hyperlinks: bool,
    pub inline_hint_style: Style,
    pub inspect_raw_lines: cli::InspectRawLines,
    pub keep_plus_minus_markers: bool,
    pub line_buffer_size: usize,
    pub line_fill_method: BgFillMethod,
    pub line_numbers_format: LeftRight<String>,
    pub line_numbers_style_leftright: LeftRight<Style>,
    pub line_numbers_style_minusplus: MinusPlus<Style>,
    pub line_numbers_zero_style: Style,
    pub line_numbers: bool,
    pub styles_map: Option<HashMap<style::AnsiTermStyleEqualityKey, Style>>,
    pub max_line_distance_for_naively_paired_lines: f64,
    pub max_line_distance: f64,
    pub max_line_length: usize,
    pub max_syntax_length: usize,
    pub merge_conflict_begin_symbol: String,
    pub merge_conflict_ours_diff_header_style: Style,
    pub merge_conflict_theirs_diff_header_style: Style,
    pub merge_conflict_end_symbol: String,
    pub minus_emph_style: Style,
    pub minus_empty_line_marker_style: Style,
    pub minus_file: Option<PathBuf>,
    pub minus_non_emph_style: Style,
    pub minus_style: Style,
    pub navigate_regex: Option<String>,
    pub navigate: bool,
    pub null_style: Style,
    pub null_syntect_style: SyntectStyle,
    pub pager: Option<String>,
    pub paging_mode: PagingMode,
    pub plus_emph_style: Style,
    pub plus_empty_line_marker_style: Style,
    pub plus_file: Option<PathBuf>,
    pub plus_non_emph_style: Style,
    pub plus_style: Style,
    pub relative_paths: bool,
    pub show_themes: bool,
    pub side_by_side_data: side_by_side::SideBySideData,
    pub side_by_side: bool,
    pub syntax_set: SyntaxSet,
    pub syntax_theme: Option<SyntaxTheme>,
    pub tab_cfg: utils::tabs::TabCfg,
    pub tokenization_regex: Regex,
    pub true_color: bool,
    pub truncation_symbol: String,
    pub whitespace_error_style: Style,
    pub wrap_config: WrapConfig,
    pub zero_style: Style,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum GrepType {
    Ripgrep,
    Classic,
}

#[cfg_attr(test, derive(Clone))]
pub enum HunkHeaderIncludeFilePath {
    Yes,
    No,
}

#[cfg_attr(test, derive(Clone))]
pub enum HunkHeaderIncludeLineNumber {
    Yes,
    No,
}

#[cfg_attr(test, derive(Clone))]
pub enum HunkHeaderIncludeCodeFragment {
    Yes,
    No,
}

impl Config {
    pub fn get_style(&self, state: &State) -> &Style {
        match state {
            State::HunkMinus(_, _) => &self.minus_style,
            State::HunkZero(_, _) => &self.zero_style,
            State::HunkPlus(_, _) => &self.plus_style,
            State::CommitMeta => &self.commit_style,
            State::DiffHeader(_) => &self.file_style,
            State::Grep(GrepType::Ripgrep, _, _, _) => &self.classic_grep_header_style,
            State::HunkHeader(_, _, _, _) => &self.hunk_header_style,
            State::SubmoduleLog => &self.file_style,
            _ => delta_unreachable("Unreachable code reached in get_style."),
        }
    }

    pub fn git_config(&self) -> Option<&GitConfig> {
        self.git_config.as_ref()
    }
}

impl From<cli::Opt> for Config {
    fn from(opt: cli::Opt) -> Self {
        let mut styles = parse_styles::parse_styles(&opt);
        let styles_map = parse_styles::parse_styles_map(&opt);

        let wrap_config = WrapConfig::from_opt(&opt, styles["inline-hint-style"]);

        let max_line_distance_for_naively_paired_lines = opt
            .env
            .experimental_max_line_distance_for_naively_paired_lines
            .as_ref()
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

        let blame_palette = make_blame_palette(opt.blame_palette, opt.computed.color_mode);

        if blame_palette.is_empty() {
            fatal("Option 'blame-palette' must not be empty.")
        }

        let file_added_label = opt.file_added_label;
        let file_copied_label = opt.file_copied_label;
        let file_modified_label = opt.file_modified_label;
        let file_removed_label = opt.file_removed_label;
        let file_renamed_label = opt.file_renamed_label;
        let right_arrow = opt.right_arrow;
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

        let navigate_regex = if (opt.navigate || opt.show_themes)
            && (opt.navigate_regex.is_none() || opt.navigate_regex == Some("".to_string()))
        {
            Some(navigate::make_navigate_regex(
                opt.show_themes,
                &file_modified_label,
                &file_added_label,
                &file_removed_label,
                &file_renamed_label,
                &hunk_label,
            ))
        } else {
            opt.navigate_regex
        };

        let grep_output_type = match opt.grep_output_type.as_deref() {
            Some("ripgrep") => Some(GrepType::Ripgrep),
            Some("classic") => Some(GrepType::Classic),
            None => None,
            _ => fatal("Invalid option for grep-output-type: Expected \"ripgrep\" or \"classic\"."),
        };

        #[cfg(not(test))]
        let cwd_of_delta_process = opt.env.current_dir;
        #[cfg(test)]
        let cwd_of_delta_process = Some(utils::path::fake_delta_cwd_for_tests());

        let cwd_relative_to_repo_root = opt.env.git_prefix;

        let cwd_of_user_shell_process = utils::path::cwd_of_user_shell_process(
            cwd_of_delta_process.as_ref(),
            cwd_relative_to_repo_root.as_deref(),
        );

        Self {
            available_terminal_width: opt.computed.available_terminal_width,
            background_color_extends_to_terminal_width: opt
                .computed
                .background_color_extends_to_terminal_width,
            blame_format: opt.blame_format,
            blame_code_style: styles.remove("blame-code-style"),
            blame_palette,
            blame_separator_format: parse_blame_line_numbers(&opt.blame_separator_format),
            blame_separator_style: styles.remove("blame-separator-style"),
            blame_timestamp_format: opt.blame_timestamp_format,
            blame_timestamp_output_format: opt.blame_timestamp_output_format,
            commit_style: styles["commit-style"],
            color_only: opt.color_only,
            commit_regex,
            cwd_of_delta_process,
            cwd_of_user_shell_process,
            cwd_relative_to_repo_root,
            decorations_width: opt.computed.decorations_width,
            default_language: opt.default_language,
            diff_args: opt.diff_args,
            diff_stat_align_width: opt.diff_stat_align_width,
            error_exit_code: 2, // Use 2 for error because diff uses 0 and 1 for non-error.
            file_added_label,
            file_copied_label,
            file_modified_label,
            file_removed_label,
            file_renamed_label,
            file_regex_replacement: opt
                .file_regex_replacement
                .as_deref()
                .and_then(RegexReplacement::from_sed_command),
            right_arrow,
            hunk_label,
            file_style: styles["file-style"],
            git_config: opt.git_config,
            grep_context_line_style: styles["grep-context-line-style"],
            grep_file_style: styles["grep-file-style"],
            classic_grep_header_file_style: styles["classic-grep-header-file-style"],
            classic_grep_header_style: styles["classic-grep-header-style"],
            ripgrep_header_style: styles["ripgrep-header-style"],
            grep_line_number_style: styles["grep-line-number-style"],
            grep_match_line_style: styles["grep-match-line-style"],
            grep_match_word_style: styles["grep-match-word-style"],
            grep_output_type,
            grep_separator_symbol: opt.grep_separator_symbol,
            handle_merge_conflicts: !opt.raw,
            hostname: opt.env.hostname,
            hunk_header_file_style: styles["hunk-header-file-style"],
            hunk_header_line_number_style: styles["hunk-header-line-number-style"],
            hunk_header_style: styles["hunk-header-style"],
            hunk_header_style_include_file_path: if opt
                .hunk_header_style
                .split(' ')
                .any(|s| s == "file")
            {
                HunkHeaderIncludeFilePath::Yes
            } else {
                HunkHeaderIncludeFilePath::No
            },
            hunk_header_style_include_line_number: if opt
                .hunk_header_style
                .split(' ')
                .any(|s| s == "line-number")
            {
                HunkHeaderIncludeLineNumber::Yes
            } else {
                HunkHeaderIncludeLineNumber::No
            },
            hunk_header_style_include_code_fragment: if opt
                .hunk_header_style
                .split(' ')
                .any(|s| s == "omit-code-fragment")
            {
                HunkHeaderIncludeCodeFragment::No
            } else {
                HunkHeaderIncludeCodeFragment::Yes
            },
            hyperlinks: opt.hyperlinks,
            hyperlinks_commit_link_format: opt.hyperlinks_commit_link_format,
            hyperlinks_file_link_format: opt.hyperlinks_file_link_format,
            inspect_raw_lines: opt.computed.inspect_raw_lines,
            inline_hint_style: styles["inline-hint-style"],
            keep_plus_minus_markers: opt.keep_plus_minus_markers,
            line_fill_method: if !opt.computed.stdout_is_term && !TESTING {
                // Don't write ANSI sequences (which rely on the width of the
                // current terminal) into a file. Also see UseFullPanelWidth.
                // But when testing always use given value.
                BgFillMethod::Spaces
            } else {
                line_fill_method
            },
            line_numbers: opt.line_numbers && !handlers::hunk::is_word_diff(),
            line_numbers_format: LeftRight::new(
                opt.line_numbers_left_format,
                opt.line_numbers_right_format,
            ),
            line_numbers_style_leftright: LeftRight::new(
                styles["line-numbers-left-style"],
                styles["line-numbers-right-style"],
            ),
            line_numbers_style_minusplus: MinusPlus::new(
                styles["line-numbers-minus-style"],
                styles["line-numbers-plus-style"],
            ),
            line_numbers_zero_style: styles["line-numbers-zero-style"],
            line_buffer_size: opt.line_buffer_size,
            max_line_distance: opt.max_line_distance,
            max_line_distance_for_naively_paired_lines,
            max_line_length: if opt.side_by_side {
                wrap_config.config_max_line_length(
                    opt.max_line_length,
                    opt.computed.available_terminal_width,
                )
            } else {
                opt.max_line_length
            },
            max_syntax_length: opt.max_syntax_length,
            merge_conflict_begin_symbol: opt.merge_conflict_begin_symbol,
            merge_conflict_ours_diff_header_style: styles["merge-conflict-ours-diff-header-style"],
            merge_conflict_theirs_diff_header_style: styles
                ["merge-conflict-theirs-diff-header-style"],
            merge_conflict_end_symbol: opt.merge_conflict_end_symbol,
            minus_emph_style: styles["minus-emph-style"],
            minus_empty_line_marker_style: styles["minus-empty-line-marker-style"],
            minus_file: opt.minus_file,
            minus_non_emph_style: styles["minus-non-emph-style"],
            minus_style: styles["minus-style"],
            navigate: opt.navigate,
            navigate_regex,
            null_style: Style::new(),
            null_syntect_style: SyntectStyle::default(),
            pager: opt.pager,
            paging_mode: opt.computed.paging_mode,
            plus_emph_style: styles["plus-emph-style"],
            plus_empty_line_marker_style: styles["plus-empty-line-marker-style"],
            plus_file: opt.plus_file,
            plus_non_emph_style: styles["plus-non-emph-style"],
            plus_style: styles["plus-style"],
            git_minus_style: styles["git-minus-style"],
            git_plus_style: styles["git-plus-style"],
            relative_paths: opt.relative_paths,
            show_themes: opt.show_themes,
            side_by_side: opt.side_by_side && !handlers::hunk::is_word_diff(),
            side_by_side_data,
            styles_map,
            syntax_set: opt.computed.syntax_set,
            syntax_theme: opt.computed.syntax_theme,
            tab_cfg: utils::tabs::TabCfg::new(opt.tab_width),
            tokenization_regex,
            true_color: opt.computed.true_color,
            truncation_symbol: format!("{}→{}", ansi::ANSI_SGR_REVERSE, ansi::ANSI_SGR_RESET),
            wrap_config,
            whitespace_error_style: styles["whitespace-error-style"],
            zero_style: styles["zero-style"],
        }
    }
}

fn make_blame_palette(blame_palette: Option<String>, mode: ColorMode) -> Vec<String> {
    match (blame_palette, mode) {
        (Some(string), _) => string
            .split_whitespace()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>(),
        (None, ColorMode::Light) => color::LIGHT_THEME_BLAME_PALETTE
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        (None, ColorMode::Dark) => color::DARK_THEME_BLAME_PALETTE
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
    }
}

/// Did the user supply `option` on the command line?
pub fn user_supplied_option(option: &str, arg_matches: &clap::ArgMatches) -> bool {
    arg_matches.value_source(option) == Some(ValueSource::CommandLine)
}

pub fn delta_unreachable(message: &str) -> ! {
    fatal(format!(
        "{message} This should not be possible. \
         Please report the bug at https://github.com/dandavison/delta/issues.",
    ));
}

#[cfg(test)]
// Usual length of the header returned by `run_delta()`, often `skip()`-ed.
pub const HEADER_LEN: usize = 7;

#[cfg(test)]
pub mod tests {
    use crate::cli;
    use crate::tests::integration_test_utils;
    use crate::utils::bat::output::PagingMode;
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
        assert!(!config.true_color);
        assert_eq!(config.decorations_width, cli::Width::Fixed(100));
        assert!(config.background_color_extends_to_terminal_width);
        assert_eq!(config.inspect_raw_lines, cli::InspectRawLines::True);
        assert_eq!(config.paging_mode, PagingMode::Never);
        assert!(config.syntax_theme.is_none());
        // syntax_set doesn't depend on gitconfig.
        remove_file(git_config_path).unwrap();
    }
}
