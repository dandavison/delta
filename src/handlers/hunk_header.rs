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
use std::convert::TryInto;
use std::fmt::Write as FmtWrite;

use super::draw;
use crate::config::{
    Config, HunkHeaderIncludeCodeFragment, HunkHeaderIncludeFilePath, HunkHeaderIncludeLineNumber,
};
use crate::delta::{self, DiffType, InMergeConflict, MergeParents, State, StateMachine};
use crate::paint::{self, BgShouldFill, Painter, StyleSectionSpecifier};
use crate::style::{DecorationStyle, Style};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ParsedHunkHeader {
    code_fragment: String,
    line_numbers_and_hunk_lengths: Vec<(usize, usize)>,
}

pub enum HunkHeaderIncludeHunkLabel {
    Yes,
    No,
}

// The output of `diff -u file1 file2` does not contain a header before the
// `--- old.lua` / `+++ new.lua` block, and the hunk can of course contain lines
// starting with '-- '. To avoid interpreting '--- lua comment' as a new header,
// count the minus lines in a hunk (provided by the '@@ -1,4 +1,3 @@' header).
// `diff` itself can not generate two diffs in this ambiguous format, so a second header
// could just be ignored. But concatenated diffs exist and are accepted by `patch`.
#[derive(Debug)]
pub struct AmbiguousDiffMinusCounter(isize);

impl AmbiguousDiffMinusCounter {
    // The internal isize representation avoids calling `if let Some(..)` on every line. For
    // nearly all input the counter is not needed, in this case it is decremented but ignored.
    // [min, COUNTER_RELEVANT_IF_GT]   unambiguous diff
    // (COUNTER_RELEVANT_IF_GT, 0]     handle next '--- ' like a header, and set counter in next @@ block
    // [1, max]                        counting minus lines in ambiguous header
    const COUNTER_RELEVANT_IF_GREATER_THAN: isize = -4096; // -1 works too, but be defensive
    const EXPECT_DIFF_3DASH_HEADER: isize = 0;

    pub fn not_needed() -> Self {
        Self(Self::COUNTER_RELEVANT_IF_GREATER_THAN)
    }
    pub fn prepare_to_count() -> Self {
        Self(Self::EXPECT_DIFF_3DASH_HEADER)
    }
    pub fn three_dashes_expected(&self) -> bool {
        if self.0 > Self::COUNTER_RELEVANT_IF_GREATER_THAN {
            self.0 <= Self::EXPECT_DIFF_3DASH_HEADER
        } else {
            true
        }
    }
    pub fn count_line(&mut self) {
        self.0 -= 1;
    }
    fn count_from(lines: usize) -> Self {
        Self(
            lines
                .try_into()
                .unwrap_or(Self::COUNTER_RELEVANT_IF_GREATER_THAN),
        )
    }
    #[allow(clippy::needless_bool)]
    fn must_count(&mut self) -> bool {
        let relevant = self.0 > Self::COUNTER_RELEVANT_IF_GREATER_THAN;
        if relevant {
            true
        } else {
            #[cfg(target_pointer_width = "32")]
            {
                self.0 = Self::COUNTER_RELEVANT_IF_GREATER_THAN;
            }
            false
        }
    }
}

impl StateMachine<'_> {
    #[inline]
    fn test_hunk_header_line(&self) -> bool {
        self.line.starts_with("@@") &&
        // A hunk header can occur within a merge conflict region, but we don't attempt to handle
        // that. See #822.
        !matches!(self.state, State::MergeConflict(_, _))
    }

    pub fn handle_hunk_header_line(&mut self) -> std::io::Result<bool> {
        use DiffType::*;
        use State::*;
        if !self.test_hunk_header_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        if let Some(parsed_hunk_header) = parse_hunk_header(&self.line) {
            let diff_type = match &self.state {
                DiffHeader(Combined(MergeParents::Unknown, InMergeConflict::No)) => {
                    // https://git-scm.com/docs/git-diff#_combined_diff_format
                    let n_parents = self.line.chars().take_while(|c| c == &'@').count() - 1;
                    Combined(MergeParents::Number(n_parents), InMergeConflict::No)
                }
                DiffHeader(diff_type)
                | HunkMinus(diff_type, _)
                | HunkZero(diff_type, _)
                | HunkPlus(diff_type, _) => diff_type.clone(),
                _ => Unified,
            };

            if self.minus_line_counter.must_count() {
                if let &[(_, minus_lines), (_, _plus_lines), ..] =
                    parsed_hunk_header.line_numbers_and_hunk_lengths.as_slice()
                {
                    self.minus_line_counter = AmbiguousDiffMinusCounter::count_from(minus_lines);
                }
            }

            self.state = HunkHeader(
                diff_type,
                parsed_hunk_header,
                self.line.clone(),
                self.raw_line.clone(),
            );
            handled_line = true;
        }
        Ok(handled_line)
    }

    /// Emit the hunk header, with any requested decoration.
    pub fn emit_hunk_header_line(
        &mut self,
        parsed_hunk_header: &ParsedHunkHeader,
        line: &str,
        raw_line: &str,
    ) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.set_highlighter();
        self.painter.emit()?;

        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parsed_hunk_header;

        if self.config.line_numbers {
            self.painter
                .line_numbers_data
                .as_mut()
                .unwrap()
                .initialize_hunk(line_numbers_and_hunk_lengths, self.plus_file.to_string());
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

            write_line_of_code_with_optional_path_and_line_number(
                code_fragment,
                line_numbers_and_hunk_lengths,
                None,
                &mut self.painter,
                line,
                if self.plus_file == "/dev/null" {
                    &self.minus_file
                } else {
                    &self.plus_file
                },
                self.config.hunk_header_style.decoration_style,
                &self.config.hunk_header_file_style,
                &self.config.hunk_header_line_number_style,
                &self.config.hunk_header_style_include_file_path,
                &self.config.hunk_header_style_include_line_number,
                &HunkHeaderIncludeHunkLabel::Yes,
                &self.config.hunk_header_style_include_code_fragment,
                ":",
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
fn parse_hunk_header(line: &str) -> Option<ParsedHunkHeader> {
    if let Some(caps) = HUNK_HEADER_REGEX.captures(line) {
        let file_coordinates = &caps[1];
        let line_numbers_and_hunk_lengths: Vec<(usize, usize)> = HUNK_HEADER_FILE_COORDINATE_REGEX
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
        if line_numbers_and_hunk_lengths.is_empty() {
            None
        } else {
            let code_fragment = caps[2].to_string();
            Some(ParsedHunkHeader {
                code_fragment,
                line_numbers_and_hunk_lengths,
            })
        }
    } else {
        None
    }
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
        "",
        &config.decorations_width,
        config.hunk_header_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_line_of_code_with_optional_path_and_line_number(
    code_fragment: &str,
    line_numbers_and_hunk_lengths: &[(usize, usize)],
    style_sections: Option<StyleSectionSpecifier>,
    painter: &mut Painter,
    line: &str,
    plus_file: &str,
    decoration_style: DecorationStyle,
    file_style: &Style,
    line_number_style: &Style,
    include_file_path: &HunkHeaderIncludeFilePath,
    include_line_number: &HunkHeaderIncludeLineNumber,
    include_hunk_label: &HunkHeaderIncludeHunkLabel,
    include_code_fragment: &HunkHeaderIncludeCodeFragment,
    file_path_separator: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, _, decoration_ansi_term_style) = draw::get_draw_function(decoration_style);
    let line = if config.color_only {
        line.to_string()
    } else if matches!(include_code_fragment, HunkHeaderIncludeCodeFragment::Yes)
        && !code_fragment.is_empty()
    {
        format!("{code_fragment} ")
    } else {
        "".to_string()
    };

    let plus_line_number = line_numbers_and_hunk_lengths[line_numbers_and_hunk_lengths.len() - 1].0;
    let file_with_line_number = paint_file_path_with_line_number(
        Some(plus_line_number),
        plus_file,
        file_style,
        line_number_style,
        include_file_path,
        include_line_number,
        file_path_separator,
        config,
    );

    if !line.is_empty() || !file_with_line_number.is_empty() {
        write_to_output_buffer(
            &file_with_line_number,
            file_path_separator,
            line,
            style_sections,
            include_hunk_label,
            painter,
            config,
        );
        draw_fn(
            painter.writer,
            &painter.output_buffer,
            &painter.output_buffer,
            "",
            &config.decorations_width,
            config.null_style,
            decoration_ansi_term_style,
        )?;
        painter.output_buffer.clear();
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn paint_file_path_with_line_number(
    line_number: Option<usize>,
    plus_file: &str,
    file_style: &Style,
    line_number_style: &Style,
    include_file_path: &HunkHeaderIncludeFilePath,
    include_line_number: &HunkHeaderIncludeLineNumber,
    separator: &str,
    config: &Config,
) -> String {
    let file_style = match include_file_path {
        HunkHeaderIncludeFilePath::Yes => Some(*file_style),
        HunkHeaderIncludeFilePath::No => None,
    };
    let line_number_style = if matches!(include_line_number, HunkHeaderIncludeLineNumber::Yes)
        && line_number.is_some()
        && !config.hunk_header_style.is_raw
        && !config.color_only
    {
        Some(*line_number_style)
    } else {
        None
    };

    paint::paint_file_path_with_line_number(
        line_number,
        plus_file,
        false,
        separator,
        false,
        file_style,
        line_number_style,
        config,
    )
}

fn write_to_output_buffer(
    file_with_line_number: &str,
    file_path_separator: &str,
    line: String,
    style_sections: Option<StyleSectionSpecifier>,
    include_hunk_label: &HunkHeaderIncludeHunkLabel,
    painter: &mut Painter,
    config: &Config,
) {
    if matches!(include_hunk_label, HunkHeaderIncludeHunkLabel::Yes)
        && !config.hunk_label.is_empty()
    {
        let _ = write!(
            &mut painter.output_buffer,
            "{} ",
            config.hunk_header_file_style.paint(&config.hunk_label)
        );
    }
    if !file_with_line_number.is_empty() {
        // The code fragment in "line" adds whitespace, but if only a line number is printed
        // then the trailing space must be added.
        let space = if line.is_empty() { " " } else { "" };
        let _ = write!(
            &mut painter.output_buffer,
            "{file_with_line_number}{file_path_separator}{space}",
        );
    }
    if !line.is_empty() {
        painter.syntax_highlight_and_paint_line(
            &line,
            style_sections.unwrap_or(StyleSectionSpecifier::Style(config.hunk_header_style)),
            delta::State::HunkHeader(
                DiffType::Unified,
                ParsedHunkHeader::default(),
                "".to_owned(),
                "".to_owned(),
            ),
            BgShouldFill::No,
        );
        painter.output_buffer.pop(); // trim newline
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::ansi::strip_ansi_codes;
    use crate::tests::integration_test_utils;

    #[test]
    fn test_parse_hunk_header() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@ -74,15 +75,14 @@ pub fn delta(\n").unwrap();
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 14),);
    }

    #[test]
    fn test_parse_hunk_header_with_omitted_hunk_lengths() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@ -74 +75,2 @@ pub fn delta(\n").unwrap();
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 1),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 2),);
    }

    #[test]
    fn test_parse_hunk_header_with_no_hunk_lengths() {
        let result = parse_hunk_header("@@  @@\n");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hunk_header_added_file() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@ -1,22 +0,0 @@").unwrap();
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (1, 22),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (0, 0),);
    }

    #[test]
    fn test_parse_hunk_header_deleted_file() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@ -0,0 +1,3 @@").unwrap();
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (0, 0),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (1, 3),);
    }

    #[test]
    fn test_parse_hunk_header_merge() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@@ -293,11 -358,15 +358,16 @@@ dependencies =").unwrap();
        assert_eq!(code_fragment, " dependencies =");
        assert_eq!(line_numbers_and_hunk_lengths[0], (293, 11),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (358, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[2], (358, 16),);
    }

    #[test]
    fn test_parse_hunk_header_cthulhu() {
        let ParsedHunkHeader {
            code_fragment,
            line_numbers_and_hunk_lengths,
        } = parse_hunk_header("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -444,17 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 -446,6 +444,17 @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ int snd_soc_jack_add_gpios(struct snd_s").unwrap();
        assert_eq!(code_fragment, " int snd_soc_jack_add_gpios(struct snd_s");
        assert_eq!(line_numbers_and_hunk_lengths[0], (446, 6),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (446, 6),);
        assert_eq!(line_numbers_and_hunk_lengths[2], (446, 6),);
        assert_eq!(line_numbers_and_hunk_lengths[65], (446, 6),);
    }

    #[test]
    fn test_paint_file_path_with_line_number_default() {
        // hunk-header-style (by default) includes 'line-number' but not 'file'.
        // This test confirms that `paint_file_path_with_line_number` returns a painted line number.
        let config = integration_test_utils::make_config_from_args(&[]);

        let result = paint_file_path_with_line_number(
            Some(3),
            "some-file",
            &config.hunk_header_style,
            &config.hunk_header_line_number_style,
            &config.hunk_header_style_include_file_path,
            &config.hunk_header_style_include_line_number,
            ":",
            &config,
        );

        assert_eq!(result, "\u{1b}[34m3\u{1b}[0m");
    }

    #[test]
    fn test_paint_file_path_with_line_number_hyperlinks() {
        use std::{iter::FromIterator, path::PathBuf};

        use crate::utils;

        // hunk-header-style (by default) includes 'line-number' but not 'file'.
        // Normally, `paint_file_path_with_line_number` would return a painted line number.
        // But in this test hyperlinks are activated, and the test ensures that delta.__workdir__ is
        // present in git_config_entries.
        // This test confirms that, under those circumstances, `paint_file_path_with_line_number`
        // returns a hyperlinked file path with line number.

        let config = integration_test_utils::make_config_from_args(&["--features", "hyperlinks"]);
        let relative_path = PathBuf::from_iter(["some-dir", "some-file"]);

        let result = paint_file_path_with_line_number(
            Some(3),
            &relative_path.to_string_lossy(),
            &config.hunk_header_style,
            &config.hunk_header_line_number_style,
            &config.hunk_header_style_include_file_path,
            &config.hunk_header_style_include_line_number,
            ":",
            &config,
        );

        assert_eq!(
            result,
            format!(
                "\u{1b}]8;;file://{}\u{1b}\\\u{1b}[34m3\u{1b}[0m\u{1b}]8;;\u{1b}\\",
                utils::path::fake_delta_cwd_for_tests()
                    .join(relative_path)
                    .to_string_lossy()
            )
        );
    }

    #[test]
    fn test_paint_file_path_with_line_number_empty() {
        // hunk-header-style includes neither 'file' nor 'line-number'.
        // This causes `paint_file_path_with_line_number` to return empty string.
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
        ]);

        let result = paint_file_path_with_line_number(
            Some(3),
            "some-file",
            &config.hunk_header_style,
            &config.hunk_header_line_number_style,
            &config.hunk_header_style_include_file_path,
            &config.hunk_header_style_include_line_number,
            ":",
            &config,
        );

        // result is
        // "\u{1b}[1msome-file\u{1b}[0m:\u{1b}[34m3\u{1b}[0m"
        assert_eq!(result, "");
    }

    #[test]
    fn test_paint_file_path_with_line_number_empty_hyperlinks() {
        // hunk-header-style includes neither 'file' nor 'line-number'.
        // This causes `paint_file_path_with_line_number` to return empty string.
        // This test confirms that this remains true even when we are requesting hyperlinks.

        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
            "--features",
            "hyperlinks",
        ]);

        let result = paint_file_path_with_line_number(
            Some(3),
            "some-file",
            &config.hunk_header_style,
            &config.hunk_header_line_number_style,
            &config.hunk_header_style_include_file_path,
            &config.hunk_header_style_include_line_number,
            ":",
            &config,
        );

        assert_eq!(result, "");
    }

    #[test]
    fn test_paint_file_path_with_line_number_empty_navigate() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax bold",
            "--hunk-header-decoration-style",
            "omit",
            "--navigate",
        ]);

        let result = paint_file_path_with_line_number(
            Some(3),
            "δ some-file",
            &config.hunk_header_style,
            &config.hunk_header_line_number_style,
            &config.hunk_header_style_include_file_path,
            &config.hunk_header_style_include_line_number,
            ":",
            &config,
        );

        // result is
        // "\u{1b}[1mδ some-file\u{1b}[0m:\u{1b}[34m3\u{1b}[0m"
        assert_eq!(result, "");
    }

    #[test]
    fn test_not_a_hunk_header_is_handled_gracefully() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output =
            integration_test_utils::run_delta(GIT_LOG_OUTPUT_WITH_NOT_A_HUNK_HEADER, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("@@@2021-12-05"));
    }

    const GIT_LOG_OUTPUT_WITH_NOT_A_HUNK_HEADER: &str = "\
@@@2021-12-05

src/config.rs                  |   2 +-
src/delta.rs                   |   3 ++-
src/handlers/hunk.rs           |  12 ++++++------
src/handlers/hunk_header.rs    | 119 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++------------------------------------------
src/handlers/merge_conflict.rs |   2 +-
src/handlers/submodule.rs      |   4 ++--
src/paint.rs                   |   2 +-
7 files changed, 90 insertions(+), 54 deletions(-)
";
}
