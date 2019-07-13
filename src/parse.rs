use std::io::Write;

use ansi_term::Colour::{Blue, Yellow};
use box_drawing;
use console::strip_ansi_codes;

use crate::bat::assets::HighlightingAssets;
use crate::cli;
use crate::draw;
use crate::paint::{Config, Painter, NO_BACKGROUND_COLOR_STYLE_MODIFIER};
use crate::parse::parse_git_diff::{
    get_file_change_description_from_diff_line, get_file_extension_from_diff_line,
    parse_hunk_metadata,
};

#[derive(Debug, PartialEq)]
pub enum State {
    Commit,    // In commit metadata section
    FileMeta,  // In diff metadata section, between commit metadata and first hunk
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
// | from \ to | Commit      | FileMeta    | HunkMeta    | HunkZero    | HunkMinus   | HunkPlus |
// |-----------+-------------+-------------+-------------+-------------+-------------+----------|
// | Commit    | emit        | emit        |             |             |             |          |
// | FileMeta  |             | emit        | emit        |             |             |          |
// | HunkMeta  |             |             |             | emit        | push        | push     |
// | HunkZero  | emit        | emit        | emit        | emit        | push        | push     |
// | HunkMinus | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus  | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

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
        config: config,
    };

    let mut state = State::Unknown;

    for raw_line in lines {
        let line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("commit") {
            painter.paint_buffered_lines();
            state = State::Commit;
            match config.opt.commit_style {
                cli::SectionStyle::Plain => (),
                cli::SectionStyle::Box => {
                    painter.emit()?;
                    let ansi_style = Yellow.normal();
                    let box_width = line.len() + 1;
                    draw::write_boxed_with_line(
                        &raw_line,
                        box_width,
                        ansi_style,
                        true,
                        painter.writer,
                    )?;
                    write!(
                        painter.writer,
                        "{}",
                        ansi_style.paint(
                            box_drawing::heavy::HORIZONTAL
                                .repeat(config.terminal_width - box_width - 1),
                        )
                    )?;
                    continue;
                }
                cli::SectionStyle::Underline => {
                    panic!("--commit-style underline is not implemented!") // TODO
                }
            }
        } else if line.starts_with("diff --") {
            painter.paint_buffered_lines();
            state = State::FileMeta;
            painter.syntax = match get_file_extension_from_diff_line(&line) {
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
            match config.opt.file_style {
                cli::SectionStyle::Plain => (),
                cli::SectionStyle::Box => {
                    painter.emit()?;
                    let file_change_description = get_file_change_description_from_diff_line(&line);

                    let ansi_style = Blue.bold();
                    let box_width = file_change_description.len() + 1;
                    draw::write_boxed_with_line(
                        &file_change_description,
                        box_width,
                        ansi_style,
                        true,
                        painter.writer,
                    )?;
                    write!(
                        painter.writer,
                        "{}",
                        ansi_style.paint(
                            box_drawing::heavy::HORIZONTAL
                                .repeat(config.terminal_width - box_width - 1),
                        )
                    )?;
                    continue;
                }
                cli::SectionStyle::Underline => {
                    painter.emit()?;
                    let file_change_description = get_file_change_description_from_diff_line(&line);

                    let ansi_style = Blue.bold();
                    writeln!(
                        painter.writer,
                        "\n{}\n{}",
                        ansi_style.paint(file_change_description),
                        ansi_style
                            .paint(box_drawing::heavy::HORIZONTAL.repeat(config.terminal_width))
                    )?;
                    continue;
                }
            }
        } else if line.starts_with("@@") {
            state = State::HunkMeta;
            match config.opt.hunk_style {
                cli::SectionStyle::Plain => (),
                cli::SectionStyle::Box => {
                    painter.emit()?;
                    let (code_fragment, line_number) = parse_hunk_metadata(&line);
                    let ansi_style = Blue.normal();
                    if code_fragment.len() > 0 {
                        painter.paint_lines(
                            vec![code_fragment.clone()],
                            vec![vec![(
                                NO_BACKGROUND_COLOR_STYLE_MODIFIER,
                                code_fragment.clone(),
                            )]],
                        );
                        painter.output_buffer.pop(); // trim newline
                        draw::write_boxed(
                            &painter.output_buffer,
                            code_fragment.len() + 1,
                            ansi_style,
                            false,
                            &mut painter.writer,
                        )?;
                    }
                    writeln!(painter.writer, "\n{}", ansi_style.paint(line_number))?;
                    painter.output_buffer.truncate(0);
                    continue;
                }
                cli::SectionStyle::Underline => {
                    panic!("--hunk-style underline is not implemented!") // TODO
                }
            }
        } else if state.is_in_hunk() && painter.syntax.is_some() {
            match line.chars().next() {
                Some('-') => {
                    if state == State::HunkPlus {
                        painter.paint_buffered_lines();
                    }
                    painter.minus_lines.push(prepare(&line, config));
                    state = State::HunkMinus;
                }
                Some('+') => {
                    painter.plus_lines.push(prepare(&line, config));
                    state = State::HunkPlus;
                }
                _ => {
                    painter.paint_buffered_lines();
                    state = State::HunkZero;
                    let line = prepare(&line, config);
                    painter.paint_lines(
                        vec![line.clone()],
                        vec![vec![(NO_BACKGROUND_COLOR_STYLE_MODIFIER, line.clone())]],
                    );
                }
            };
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

/// Replace initial -/+ character with ' ' and pad to width.
fn prepare(_line: &str, config: &Config) -> String {
    let mut line = String::new();
    if _line.len() > 0 {
        line.push_str(" ");
        line.push_str(&_line[1..]);
    }
    match config.width {
        Some(width) => {
            if line.len() < width {
                line = format!("{}{}", line, " ".repeat(width - line.len()));
            }
        }
        _ => (),
    }
    line
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

    // TODO: Don't parse the line twice (once for change description and once for extensions).
    pub fn get_file_change_description_from_diff_line(line: &str) -> String {
        match get_file_paths_from_diff_line(line) {
            (Some(file_1), Some(file_2)) if file_1 == file_2 => format!("{}", file_1),
            (Some(file), Some("/dev/null")) => format!("deleted: {}", file),
            (Some("/dev/null"), Some(file)) => format!("added: {}", file),
            (Some(file_1), Some(file_2)) => format!("renamed: {} ⟶  {}", file_1, file_2),
            _ => format!("?"),
        }
    }

    /// Given input like
    /// "@@ -74,15 +74,14 @@ pub fn delta("
    /// Return " pub fn delta("
    pub fn parse_hunk_metadata(line: &str) -> (String, String) {
        let mut iter = line.split("@@").skip(1);
        let line_number = iter
            .next()
            .and_then(|s| {
                s.split("+")
                    .skip(1)
                    .next()
                    .and_then(|s| s.split(",").next())
            })
            .unwrap_or("")
            .to_string();
        let code_fragment = iter.next().unwrap_or("").to_string();
        (code_fragment, line_number)
    }

    fn get_file_paths_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
        let mut iter = line.split(" ");
        iter.next(); // diff
        iter.next(); // --git
        (
            iter.next().and_then(|s| Some(&s[2..])),
            iter.next().and_then(|s| Some(&s[2..])),
        )
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_get_file_extension_from_diff_line() {
            assert_eq!(
                get_file_extension_from_diff_line("diff --git a/src/main.rs b/src/main.rs"),
                Some("rs")
            );
        }

        #[test]
        fn test_get_file_change_description_from_diff_line() {
            assert_eq!(
                get_file_change_description_from_diff_line(
                    "diff --git a/src/main.rs b/src/main.rs"
                ),
                "src/main.rs"
            );
        }

        #[test]
        fn test_parse_hunk_metadata() {
            assert_eq!(
                parse_hunk_metadata("@@ -74,15 +75,14 @@ pub fn delta(\n"),
                (" pub fn delta(\n".to_string(), "75".to_string())
            );
        }
    }

}
