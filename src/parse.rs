use console::strip_ansi_codes;

use crate::bat::assets::HighlightingAssets;
use crate::bat::output::{OutputType, PagingMode};
use crate::paint::{Config, Painter};
use crate::parse::parse_git_diff::get_file_extension_from_diff_line;

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
// | from \ to | Commit      | DiffMeta    | HunkMeta    | HunkZero    | HunkMinus   | HunkPlus |
// |-----------+-------------+-------------+-------------+-------------+-------------+----------|
// | Commit    | emit        | emit        |             |             |             |          |
// | DiffMeta  |             | emit        | emit        |             |             |          |
// | HunkMeta  |             |             |             | emit        | push        | push     |
// | HunkZero  | emit        | emit        | emit        | emit        | push        | push     |
// | HunkMinus | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus  | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

pub fn delta(
    lines: impl Iterator<Item = String>,
    config: &Config,
    assets: &HighlightingAssets,
) -> std::io::Result<()> {
    let mut output_type =
        OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(config.pager)).unwrap();

    // TODO: Painter::new(config)
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
        let line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("diff --") {
            painter.paint_and_emit_buffered_lines()?;
            state = State::DiffMeta;
            painter.syntax = match get_file_extension_from_diff_line(&line) {
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            }
        } else if line.starts_with("commit") {
            painter.paint_and_emit_buffered_lines()?;
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
        } else if state.is_in_hunk() && painter.syntax.is_some() {
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

mod parse_git_diff {
    use std::path::Path;

    /// Given input like
    /// "diff --git a/src/main.rs b/src/main.rs"
    /// Return "rs", i.e. a single file extension consistent with both files.
    pub fn get_file_extension_from_diff_line(line: &str) -> Option<&str> {
        match get_file_extensions_from_diff_line(line) {
            (Some(ext1), Some(ext2)) => {
                if ext1 == ext2 {
                    Some(ext1)
                } else {
                    // Unexpected: old and new files have different extensions.
                    None
                }
            }
            (Some(ext1), None) => Some(ext1),
            (None, Some(ext2)) => Some(ext2),
            (None, None) => None,
        }
    }

    /// Given input like "diff --git a/src/main.rs b/src/main.rs"
    /// return ("rs", "rs").
    fn get_file_extensions_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
        let mut iter = line.split(" ");
        iter.next(); // diff
        iter.next(); // --git
        (
            iter.next().and_then(|s| get_extension(&s[2..])),
            iter.next().and_then(|s| get_extension(&s[2..])),
        )
    }

    /// Attempt to parse input as a file path and return extension as a &str.
    fn get_extension(s: &str) -> Option<&str> {
        let path = Path::new(s);
        path.extension()
            .and_then(|e| e.to_str())
            // E.g. 'Makefile' is the file name and also the extension
            .or_else(|| path.file_name().and_then(|s| s.to_str()))
    }
}
