// A module for constructing and writing the hunk header.
//
// The structure of the hunk header output by delta is
// ```
// (file):(line-number): (code-fragment)
// ```
//
// The code fragment and line number derive from a line of git/diff output that looks like
// ```
// @@ -119,12 +119,7 @@ fn write_to_output_buffer(
// ```
//
// Whether or not file and line-number are included is controlled by the presence of the special
// style attributes 'file' and 'line-number' in the hunk-header-style string. For example, delta
// might output the above hunk header as
// ```
// ───────────────────────────────────────────────────┐
// src/hunk_header.rs:119: fn write_to_output_buffer( │
// ───────────────────────────────────────────────────┘
// ```

use std::fmt::Write as FmtWrite;

use unicode_segmentation::UnicodeSegmentation;

use crate::config::Config;
use crate::delta;
use crate::draw;
use crate::features;
use crate::paint::Painter;
use crate::style::DecorationStyle;

pub fn write_hunk_header_raw(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(config.hunk_header_style.decoration_style);
    if config.hunk_header_style.decoration_style != DecorationStyle::NoDecoration {
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        &config.decorations_width,
        config.hunk_header_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

pub fn write_hunk_header(
    code_fragment: &str,
    line_numbers: &[(usize, usize)],
    painter: &mut Painter,
    line: &str,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, _, decoration_ansi_term_style) =
        draw::get_draw_function(config.hunk_header_style.decoration_style);
    let line = if config.color_only {
        format!(" {}", &line)
    } else if !code_fragment.is_empty() {
        format!("{} ", code_fragment)
    } else {
        "".to_string()
    };

    let file_with_line_number = get_painted_file_with_line_number(line_numbers, plus_file, config);

    if !line.is_empty() || !file_with_line_number.is_empty() {
        write_to_output_buffer(&file_with_line_number, line, painter, config);
        draw_fn(
            painter.writer,
            &painter.output_buffer,
            &painter.output_buffer,
            &config.decorations_width,
            config.null_style,
            decoration_ansi_term_style,
        )?;
        painter.output_buffer.clear();
    }

    Ok(())
}

fn get_painted_file_with_line_number(
    line_numbers: &[(usize, usize)],
    plus_file: &str,
    config: &Config,
) -> String {
    let mut file_with_line_number = Vec::new();
    let modified_label;
    if config.navigate {
        modified_label = format!("{} ", config.file_modified_label);
        file_with_line_number.push(config.hunk_header_file_style.paint(&modified_label));
    }
    let plus_line_number = line_numbers[line_numbers.len() - 1].0;
    if config.hunk_header_style_include_file_path {
        file_with_line_number.push(config.hunk_header_file_style.paint(plus_file))
    };
    if config.hunk_header_style_include_line_number
        && !config.hunk_header_style.is_raw
        && !config.color_only
    {
        if !file_with_line_number.is_empty() {
            file_with_line_number.push(ansi_term::ANSIString::from(":"));
        }
        file_with_line_number.push(
            config
                .hunk_header_line_number_style
                .paint(format!("{}", plus_line_number)),
        )
    }
    let file_with_line_number = ansi_term::ANSIStrings(&file_with_line_number).to_string();
    if config.hyperlinks && !file_with_line_number.is_empty() {
        features::hyperlinks::format_osc8_file_hyperlink(
            plus_file,
            Some(plus_line_number),
            &file_with_line_number,
            config,
        )
        .into()
    } else {
        file_with_line_number
    }
}

fn write_to_output_buffer(
    file_with_line_number: &str,
    line: String,
    painter: &mut Painter,
    config: &Config,
) {
    if !file_with_line_number.is_empty() {
        let _ = write!(&mut painter.output_buffer, "{}: ", file_with_line_number);
    }
    if !line.is_empty() {
        let lines = vec![(
            painter.expand_tabs(line.graphemes(true)),
            delta::State::HunkHeader,
        )];
        let syntax_style_sections = Painter::get_syntax_style_sections_for_lines(
            &lines,
            &delta::State::HunkHeader,
            &mut painter.highlighter,
            &painter.config,
        );
        Painter::paint_lines(
            syntax_style_sections,
            vec![vec![(config.hunk_header_style, &lines[0].0)]], // TODO: compute style from state
            [delta::State::HunkHeader].iter(),
            &mut painter.output_buffer,
            config,
            &mut None,
            None,
            None,
            Some(false),
        );
        painter.output_buffer.pop(); // trim newline
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::integration_test_utils;

    #[test]
    fn test_get_painted_file_with_line_number_default() {
        let cfg = integration_test_utils::make_config_from_args(&[]);

        let result = get_painted_file_with_line_number(&vec![(3, 4)], "some-file", &cfg);

        assert_eq!(result, "\u{1b}[34m3\u{1b}[0m");
    }

    #[test]
    fn test_get_painted_file_with_line_number_hyperlinks() {
        let cfg = integration_test_utils::make_config_from_args(&["--features", "hyperlinks"]);

        let result = get_painted_file_with_line_number(&vec![(3, 4)], "some-file", &cfg);

        assert_eq!(result, "some-file");
    }

    #[test]
    fn test_get_painted_file_with_line_number_empty() {
        let cfg = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
        ]);

        let result = get_painted_file_with_line_number(&vec![(3, 4)], "some-file", &cfg);

        assert_eq!(result, "");
    }

    #[test]
    fn test_get_painted_file_with_line_number_empty_hyperlinks() {
        let cfg = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
            "--features",
            "hyperlinks",
        ]);

        let result = get_painted_file_with_line_number(&vec![(3, 4)], "some-file", &cfg);

        assert_eq!(result, "");
    }
}
