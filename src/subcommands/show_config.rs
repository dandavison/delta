use std::io::Write;

use clap::ValueEnum;
use itertools::Itertools;

use crate::cli;
use crate::config;
use crate::features::side_by_side::{Left, Right};
use crate::minusplus::*;
use crate::paint::BgFillMethod;
use crate::style;
use crate::utils::bat::output::PagingMode;

pub fn show_config(opt: cli::Opt, writer: &mut dyn Write) -> std::io::Result<()> {
    let options = option_values(&opt);
    let config = config::Config::from(opt);
    let mut output = Vec::new();
    write_config(&config, &mut output)?;
    write_additional_styles(&config, &mut output)?;
    write_decoration_styles(&config, &mut output)?;
    write_additional_values(&config, &mut output)?;
    for (key, value) in options {
        writeln!(output, "    {key:29} = {value}")?;
    }
    for line in output.split_inclusive(|&byte| byte == b'\n').sorted() {
        writer.write_all(line)?;
    }
    Ok(())
}

fn option_values(opt: &cli::Opt) -> Vec<(String, String)> {
    let mut options = Vec::new();
    macro_rules! values {
        ($($field:ident),* $(,)?) => {
            $(options.push((stringify!($field).replace('_', "-"), opt.$field.to_string()));)*
        };
    }
    macro_rules! strings {
        ($($field:ident),* $(,)?) => {
            $(options.push((stringify!($field).replace('_', "-"), format_option_value(&opt.$field)));)*
        };
    }
    macro_rules! optional_strings {
        ($($field:ident),* $(,)?) => {
            $(options.push((stringify!($field).replace('_', "-"), format_option_value(opt.$field.as_deref().unwrap_or(""))));)*
        };
    }
    values!(
        dark,
        light,
        diff_highlight,
        diff_so_fancy,
        no_gitconfig,
        raw
    );
    strings!(
        config,
        blame_separator_format,
        wrap_max_lines,
        wrap_right_percent,
    );
    optional_strings!(features, map_styles);
    options.push((
        "file-transformation".to_string(),
        format_option_value(opt.file_regex_replacement.as_deref().unwrap_or("")),
    ));
    options.push((
        "detect-dark-light".to_string(),
        opt.detect_dark_light
            .to_possible_value()
            .unwrap()
            .get_name()
            .to_string(),
    ));

    options
}

fn write_config(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
    // styles first
    writeln!(
        writer,
        "    commit-style                  = {commit_style}
    file-style                    = {file_style}
    hunk-header-style             = {hunk_header_style}
    minus-style                   = {minus_style}
    minus-non-emph-style          = {minus_non_emph_style}
    minus-emph-style              = {minus_emph_style}
    minus-empty-line-marker-style = {minus_empty_line_marker_style}
    zero-style                    = {zero_style}
    plus-style                    = {plus_style}
    plus-non-emph-style           = {plus_non_emph_style}
    plus-emph-style               = {plus_emph_style}
    plus-empty-line-marker-style  = {plus_empty_line_marker_style}
    grep-file-style               = {grep_file_style}
    grep-line-number-style        = {grep_line_number_style}
    whitespace-error-style        = {whitespace_error_style}
    blame-palette                 = {blame_palette}",
        blame_palette = config
            .blame_palette
            .iter()
            .map(|s| style::paint_color_string(s, config.true_color, config.git_config()))
            .join(" "),
        commit_style = config.commit_style.to_painted_string(),
        file_style = config.file_style.to_painted_string(),
        hunk_header_style = config.hunk_header_style.paint(format!(
            "{}{}{}{}",
            if matches!(
                config.hunk_header_style_include_file_path,
                config::HunkHeaderIncludeFilePath::Yes
            ) {
                "file "
            } else {
                ""
            },
            if matches!(
                config.hunk_header_style_include_line_number,
                config::HunkHeaderIncludeLineNumber::Yes
            ) {
                "line-number "
            } else {
                ""
            },
            if matches!(
                config.hunk_header_style_include_code_fragment,
                config::HunkHeaderIncludeCodeFragment::No
            ) {
                "omit-code-fragment "
            } else {
                ""
            },
            config.hunk_header_style,
        )),
        minus_emph_style = config.minus_emph_style.to_painted_string(),
        minus_empty_line_marker_style = config.minus_empty_line_marker_style.to_painted_string(),
        minus_non_emph_style = config.minus_non_emph_style.to_painted_string(),
        minus_style = config.minus_style.to_painted_string(),
        plus_emph_style = config.plus_emph_style.to_painted_string(),
        plus_empty_line_marker_style = config.plus_empty_line_marker_style.to_painted_string(),
        plus_non_emph_style = config.plus_non_emph_style.to_painted_string(),
        plus_style = config.plus_style.to_painted_string(),
        grep_file_style = config.grep_file_style.to_painted_string(),
        grep_line_number_style = config.grep_line_number_style.to_painted_string(),
        whitespace_error_style = config.whitespace_error_style.to_painted_string(),
        zero_style = config.zero_style.to_painted_string(),
    )?;
    // Everything else
    writeln!(
        writer,
        "    true-color                    = {true_color}
    file-added-label              = {file_added_label}
    file-modified-label           = {file_modified_label}
    file-removed-label            = {file_removed_label}
    file-renamed-label            = {file_renamed_label}
    right-arrow                   = {right_arrow}",
        true_color = config.true_color,
        file_added_label = format_option_value(&config.file_added_label),
        file_modified_label = format_option_value(&config.file_modified_label),
        file_removed_label = format_option_value(&config.file_removed_label),
        file_renamed_label = format_option_value(&config.file_renamed_label),
        right_arrow = format_option_value(&config.right_arrow),
    )?;
    writeln!(
        writer,
        "    hyperlinks                    = {hyperlinks}",
        hyperlinks = config.hyperlinks
    )?;
    writeln!(
        writer,
        "    hyperlinks-file-link-format   = {hyperlinks_file_link_format}",
        hyperlinks_file_link_format = format_option_value(&config.hyperlinks_file_link_format),
    )?;
    writeln!(
        writer,
        "    inspect-raw-lines             = {inspect_raw_lines}
    keep-plus-minus-markers       = {keep_plus_minus_markers}",
        inspect_raw_lines = match config.inspect_raw_lines {
            cli::InspectRawLines::True => "true",
            cli::InspectRawLines::False => "false",
        },
        keep_plus_minus_markers = config.keep_plus_minus_markers,
    )?;
    writeln!(
        writer,
        "    line-numbers                  = {line_numbers}",
        line_numbers = config.line_numbers
    )?;
    writeln!(
        writer,
        "    line-numbers-minus-style      = {line_numbers_minus_style}
    line-numbers-zero-style       = {line_numbers_zero_style}
    line-numbers-plus-style       = {line_numbers_plus_style}
    line-numbers-left-style       = {line_numbers_left_style}
    line-numbers-right-style      = {line_numbers_right_style}
    line-numbers-left-format      = {line_numbers_left_format}
    line-numbers-right-format     = {line_numbers_right_format}",
        line_numbers_minus_style = config.line_numbers_style_minusplus[Minus].to_painted_string(),
        line_numbers_zero_style = config.line_numbers_zero_style.to_painted_string(),
        line_numbers_plus_style = config.line_numbers_style_minusplus[Plus].to_painted_string(),
        line_numbers_left_style = config.line_numbers_style_leftright[Left].to_painted_string(),
        line_numbers_right_style = config.line_numbers_style_leftright[Right].to_painted_string(),
        line_numbers_left_format = format_option_value(&config.line_numbers_format[Left]),
        line_numbers_right_format = format_option_value(&config.line_numbers_format[Right]),
    )?;
    writeln!(
        writer,
        "    max-line-distance             = {max_line_distance}
    max-line-length               = {max_line_length}
    diff-stat-align-width         = {diff_stat_align_width}
    line-fill-method              = {line_fill_method}
    navigate                      = {navigate}
    navigate-regex                = {navigate_regex}
    pager                         = {pager}
    paging                        = {paging_mode}
    side-by-side                  = {side_by_side}
    syntax-theme                  = {syntax_theme}
    width                         = {width}
    tabs                          = {tab_width}
    word-diff-regex               = {tokenization_regex}",
        diff_stat_align_width = config.diff_stat_align_width,
        max_line_distance = config.max_line_distance,
        max_line_length = config.max_line_length,
        line_fill_method = match config.line_fill_method {
            BgFillMethod::TryAnsiSequence => "ansi",
            BgFillMethod::Spaces => "spaces",
        },
        navigate = config.navigate,
        navigate_regex = match &config.navigate_regex {
            None => "".to_string(),
            Some(s) => format_option_value(s),
        },
        pager = config.pager.clone().unwrap_or_else(|| "none".to_string()),
        paging_mode = match config.paging_mode {
            PagingMode::Always => "always",
            PagingMode::Never => "never",
            PagingMode::QuitIfOneScreen => "auto",
            PagingMode::Capture => unreachable!("capture can not be set"),
        },
        side_by_side = config.side_by_side,
        syntax_theme = config
            .syntax_theme
            .clone()
            .map(|t| t.name.unwrap_or_else(|| "none".to_string()))
            .unwrap_or_else(|| "none".to_string()),
        width = match config.decorations_width {
            cli::Width::Fixed(width) => width.to_string(),
            cli::Width::Variable => "variable".to_string(),
        },
        tab_width = config.tab_cfg.width(),
        tokenization_regex = format_option_value(config.tokenization_regex.to_string()),
    )?;
    Ok(())
}

fn write_additional_styles(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
    for (key, style) in [
        ("blame-code-style", config.blame_code_style),
        ("blame-separator-style", config.blame_separator_style),
        (
            "grep-context-line-style",
            Some(config.grep_context_line_style),
        ),
        (
            "grep-header-file-style",
            Some(config.classic_grep_header_file_style),
        ),
        ("grep-match-line-style", Some(config.grep_match_line_style)),
        ("grep-match-word-style", Some(config.grep_match_word_style)),
        (
            "hunk-header-file-style",
            Some(config.hunk_header_file_style),
        ),
        (
            "hunk-header-line-number-style",
            Some(config.hunk_header_line_number_style),
        ),
        ("inline-hint-style", Some(config.inline_hint_style)),
        (
            "merge-conflict-ours-diff-header-style",
            Some(config.merge_conflict_ours_diff_header_style),
        ),
        (
            "merge-conflict-theirs-diff-header-style",
            Some(config.merge_conflict_theirs_diff_header_style),
        ),
    ] {
        let value = style
            .map(|style| style.to_painted_string().to_string())
            .unwrap_or_else(|| "''".to_string());
        writeln!(writer, "    {key:29} = {value}")?;
    }
    Ok(())
}

fn write_decoration_styles(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
    let grep_header_style = match config.grep_output_type {
        Some(config::GrepType::Ripgrep) => config.ripgrep_header_style,
        _ => config.classic_grep_header_style,
    };
    for (key, style) in [
        (
            "commit-decoration-style",
            config.commit_style.decoration_style,
        ),
        ("file-decoration-style", config.file_style.decoration_style),
        (
            "hunk-header-decoration-style",
            config.hunk_header_style.decoration_style,
        ),
        (
            "grep-header-decoration-style",
            grep_header_style.decoration_style,
        ),
        (
            "merge-conflict-ours-diff-header-decoration-style",
            config
                .merge_conflict_ours_diff_header_style
                .decoration_style,
        ),
        (
            "merge-conflict-theirs-diff-header-decoration-style",
            config
                .merge_conflict_theirs_diff_header_style
                .decoration_style,
        ),
    ] {
        writeln!(writer, "    {key:29} = {}", painted_decoration_style(style))?;
    }
    Ok(())
}

fn write_additional_values(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
    for (key, value) in [
        ("blame-format", config.blame_format.as_str()),
        (
            "blame-timestamp-format",
            config.blame_timestamp_format.as_str(),
        ),
        (
            "blame-timestamp-output-format",
            config
                .blame_timestamp_output_format
                .as_deref()
                .unwrap_or(""),
        ),
        ("default-language", config.default_language.as_str()),
        ("diff-args", config.diff_args.as_str()),
        ("file-copied-label", config.file_copied_label.as_str()),
        (
            "grep-separator-symbol",
            config.grep_separator_symbol.as_str(),
        ),
        (
            "grep-output-type",
            match config.grep_output_type {
                Some(config::GrepType::Classic) => "classic",
                Some(config::GrepType::Ripgrep) => "ripgrep",
                None => "",
            },
        ),
        ("hunk-label", config.hunk_label.as_str()),
        (
            "hyperlinks-commit-link-format",
            config
                .hyperlinks_commit_link_format
                .as_deref()
                .unwrap_or(""),
        ),
        (
            "merge-conflict-begin-symbol",
            config.merge_conflict_begin_symbol.as_str(),
        ),
        (
            "merge-conflict-end-symbol",
            config.merge_conflict_end_symbol.as_str(),
        ),
    ] {
        writeln!(writer, "    {key:29} = {}", format_option_value(value))?;
    }
    for (key, value) in [
        ("wrap-left-symbol", config.wrap_config.left_symbol.as_str()),
        (
            "wrap-right-symbol",
            config.wrap_config.right_symbol.as_str(),
        ),
        (
            "wrap-right-prefix-symbol",
            config.wrap_config.right_prefix_symbol.as_str(),
        ),
    ] {
        writeln!(
            writer,
            "    {key:29} = {}",
            config.inline_hint_style.paint(format_option_value(value))
        )?;
    }
    writeln!(
        writer,
        "    {:29} = {}",
        "commit-regex",
        format_option_value(config.commit_regex.as_str())
    )?;
    writeln!(writer, "    {:29} = {}", "color-only", config.color_only)?;
    writeln!(
        writer,
        "    {:29} = {}",
        "relative-paths", config.relative_paths
    )?;
    writeln!(
        writer,
        "    {:29} = {}",
        "line-buffer-size", config.line_buffer_size
    )?;
    writeln!(
        writer,
        "    {:29} = {}",
        "max-syntax-highlighting-length", config.max_syntax_length
    )?;
    Ok(())
}

fn painted_decoration_style(decoration: style::DecorationStyle) -> String {
    use style::DecorationStyle::*;
    let (ansi_term_style, attributes) = match decoration {
        NoDecoration => return "none".to_string(),
        Box(style) => (style, "box"),
        Underline(style) => (style, "ul"),
        Overline(style) => (style, "ol"),
        UnderOverline(style) => (style, "ul ol"),
        BoxWithUnderline(style) => (style, "box ul"),
        BoxWithOverline(style) => (style, "box ol"),
        BoxWithUnderOverline(style) => (style, "box ul ol"),
    };
    let style = style::Style {
        ansi_term_style,
        ..style::Style::default()
    };
    style.paint(format!("{style} {attributes}")).to_string()
}

// Heuristics determining whether to quote string option values when printing values intended for
// git config.
fn format_option_value<S>(s: S) -> String
where
    S: AsRef<str>,
{
    let s = s.as_ref();
    if s.ends_with(' ')
        || s.starts_with(' ')
        || s.contains(&['\\', '{', '}', ':'][..])
        || s.is_empty()
    {
        format!("'{s}'")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::tests::integration_test_utils;

    use super::*;
    use crate::ansi;
    use std::io::{Cursor, Read, Seek};

    #[test]
    fn test_show_config() {
        let opt = integration_test_utils::make_options_from_args(&[]);
        let mut writer = Cursor::new(vec![0; 1024]);
        show_config(opt, &mut writer).unwrap();
        let mut s = String::new();
        writer.rewind().unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("    commit-style                  = raw\n"));
        assert!(s.contains(r"    word-diff-regex               = '\w+'"));
    }

    #[test]
    fn test_show_config_includes_all_config_keys() {
        let opt = integration_test_utils::make_options_from_args(&[]);
        let mut writer = Vec::new();
        show_config(opt, &mut writer).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let output = ansi::strip_ansi_codes(&output);
        let keys: Vec<_> = output
            .lines()
            .map(|line| line.split_once('=').unwrap().0.trim())
            .collect();
        let actual: BTreeSet<_> = keys.iter().copied().collect();
        assert_eq!(keys.len(), actual.len(), "duplicate config keys");
        let names = cli::Opt::get_argument_and_option_names();
        let expected: BTreeSet<_> = names
            .values()
            .map(String::as_str)
            .filter(|name| {
                !matches!(
                    *name,
                    "24-bit-color" | "parse-ansi" | "show-colors" | "show-themes"
                )
            })
            .collect();
        assert_eq!(actual, expected);
        assert!(
            keys.windows(2).all(|pair| pair[0] < pair[1]),
            "config keys are not sorted alphabetically"
        );
    }

    #[test]
    fn test_show_config_paints_decoration_and_additional_styles() {
        let opt = integration_test_utils::make_options_from_args(&[
            "--file-decoration-style=bold red blue box ul ol",
            "--grep-context-line-style=italic green",
            "--hunk-header-style=file line-number omit-code-fragment green",
        ]);
        let mut writer = Vec::new();
        show_config(opt, &mut writer).unwrap();
        let output = String::from_utf8(writer).unwrap();
        for (key, value) in [
            ("file-decoration-style", "bold red blue box ul ol"),
            ("grep-context-line-style", "italic green"),
            (
                "hunk-header-style",
                "file line-number omit-code-fragment green",
            ),
        ] {
            let line = output
                .lines()
                .find(|line| line.trim_start().starts_with(key))
                .unwrap();
            let printed_value = line.split_once('=').unwrap().1.trim();
            assert_eq!(ansi::strip_ansi_codes(printed_value), value);
            assert!(printed_value.contains('\x1b'), "{} is not painted", key);
        }
    }
}
