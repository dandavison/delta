use std::io::Write;

use console::strip_ansi_codes;
use syntect::highlighting::Color;
use syntect::parsing::SyntaxReference;

use crate::assets::HighlightingAssets;
use crate::output::{OutputType, PagingMode};
use crate::paint::{paint_text, Config};
use crate::parse_diff::get_file_extension_from_diff_line;

#[derive(Debug, PartialEq)]
pub enum State {
    Commit,    // In commit metadata section
    DiffMeta,  // In diff metadata section, between commit metadata and first hunk
    HunkMeta,  // In hunk metadata line
    HunkZero,  // In hunk; unchanged line
    HunkMinus, // In hunk; removed line
    HunkPlus,  // In hunk; added line
    Unknown,
}

// Possible transitions, with actions on entry:
//
//
// | from \ to | Commit      | DiffMeta    | HunkMeta    | HunkZero    | HunkMinus   | HunkPlus |
// |-----------+-------------+-------------+-------------+-------------+-------------+----------|
// | Commit    | emit        | emit        |             |             |             |          |
// | DiffMeta  |             | emit        | emit        |             |             |          |
// | HunkMeta  |             |             |             | emit        | push        | push     |
// | HunkZero  | emit        |             | emit        | emit        | push        | push     |
// | HunkMinus | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus  | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

struct Painter<'a> {
    minus_lines: Vec<String>,
    plus_lines: Vec<String>,
    output_buffer: String,
    writer: &'a mut Write,
    syntax: Option<&'a SyntaxReference>,
    config: &'a Config<'a>,
}

impl<'a> Painter<'a> {
    fn is_empty(&self) -> bool {
        return self.minus_lines.len() == 0 && self.plus_lines.len() == 0;
    }

    fn paint_and_emit_buffered_lines(&mut self) -> std::io::Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        self.paint_and_emit_text(
            self.minus_lines.join("\n"),
            Some(self.config.minus_color),
            self.config.highlight_removed,
        );
        self.minus_lines.clear();
        self.paint_and_emit_text(
            self.plus_lines.join("\n"),
            Some(self.config.plus_color),
            true,
        );
        self.plus_lines.clear();
        Ok(())
    }

    fn paint_and_emit_text(
        &mut self,
        text: String,
        background_color: Option<Color>,
        apply_syntax_highlighting: bool,
    ) -> std::io::Result<()> {
        paint_text(
            text,
            self.syntax.unwrap(),
            background_color,
            self.config,
            apply_syntax_highlighting,
            &mut self.output_buffer,
        );
        writeln!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.truncate(0);
        Ok(())
    }
}

pub fn delta(
    lines: impl Iterator<Item = String>,
    config: &Config,
    assets: &HighlightingAssets,
) -> std::io::Result<()> {
    let mut line: String;
    let mut output_type =
        OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(config.pager)).unwrap();
    let mut painter = Painter {
        minus_lines: Vec::new(),
        plus_lines: Vec::new(),
        output_buffer: String::new(),
        writer: output_type.handle().unwrap(),
        syntax: None,
        config: config,
    };

    let mut state = State::Unknown;

    for raw_line in lines {
        line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("diff --") {
            painter.paint_and_emit_buffered_lines()?;
            state = State::DiffMeta;
            painter.syntax = match get_file_extension_from_diff_line(&line) {
                // TODO: cache syntaxes?
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            painter.paint_and_emit_buffered_lines()?;
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
        } else if (state == State::HunkMeta
            || state == State::HunkZero
            || state == State::HunkMinus
            || state == State::HunkPlus)
            && painter.syntax.is_some()
        {
            match line.chars().next() {
                Some('-') => {
                    if state == State::HunkPlus {
                        painter.paint_and_emit_buffered_lines()?;
                    }
                    painter.minus_lines.push(line);
                    state = State::HunkMinus;
                }
                Some('+') => {
                    painter.plus_lines.push(line);
                    state = State::HunkPlus;
                }
                _ => {
                    painter.paint_and_emit_buffered_lines()?;
                    state = State::HunkZero;
                    painter.paint_and_emit_text(line, None, true)?;
                }
            };
            continue;
        }
        writeln!(painter.writer, "{}", raw_line)?;
    }
    painter.paint_and_emit_buffered_lines()?;
    Ok(())
}
