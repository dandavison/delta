use std::io::Write;

use ansi_term::Colour::{Blue, Yellow};
use console::strip_ansi_codes;
use unicode_segmentation::UnicodeSegmentation;

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

pub fn delta<I>(
    lines: I,
    config: &Config,
    assets: &HighlightingAssets,
    writer: &mut Write,
) -> std::io::Result<()>
where
    I: Iterator<Item = String>,
{
    let mut painter = Painter::new(writer, config, assets);
    let mut minus_file = "".to_string();
    let mut plus_file;
    let mut state = State::Unknown;

    for raw_line in lines {
        let line = strip_ansi_codes(&raw_line).to_string();
        if line.starts_with("commit ") {
            painter.paint_buffered_lines();
            state = State::CommitMeta;
            if config.opt.commit_style != cli::SectionStyle::Plain {
                painter.emit()?;
                handle_commit_meta_header_line(&mut painter, &raw_line, config)?;
                continue;
            }
        } else if line.starts_with("diff --git ") {
            painter.paint_buffered_lines();
            state = State::FileMeta;
            painter.syntax = match parse::get_file_extension_from_diff_line(&line) {
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if (line.starts_with("--- ") || line.starts_with("rename from "))
            && config.opt.file_style != cli::SectionStyle::Plain
        {
            minus_file = parse::get_file_path_from_file_meta_line(&line);
        } else if (line.starts_with("+++ ") || line.starts_with("rename to "))
            && config.opt.file_style != cli::SectionStyle::Plain
        {
            plus_file = parse::get_file_path_from_file_meta_line(&line);
            painter.emit()?;
            handle_file_meta_header_line(&mut painter, &minus_file, &plus_file, config)?;
        } else if line.starts_with("@@ ") {
            state = State::HunkMeta;
            if painter.syntax.is_some() {
                painter.reset_highlighter();
            }
            if config.opt.hunk_style != cli::SectionStyle::Plain {
                painter.emit()?;
                handle_hunk_meta_line(&mut painter, &line, config)?;
                continue;
            }
        } else if state.is_in_hunk() && painter.syntax.is_some() {
            state = handle_hunk_line(&mut painter, &line, state, config);
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

fn handle_commit_meta_header_line(
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

fn handle_file_meta_header_line(
    painter: &mut Painter,
    minus_file: &str,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let draw_fn = match config.opt.file_style {
        cli::SectionStyle::Box => draw::write_boxed_with_line,
        cli::SectionStyle::Underline => draw::write_underlined,
        cli::SectionStyle::Plain => panic!(),
    };
    let ansi_style = Blue.bold();
    write!(painter.writer, "\n")?;
    draw_fn(
        painter.writer,
        &ansi_style.paint(parse::get_file_change_description_from_file_paths(
            minus_file, plus_file,
        )),
        config.terminal_width,
        ansi_style,
        true,
    )?;
    Ok(())
}

fn handle_hunk_meta_line(
    painter: &mut Painter,
    line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let draw_fn = match config.opt.hunk_style {
        cli::SectionStyle::Box => draw::write_boxed,
        cli::SectionStyle::Underline => draw::write_underlined,
        cli::SectionStyle::Plain => panic!(),
    };
    let ansi_style = Blue.normal();
    let (code_fragment, line_number) = parse::parse_hunk_metadata(&line);
    if code_fragment.len() > 0 {
        let syntax_style_sections = Painter::get_line_syntax_style_sections(
            code_fragment,
            &mut painter.highlighter,
            &painter.config,
            true,
        );
        Painter::paint_lines(
            &mut painter.output_buffer,
            vec![syntax_style_sections],
            vec![vec![(
                style::NO_BACKGROUND_COLOR_STYLE_MODIFIER,
                code_fragment,
            )]],
        );
        painter.output_buffer.pop(); // trim newline
        draw_fn(
            painter.writer,
            &painter.output_buffer,
            config.terminal_width,
            ansi_style,
            false,
        )?;
        painter.output_buffer.clear();
    }
    writeln!(painter.writer, "\n{}", ansi_style.paint(line_number))?;
    Ok(())
}

/// Handle a hunk line, i.e. a minus line, a plus line, or an unchanged line.
// In the case of a minus or plus line, we store the line in a
// buffer. When we exit the changed region we process the collected
// minus and plus lines jointly, in order to paint detailed
// highlighting according to inferred edit operations. In the case of
// an unchanged line, we paint it immediately.
fn handle_hunk_line(painter: &mut Painter, line: &str, state: State, config: &Config) -> State {
    // Don't let the line buffers become arbitrarily large -- if we
    // were to allow that, then for a large deleted/added file we
    // would process the entire file before painting anything.
    if painter.minus_lines.len() > config.max_buffered_lines
        || painter.plus_lines.len() > config.max_buffered_lines
    {
        painter.paint_buffered_lines();
    }
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
            let syntax_style_sections = Painter::get_line_syntax_style_sections(
                &line,
                &mut painter.highlighter,
                &painter.config,
                true,
            );
            Painter::paint_lines(
                &mut painter.output_buffer,
                vec![syntax_style_sections],
                vec![vec![(style::NO_BACKGROUND_COLOR_STYLE_MODIFIER, &line)]],
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
    let line_length = line.graphemes(true).count();
    match config.width {
        Some(width) if width > line_length => {
            format!("{}{}\n", line, " ".repeat(width - line_length))
        }
        _ => format!("{}\n", line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::strip_ansi_codes;
    use structopt::StructOpt;

    #[test]
    fn test_added_file() {
        let input = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

diff --git a/a.py b/a.py
new file mode 100644
index 0000000..8c55b7d
--- /dev/null
+++ b/a.py
@@ -0,0 +1,3 @@
+# hello
+class X:
+    pass";

        let expected_output = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
added: a.py
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
────────────────────────────────────────────────────────────────────────────────

────────────────────────────────────────────────────────────────────────────────
 # hello
 class X:
     pass
";

        let mut opt = cli::Opt::from_args();
        opt.width = Some("variable".to_string());
        let assets = HighlightingAssets::new();
        let config = cli::process_command_line_arguments(&assets, &opt);
        let mut writer: Vec<u8> = Vec::new();
        delta(
            input.split("\n").map(String::from),
            &config,
            &assets,
            &mut writer,
        )
        .unwrap();
        let output = strip_ansi_codes(&String::from_utf8(writer).unwrap()).to_string();
        assert!(output.contains("\nadded: a.py\n"));
        if false {
            // TODO: hline width
            assert_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_renamed_file() {
        let input = "\
commit 1281650789680f1009dfff2497d5ccfbe7b96526
Author: Dan Davison <dandavison7@gmail.com>
Date:   Wed Jul 17 20:40:23 2019 -0400

    rename

diff --git a/a.py b/b.py
similarity index 100%
rename from a.py
rename to b.py
";

        let mut opt = cli::Opt::from_args();
        opt.width = Some("variable".to_string());
        let assets = HighlightingAssets::new();
        let config = cli::process_command_line_arguments(&assets, &opt);
        let mut writer: Vec<u8> = Vec::new();
        delta(
            input.split("\n").map(String::from),
            &config,
            &assets,
            &mut writer,
        )
        .unwrap();
        let output = strip_ansi_codes(&String::from_utf8(writer).unwrap()).to_string();
        assert!(output.contains("\nrenamed: a.py ⟶   b.py\n"));
    }
}
