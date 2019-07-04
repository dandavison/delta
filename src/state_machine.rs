use std::io::Write;

use console::strip_ansi_codes;
use syntect::parsing::SyntaxReference;

use crate::assets::HighlightingAssets;
use crate::output::{OutputType, PagingMode};
use crate::paint::{paint_line, paint_text, Config};
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

struct LineBuffer<'a> {
    minus_lines: Vec<String>,
    plus_lines: Vec<String>,
    output_buffer: String,
    writer: &'a mut Write,
    syntax: Option<&'a SyntaxReference>,
    config: &'a Config<'a>,
}

impl<'a> LineBuffer<'a> {
    fn is_empty(&self) -> bool {
        return self.minus_lines.len() == 0 && self.plus_lines.len() == 0;
    }

    fn flush(&mut self) -> std::io::Result<()> {
        paint_text(
            self.minus_lines.join("\n"),
            self.syntax.unwrap(),
            Some(self.config.minus_color),
            self.config,
            self.config.highlight_removed,
            &mut self.output_buffer,
        );
        writeln!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.truncate(0);
        self.minus_lines.clear();

        paint_text(
            self.plus_lines.join("\n"),
            self.syntax.unwrap(),
            Some(self.config.plus_color),
            self.config,
            true,
            &mut self.output_buffer,
        );
        writeln!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.truncate(0);
        self.plus_lines.clear();

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
    let mut line_buffer = LineBuffer {
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
            if (state == State::HunkMinus || state == State::HunkPlus)
                && line_buffer.syntax.is_some()
                && !line_buffer.is_empty()
            {
                line_buffer.flush()?;
            };
            state = State::DiffMeta;
            line_buffer.syntax = match get_file_extension_from_diff_line(&line) {
                // TODO: cache syntaxes?
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            if (state == State::HunkMinus || state == State::HunkPlus)
                && line_buffer.syntax.is_some()
                && !line_buffer.is_empty()
            {
                line_buffer.flush()?;
            };
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
        } else if (state == State::HunkMeta
            || state == State::HunkZero
            || state == State::HunkMinus
            || state == State::HunkPlus)
            && line_buffer.syntax.is_some()
        {
            match line.chars().next() {
                Some('-') => {
                    if state == State::HunkPlus {
                        line_buffer.flush()?;
                    }
                    line_buffer.minus_lines.push(line);
                    state = State::HunkMinus;
                }
                Some('+') => {
                    line_buffer.plus_lines.push(line);
                    state = State::HunkPlus;
                }
                _ => {
                    line_buffer.flush()?;
                    state = State::HunkZero;
                    emit(
                        line,
                        &state,
                        line_buffer.syntax.unwrap(),
                        &line_buffer.config,
                        &mut line_buffer.output_buffer,
                        line_buffer.writer,
                    )?;
                }
            };
            continue;
        }
        writeln!(line_buffer.writer, "{}", raw_line)?;
    }
    if (state == State::HunkMinus || state == State::HunkPlus)
        && line_buffer.syntax.is_some()
        && !line_buffer.is_empty()
    {
        line_buffer.flush()?;
    };
    line_buffer.minus_lines.clear();
    line_buffer.plus_lines.clear();
    Ok(())
}

fn emit(
    line: String,
    state: &State,
    syntax: &SyntaxReference,
    config: &Config,
    output_buffer: &mut String,
    writer: &mut Write,
) -> std::io::Result<()> {
    paint_line(line, state, syntax, config, output_buffer);
    writeln!(writer, "{}", output_buffer)?;
    output_buffer.truncate(0);
    Ok(())
}
