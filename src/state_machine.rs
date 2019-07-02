use std::io::Write;

use console::strip_ansi_codes;
use syntect::parsing::SyntaxReference;

use crate::assets::HighlightingAssets;
use crate::output::{OutputType, PagingMode};
use crate::paint::{paint_line, Config};
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

struct LineBuffer {
    minus_lines: Vec<String>,
    plus_lines: Vec<String>,
}

pub fn delta(
    lines: impl Iterator<Item = String>,
    paint_config: &Config,
    assets: &HighlightingAssets,
) -> std::io::Result<()> {
    let mut syntax: Option<&SyntaxReference> = None;
    let mut output_buffer = String::new();
    let mut line: String;
    let mut output_type =
        OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(paint_config.pager)).unwrap();
    let mut line_buffer = LineBuffer {
        minus_lines: Vec::new(),
        plus_lines: Vec::new(),
    };
    let writer = output_type.handle().unwrap();
    let mut state = State::Unknown;

    for raw_line in lines {
        line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("diff --") {
            if (state == State::HunkMinus || state == State::HunkPlus) && syntax.is_some() {
                flush(
                    &mut line_buffer.minus_lines,
                    &mut line_buffer.plus_lines,
                    syntax.unwrap(),
                    &paint_config,
                    &mut output_buffer,
                    writer,
                )?;
                line_buffer.minus_lines.clear();
                line_buffer.plus_lines.clear();
            };
            state = State::DiffMeta;
            syntax = match get_file_extension_from_diff_line(&line) {
                // TODO: cache syntaxes?
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            if (state == State::HunkMinus || state == State::HunkPlus) && syntax.is_some() {
                flush(
                    &mut line_buffer.minus_lines,
                    &mut line_buffer.plus_lines,
                    syntax.unwrap(),
                    &paint_config,
                    &mut output_buffer,
                    writer,
                )?;
                line_buffer.minus_lines.clear();
                line_buffer.plus_lines.clear();
            };
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
        } else if (state == State::HunkMeta
            || state == State::HunkZero
            || state == State::HunkMinus
            || state == State::HunkPlus)
            && syntax.is_some()
        {
            match line.chars().next() {
                None | Some(' ') => {
                    flush(
                        &mut line_buffer.minus_lines,
                        &mut line_buffer.plus_lines,
                        syntax.unwrap(),
                        &paint_config,
                        &mut output_buffer,
                        writer,
                    )?;
                    line_buffer.minus_lines.clear();
                    line_buffer.plus_lines.clear();
                    state = State::HunkZero;
                    emit(
                        line,
                        &state,
                        syntax.unwrap(),
                        &paint_config,
                        &mut output_buffer,
                        writer,
                    )?;
                }
                Some('-') => {
                    if state == State::HunkPlus {
                        flush(
                            &mut line_buffer.minus_lines,
                            &mut line_buffer.plus_lines,
                            syntax.unwrap(),
                            &paint_config,
                            &mut output_buffer,
                            writer,
                        )?;
                        line_buffer.minus_lines.clear();
                        line_buffer.plus_lines.clear();
                    }
                    line_buffer.minus_lines.push(line);
                    state = State::HunkMinus;
                }
                Some('+') => {
                    line_buffer.plus_lines.push(line);
                    state = State::HunkPlus;
                }
                _ => panic!("Error parsing diff at line: '{}'", line),
            };
            continue;
        }
        writeln!(writer, "{}", raw_line)?;
    }
    if (state == State::HunkMinus || state == State::HunkPlus) && syntax.is_some() {
        flush(
            &mut line_buffer.minus_lines,
            &mut line_buffer.plus_lines,
            syntax.unwrap(),
            &paint_config,
            &mut output_buffer,
            writer,
        )?;
        line_buffer.minus_lines.clear();
        line_buffer.plus_lines.clear();
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

fn flush(
    minus_lines: &mut Vec<String>,
    plus_lines: &mut Vec<String>,
    syntax: &SyntaxReference,
    config: &Config,
    output_buffer: &mut String,
    writer: &mut Write,
) -> std::io::Result<()> {
    for line in minus_lines {
        emit(
            line.to_string(),
            &State::HunkMinus,
            syntax,
            config,
            output_buffer,
            writer,
        )?;
    }
    for line in plus_lines {
        emit(
            line.to_string(),
            &State::HunkPlus,
            syntax,
            config,
            output_buffer,
            writer,
        )?;
    }
    Ok(())
}
