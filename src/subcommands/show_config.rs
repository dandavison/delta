use std::io::Write;

use crate::bat_utils::output::PagingMode;
use crate::cli;
use crate::config;

pub fn show_config(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
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
    whitespace-error-style        = {whitespace_error_style}",
        commit_style = config.commit_style.to_painted_string(),
        file_style = config.file_style.to_painted_string(),
        hunk_header_style = config.hunk_header_style.to_painted_string(),
        minus_emph_style = config.minus_emph_style.to_painted_string(),
        minus_empty_line_marker_style = config.minus_empty_line_marker_style.to_painted_string(),
        minus_non_emph_style = config.minus_non_emph_style.to_painted_string(),
        minus_style = config.minus_style.to_painted_string(),
        plus_emph_style = config.plus_emph_style.to_painted_string(),
        plus_empty_line_marker_style = config.plus_empty_line_marker_style.to_painted_string(),
        plus_non_emph_style = config.plus_non_emph_style.to_painted_string(),
        plus_style = config.plus_style.to_painted_string(),
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
    file-renamed-label            = {file_renamed_label}",
        true_color = config.true_color,
        file_added_label = format_option_value(&config.file_added_label),
        file_modified_label = format_option_value(&config.file_modified_label),
        file_removed_label = format_option_value(&config.file_removed_label),
        file_renamed_label = format_option_value(&config.file_renamed_label),
    )?;
    writeln!(
        writer,
        "    hyperlinks                    = {hyperlinks}",
        hyperlinks = config.hyperlinks
    )?;
    if config.hyperlinks {
        writeln!(
            writer,
            "    hyperlinks-file-link-format   = {hyperlinks_file_link_format}",
            hyperlinks_file_link_format = format_option_value(&config.hyperlinks_file_link_format),
        )?
    }
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
    if config.line_numbers {
        writeln!(
            writer,
            "    line-numbers-minus-style      = {line_numbers_minus_style}
    line-numbers-zero-style       = {line_numbers_zero_style}
    line-numbers-plus-style       = {line_numbers_plus_style}
    line-numbers-left-style       = {line_numbers_left_style}
    line-numbers-right-style      = {line_numbers_right_style}
    line-numbers-left-format      = {line_numbers_left_format}
    line-numbers-right-format     = {line_numbers_right_format}",
            line_numbers_minus_style = config.line_numbers_minus_style.to_painted_string(),
            line_numbers_zero_style = config.line_numbers_zero_style.to_painted_string(),
            line_numbers_plus_style = config.line_numbers_plus_style.to_painted_string(),
            line_numbers_left_style = config.line_numbers_left_style.to_painted_string(),
            line_numbers_right_style = config.line_numbers_right_style.to_painted_string(),
            line_numbers_left_format = format_option_value(&config.line_numbers_left_format),
            line_numbers_right_format = format_option_value(&config.line_numbers_right_format),
        )?
    }
    writeln!(
        writer,
        "    max-line-distance             = {max_line_distance}
    max-line-length               = {max_line_length}
    navigate                      = {navigate}
    navigate-regexp               = {navigate_regexp}
    pager                         = {pager}
    paging                        = {paging_mode}
    side-by-side                  = {side_by_side}
    syntax-theme                  = {syntax_theme}
    width                         = {width}
    tabs                          = {tab_width}
    word-diff-regex               = {tokenization_regex}",
        max_line_distance = config.max_line_distance,
        max_line_length = config.max_line_length,
        navigate = config.navigate,
        navigate_regexp = match &config.navigate_regexp {
            None => "".to_string(),
            Some(s) => s.to_string(),
        },
        pager = config.pager.clone().unwrap_or_else(|| "none".to_string()),
        paging_mode = match config.paging_mode {
            PagingMode::Always => "always",
            PagingMode::Never => "never",
            PagingMode::QuitIfOneScreen => "auto",
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
        tab_width = config.tab_width,
        tokenization_regex = format_option_value(&config.tokenization_regex.to_string()),
    )?;
    Ok(())
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
        format!("'{}'", s)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::integration_test_utils;

    use super::*;
    use crate::ansi;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    #[test]
    fn test_show_config() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![0; 1024]);
        show_config(&config, &mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("    commit-style                  = raw\n"));
        assert!(s.contains(r"    word-diff-regex               = '\w+'"));
    }
}
