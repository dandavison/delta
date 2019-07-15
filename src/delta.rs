use std::io::Write;

use ansi_term::Colour::{Blue, Yellow};
use console::strip_ansi_codes;
use syntect::easy::HighlightLines;

use crate::bat::assets::HighlightingAssets;
use crate::cli;
use crate::config::Config;
use crate::draw;
use crate::paint::Painter;
use crate::parse;
use crate::style;

#[derive(Debug, PartialEq)]
pub enum State {
    CommitMeta, // In commit metadata section
    FileMeta,   // In diff metadata section, between commit metadata and first hunk
    HunkMeta,   // In hunk metadata line
    HunkZero,   // In hunk; unchanged line
    HunkMinus,  // In hunk; removed line
    HunkPlus,   // In hunk; added line
    Unknown,
}

impl State {
    fn is_in_hunk(&self) -> bool {
        match *self {
            State::HunkMeta | State::HunkZero | State::HunkMinus | State::HunkPlus => true,
            _ => false,
        }
    }
}

// Possible transitions, with actions on entry:
//
//
// | from \ to  | CommitMeta  | FileMeta    | HunkMeta    | HunkZero    | HunkMinus   | HunkPlus |
// |------------+-------------+-------------+-------------+-------------+-------------+----------|
// | CommitMeta | emit        | emit        |             |             |             |          |
// | FileMeta   |             | emit        | emit        |             |             |          |
// | HunkMeta   |             |             |             | emit        | push        | push     |
// | HunkZero   | emit        | emit        | emit        | emit        | push        | push     |
// | HunkMinus  | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus   | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

pub fn delta(
    lines: impl Iterator<Item = String>,
    config: &Config,
    assets: &HighlightingAssets,
    writer: &mut Write,
) -> std::io::Result<()> {
    // TODO: Painter::new(config)
    let mut painter = Painter {
        minus_lines: Vec::new(),
        plus_lines: Vec::new(),
        minus_line_style_sections: Vec::new(),
        plus_line_style_sections: Vec::new(),
        output_buffer: String::new(),
        writer: writer,
        syntax: None,
        highlighter: HighlightLines::new(
            assets.syntax_set.find_syntax_by_extension("txt").unwrap(),
            config.theme,
        ),
        config: config,
    };

    let mut state = State::Unknown;

    for raw_line in lines {
        let line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("commit") {
            painter.paint_buffered_lines();
            state = State::CommitMeta;
            if config.opt.commit_style != cli::SectionStyle::Plain {
                painter.emit()?;
                write_commit_meta_header_line(&mut painter, &raw_line, config)?;
                continue;
            }
        } else if line.starts_with("diff --") {
            painter.paint_buffered_lines();
            state = State::FileMeta;
            painter.syntax = match parse::get_file_extension_from_diff_line(&line) {
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
            if config.opt.file_style != cli::SectionStyle::Plain {
                painter.emit()?;
                write_file_meta_header_line(&mut painter, &raw_line, config)?;
                continue;
            }
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
            if painter.syntax.is_some() {
                painter.reset_highlighter();
            }
            if config.opt.hunk_style != cli::SectionStyle::Plain {
                painter.emit()?;
                write_hunk_meta_line(&mut painter, &line, config)?;
                continue;
            }
        } else if state.is_in_hunk() && painter.syntax.is_some() {
            state = paint_hunk_line(state, &mut painter, &line, config);
            painter.emit()?;
            continue;
        }
        if state == State::FileMeta && config.opt.file_style != cli::SectionStyle::Plain {
            // The file metadata section is 4 lines. Skip them under non-plain file-styles.
            continue;
        } else {
            painter.emit()?;
            writeln!(painter.writer, "{}", raw_line)?;
        }
    }

    painter.paint_buffered_lines();
    painter.emit()?;
    Ok(())
}

fn write_commit_meta_header_line(
    painter: &mut Painter,
    line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let draw_fn = match config.opt.commit_style {
        cli::SectionStyle::Box => draw::write_boxed_with_line,
        cli::SectionStyle::Underline => draw::write_underlined,
        cli::SectionStyle::Plain => panic!(),
    };
    draw_fn(
        painter.writer,
        line,
        config.terminal_width,
        Yellow.normal(),
        true,
    )?;
    Ok(())
}

fn write_file_meta_header_line(
    painter: &mut Painter,
    line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let draw_fn = match config.opt.file_style {
        cli::SectionStyle::Box => draw::write_boxed_with_line,
        cli::SectionStyle::Underline => draw::write_underlined,
        cli::SectionStyle::Plain => panic!(),
    };
    let ansi_style = Blue.bold();
    draw_fn(
        painter.writer,
        &ansi_style.paint(parse::get_file_change_description_from_diff_line(&line)),
        config.terminal_width,
        ansi_style,
        true,
    )?;
    Ok(())
}

fn write_hunk_meta_line(painter: &mut Painter, line: &str, config: &Config) -> std::io::Result<()> {
    let draw_fn = match config.opt.hunk_style {
        cli::SectionStyle::Box => draw::write_boxed,
        cli::SectionStyle::Underline => draw::write_underlined,
        cli::SectionStyle::Plain => panic!(),
    };
    let ansi_style = Blue.normal();
    let (code_fragment, line_number) = parse::parse_hunk_metadata(&line);
    if code_fragment.len() > 0 {
        painter.paint_lines(
            vec![code_fragment.clone()],
            vec![vec![(
                style::NO_BACKGROUND_COLOR_STYLE_MODIFIER,
                code_fragment.clone(),
            )]],
            true,
        );
        painter.output_buffer.pop(); // trim newline
        draw_fn(
            painter.writer,
            &painter.output_buffer,
            config.terminal_width,
            ansi_style,
            false,
        )?;
        painter.output_buffer.truncate(0);
    }
    writeln!(painter.writer, "\n{}", ansi_style.paint(line_number))?;
    Ok(())
}

fn paint_hunk_line(state: State, painter: &mut Painter, line: &str, config: &Config) -> State {
    match line.chars().next() {
        Some('-') => {
            if state == State::HunkPlus {
                painter.paint_buffered_lines();
            }
            painter.minus_lines.push(prepare(&line, config));
            State::HunkMinus
        }
        Some('+') => {
            painter.plus_lines.push(prepare(&line, config));
            State::HunkPlus
        }
        _ => {
            painter.paint_buffered_lines();
            let line = prepare(&line, config);
            painter.paint_lines(
                vec![line.clone()],
                vec![vec![(
                    style::NO_BACKGROUND_COLOR_STYLE_MODIFIER,
                    line.clone(),
                )]],
                true,
            );
            State::HunkZero
        }
    }
}

/// Replace initial -/+ character with ' ', pad to width, and terminate with newline character.
fn prepare(_line: &str, config: &Config) -> String {
    let mut line = String::new();
    if _line.len() > 0 {
        line.push_str(" ");
        line.push_str(&_line[1..]);
    }
    match config.width {
        Some(width) if width > line.len() => {
            format!("{}{}\n", line, " ".repeat(width - line.len()))
        }
        _ => format!("{}\n", line),
    }
}
