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

use lazy_static::lazy_static;
use regex::Regex;

use super::draw;
use crate::config::Config;
use crate::delta::{self, State, StateMachine};
use crate::features;
use crate::paint::{BgShouldFill, Painter};
use crate::style::DecorationStyle;

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_hunk_header_line(&self) -> bool {
        self.line.starts_with("@@")
    }

    pub fn handle_hunk_header_line(&mut self) -> std::io::Result<bool> {
        if !self.test_hunk_header_line() {
            return Ok(false);
        }
        self.state = State::HunkHeader(self.line.clone(), self.raw_line.clone());
        Ok(true)
    }

    /// Emit the hunk header, with any requested decoration.
    pub fn emit_hunk_header_line(&mut self, line: &str, raw_line: &str) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.set_highlighter();
        self.painter.emit()?;

        let (code_fragment, line_numbers) = parse_hunk_header(line);
        if self.config.line_numbers {
            self.painter
                .line_numbers_data
                .as_mut()
                .unwrap()
                .initialize_hunk(&line_numbers, self.plus_file.to_string());
        }

        if self.config.hunk_header_style.is_raw {
            write_hunk_header_raw(&mut self.painter, line, raw_line, self.config)?;
        } else if self.config.hunk_header_style.is_omitted {
            writeln!(self.painter.writer)?;
        } else {
            // Add a blank line below the hunk-header-line for readability, unless
            // color_only mode is active.
            if !self.config.color_only {
                writeln!(self.painter.writer)?;
            }

            write_hunk_header(
                &code_fragment,
                &line_numbers,
                &mut self.painter,
                line,
                if self.plus_file == "/dev/null" {
                    &self.minus_file
                } else {
                    &self.plus_file
                },
                self.config,
            )?;
        };
        self.painter.set_highlighter();
        Ok(true)
    }
}

lazy_static! {
    static ref HUNK_HEADER_REGEX: Regex = Regex::new(r"@+ ([^@]+)@+(.*\s?)").unwrap();
}

// Parse unified diff hunk header format. See
// https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html
// https://www.artima.com/weblogs/viewpost.jsp?thread=164293
lazy_static! {
    static ref HUNK_HEADER_FILE_COORDINATE_REGEX: Regex = Regex::new(
        r"(?x)
[-+]
(\d+)            # 1. Hunk start line number
(?:              # Start optional hunk length section (non-capturing)
  ,              #   Literal comma
  (\d+)          #   2. Optional hunk length (defaults to 1)
)?"
    )
    .unwrap();
}

/// Given input like
/// "@@ -74,15 +74,14 @@ pub fn delta("
/// Return " pub fn delta(" and a vector of (line_number, hunk_length) tuples.
fn parse_hunk_header(line: &str) -> (String, Vec<(usize, usize)>) {
    let caps = HUNK_HEADER_REGEX.captures(line).unwrap();
    let file_coordinates = &caps[1];
    let line_numbers_and_hunk_lengths = HUNK_HEADER_FILE_COORDINATE_REGEX
        .captures_iter(file_coordinates)
        .map(|caps| {
            (
                caps[1].parse::<usize>().unwrap(),
                caps.get(2)
                    .map(|m| m.as_str())
                    // Per the specs linked above, if the hunk length is absent then it is 1.
                    .unwrap_or("1")
                    .parse::<usize>()
                    .unwrap(),
            )
        })
        .collect();
    let code_fragment = &caps[2];
    (code_fragment.to_string(), line_numbers_and_hunk_lengths)
}

fn write_hunk_header_raw(
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

fn write_hunk_header(
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
    if config.navigate {
        let _ = write!(
            &mut painter.output_buffer,
            "{} ",
            config.hunk_header_file_style.paint(&config.hunk_label)
        );
    }
    if !file_with_line_number.is_empty() {
        let _ = write!(&mut painter.output_buffer, "{}: ", file_with_line_number);
    }
    if !line.is_empty() {
        painter.syntax_highlight_and_paint_line(
            &line,
            config.hunk_header_style,
            delta::State::HunkHeader("".to_owned(), "".to_owned()),
            BgShouldFill::No,
        );
        painter.output_buffer.pop(); // trim newline
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::integration_test_utils;

    #[test]
    fn test_parse_hunk_header() {
        let parsed = parse_hunk_header("@@ -74,15 +75,14 @@ pub fn delta(\n");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 14),);
    }

    #[test]
    fn test_parse_hunk_header_with_omitted_hunk_lengths() {
        let parsed = parse_hunk_header("@@ -74 +75,2 @@ pub fn delta(\n");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 1),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 2),);
    }

    #[test]
    fn test_parse_hunk_header_added_file() {
        let parsed = parse_hunk_header("@@ -1,22 +0,0 @@");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (1, 22),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (0, 0),);
    }

    #[test]
    fn test_parse_hunk_header_deleted_file() {
        let parsed = parse_hunk_header("@@ -0,0 +1,3 @@");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (0, 0),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (1, 3),);
    }

    #[test]
    fn test_parse_hunk_header_merge() {
        let parsed = parse_hunk_header("@@@ -293,11 -358,15 +358,16 @@@ dependencies =");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " dependencies =");
        assert_eq!(line_numbers_and_hunk_lengths[0], (293, 11),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (358, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[2], (358, 16),);
    }
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

    #[test]
    fn test_get_painted_file_with_line_number_empty_navigate() {
        let cfg = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
            "--navigate",
        ]);

        let result = get_painted_file_with_line_number(&vec![(3, 4)], "δ some-file", &cfg);

        assert_eq!(result, "");
    }
}
