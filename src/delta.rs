use std::borrow::Cow;
use std::io::BufRead;
use std::io::Write;

use bytelines::ByteLines;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::cli;
use crate::config::Config;
use crate::draw;
use crate::features;
use crate::format;
use crate::hunk_header;
use crate::paint::Painter;
use crate::parse;
use crate::style::{self, DecorationStyle};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    CommitMeta,                // In commit metadata section
    FileMeta, // In diff metadata section, between (possible) commit metadata and first hunk
    HunkHeader, // In hunk metadata line
    HunkZero, // In hunk; unchanged line
    HunkMinus(Option<String>), // In hunk; removed line (raw_line)
    HunkPlus(Option<String>), // In hunk; added line (raw_line)
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum Source {
    GitDiff,     // Coming from a `git diff` command
    DiffUnified, // Coming from a `diff -u` command
    Unknown,
}

impl State {
    fn is_in_hunk(&self) -> bool {
        matches!(*self, State::HunkHeader | State::HunkZero | State::HunkMinus(_) | State::HunkPlus(_))
    }
}

// Possible transitions, with actions on entry:
//
//
// | from \ to   | CommitMeta  | FileMeta    | HunkHeader  | HunkZero    | HunkMinus   | HunkPlus |
// |-------------+-------------+-------------+-------------+-------------+-------------+----------|
// | CommitMeta  | emit        | emit        |             |             |             |          |
// | FileMeta    |             | emit        | emit        |             |             |          |
// | HunkHeader  |             |             |             | emit        | push        | push     |
// | HunkZero    | emit        | emit        | emit        | emit        | push        | push     |
// | HunkMinus   | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus    | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

struct StateMachine<'a> {
    state: State,
    source: Source,
    minus_file: String,
    plus_file: String,
    file_event: parse::FileEvent,
    painter: Painter<'a>,
    config: &'a Config,

    // When a file is modified, we use lines starting with '---' or '+++' to obtain the file name.
    // When a file is renamed without changes, we use lines starting with 'rename' to obtain the
    // file name (there is no diff hunk and hence no lines starting with '---' or '+++'). But when
    // a file is renamed with changes, both are present, and we rely on the following variables to
    // avoid emitting the file meta header line twice (#245).
    current_file_pair: Option<(String, String)>,
    handled_file_meta_header_line_file_pair: Option<(String, String)>,
}

impl<'a> StateMachine<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a Config) -> Self {
        Self {
            state: State::Unknown,
            source: Source::Unknown,
            minus_file: "".to_string(),
            plus_file: "".to_string(),
            file_event: parse::FileEvent::NoEvent,
            current_file_pair: None,
            handled_file_meta_header_line_file_pair: None,
            painter: Painter::new(writer, config),
            config,
        }
    }
}

pub fn delta<I>(
    mut lines: ByteLines<I>,
    writer: &mut dyn Write,
    config: &Config,
) -> std::io::Result<()>
where
    I: BufRead,
{
    let mut machine = StateMachine::new(writer, config);

    while let Some(Ok(raw_line_bytes)) = lines.next() {
        let raw_line = String::from_utf8_lossy(&raw_line_bytes);
        let raw_line = if config.max_line_length > 0 && raw_line.len() > config.max_line_length {
            ansi::truncate_str(&raw_line, config.max_line_length, &config.truncation_symbol)
        } else {
            raw_line
        };
        let line = ansi::strip_ansi_codes(&raw_line).to_string();

        if machine.source == Source::Unknown {
            machine.source = detect_source(&line);
        }

        let mut handled_line = if line.starts_with("commit ") {
            machine.handle_commit_meta_header_line(&line, &raw_line)?
        } else if line.starts_with("diff ") {
            machine.handle_file_meta_diff_line()?
        } else if (machine.state == State::FileMeta || machine.source == Source::DiffUnified)
            && (line.starts_with("--- ")
                || line.starts_with("rename from ")
                || line.starts_with("copy from "))
        {
            machine.handle_file_meta_minus_line(&line, &raw_line)?
        } else if (machine.state == State::FileMeta || machine.source == Source::DiffUnified)
            && (line.starts_with("+++ ")
                || line.starts_with("rename to ")
                || line.starts_with("copy to "))
        {
            machine.handle_file_meta_plus_line(&line, &raw_line)?
        } else if line.starts_with("@@") {
            machine.handle_hunk_header_line(&line, &raw_line)?
        } else if machine.source == Source::DiffUnified && line.starts_with("Only in ")
            || line.starts_with("Submodule ")
            || line.starts_with("Binary files ")
        {
            machine.handle_additional_file_meta_cases(&line, &raw_line)?
        } else if machine.state.is_in_hunk() {
            // A true hunk line should start with one of: '+', '-', ' '. However, handle_hunk_line
            // handles all lines until the state machine transitions away from the hunk states.
            machine.handle_hunk_line(&line, &raw_line)?
        } else {
            false
        };
        if machine.state == State::FileMeta && machine.should_handle() && !config.color_only {
            // The file metadata section is 4 lines. Skip them under non-plain file-styles.
            // However in the case of color_only mode,
            // we won't skip because we can't change raw_line structure.
            handled_line = true
        }
        if !handled_line {
            machine.painter.emit()?;
            writeln!(
                machine.painter.writer,
                "{}",
                format::format_raw_line(&raw_line, config)
            )?;
        }
    }

    machine.painter.paint_buffered_minus_and_plus_lines();
    machine.painter.emit()?;
    Ok(())
}

impl<'a> StateMachine<'a> {
    /// Should a handle_* function be called on this element?
    fn should_handle(&self) -> bool {
        let style = self.config.get_style(&self.state);
        !(style.is_raw && style.decoration_style == DecorationStyle::NoDecoration)
    }

    fn handle_commit_meta_header_line(
        &mut self,
        line: &str,
        raw_line: &str,
    ) -> std::io::Result<bool> {
        let mut handled_line = false;
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::CommitMeta;
        if self.should_handle() {
            self.painter.emit()?;
            self._handle_commit_meta_header_line(&line, &raw_line)?;
            handled_line = true
        }
        Ok(handled_line)
    }

    fn _handle_commit_meta_header_line(
        &mut self,
        line: &str,
        raw_line: &str,
    ) -> std::io::Result<()> {
        if self.config.commit_style.is_omitted {
            return Ok(());
        }
        let (mut draw_fn, pad, decoration_ansi_term_style) =
            draw::get_draw_function(self.config.commit_style.decoration_style);
        let (formatted_line, formatted_raw_line) = if self.config.hyperlinks {
            (
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    line,
                    self.config,
                ),
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    raw_line,
                    self.config,
                ),
            )
        } else {
            (Cow::from(line), Cow::from(raw_line))
        };

        draw_fn(
            self.painter.writer,
            &format!("{}{}", formatted_line, if pad { " " } else { "" }),
            &format!("{}{}", formatted_raw_line, if pad { " " } else { "" }),
            &self.config.decorations_width,
            self.config.commit_style,
            decoration_ansi_term_style,
        )?;
        Ok(())
    }

    fn handle_file_meta_diff_line(&mut self) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::FileMeta;
        self.handled_file_meta_header_line_file_pair = None;
        Ok(false)
    }

    fn handle_file_meta_minus_line(&mut self, line: &str, raw_line: &str) -> std::io::Result<bool> {
        let mut handled_line = false;

        let parsed_file_meta_line =
            parse::parse_file_meta_line(&line, self.source == Source::GitDiff);
        self.minus_file = parsed_file_meta_line.0;
        self.file_event = parsed_file_meta_line.1;

        if self.source == Source::DiffUnified {
            self.state = State::FileMeta;
            self.painter
                .set_syntax(parse::get_file_extension_from_marker_line(&line));
        } else {
            self.painter
                .set_syntax(parse::get_file_extension_from_file_meta_line_file_path(
                    &self.minus_file,
                ));
        }

        // In color_only mode, raw_line's structure shouldn't be changed.
        // So it needs to avoid fn _handle_file_meta_header_line
        // (it connects the plus_file and minus_file),
        // and to call fn handle_generic_file_meta_header_line directly.
        if self.config.color_only {
            self._handle_generic_file_meta_header_line(&line, &raw_line)?;
            handled_line = true;
        }
        Ok(handled_line)
    }

    fn handle_file_meta_plus_line(&mut self, line: &str, raw_line: &str) -> std::io::Result<bool> {
        let mut handled_line = false;
        let parsed_file_meta_line =
            parse::parse_file_meta_line(&line, self.source == Source::GitDiff);
        self.plus_file = parsed_file_meta_line.0;
        self.painter
            .set_syntax(parse::get_file_extension_from_file_meta_line_file_path(
                &self.plus_file,
            ));
        self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));

        // In color_only mode, raw_line's structure shouldn't be changed.
        // So it needs to avoid fn _handle_file_meta_header_line
        // (it connects the plus_file and minus_file),
        // and to call fn handle_generic_file_meta_header_line directly.
        if self.config.color_only {
            self._handle_generic_file_meta_header_line(&line, &raw_line)?;
            handled_line = true
        } else if self.should_handle()
            && self.handled_file_meta_header_line_file_pair != self.current_file_pair
        {
            self.painter.emit()?;
            self._handle_file_meta_header_line(self.source == Source::DiffUnified)?;
            self.handled_file_meta_header_line_file_pair = self.current_file_pair.clone()
        }
        Ok(handled_line)
    }

    /// Construct file change line from minus and plus file and write with FileMeta styling.
    fn _handle_file_meta_header_line(&mut self, comparing: bool) -> std::io::Result<()> {
        let line = parse::get_file_change_description_from_file_paths(
            &self.minus_file,
            &self.plus_file,
            comparing,
            &self.file_event,
            self.config,
        );
        // FIXME: no support for 'raw'
        self._handle_generic_file_meta_header_line(&line, &line)
    }

    fn handle_additional_file_meta_cases(
        &mut self,
        line: &str,
        raw_line: &str,
    ) -> std::io::Result<bool> {
        let mut handled_line = false;

        // Additional FileMeta cases:
        //
        // 1. When comparing directories with diff -u, if filenames match between the
        //    directories, the files themselves will be compared. However, if an equivalent
        //    filename is not present, diff outputs a single line (Only in...) starting
        //    indicating that the file is present in only one of the directories.
        //
        // 2. Git diff emits lines describing submodule state such as "Submodule x/y/z contains
        //    untracked content"
        //
        // See https://github.com/dandavison/delta/issues/60#issuecomment-557485242 for a
        // proposal for more robust parsing logic.

        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::FileMeta;
        if self.should_handle() {
            self.painter.emit()?;
            self._handle_generic_file_meta_header_line(&line, &raw_line)?;
            handled_line = true;
        }

        Ok(handled_line)
    }

    /// Write `line` with FileMeta styling.
    fn _handle_generic_file_meta_header_line(
        &mut self,
        line: &str,
        raw_line: &str,
    ) -> std::io::Result<()> {
        // If file_style is "omit", we'll skip the process and print nothing.
        // However in the case of color_only mode,
        // we won't skip because we can't change raw_line structure.
        if self.config.file_style.is_omitted && !self.config.color_only {
            return Ok(());
        }
        let (mut draw_fn, pad, decoration_ansi_term_style) =
            draw::get_draw_function(self.config.file_style.decoration_style);
        // Prints the new line below file-meta-line.
        // However in the case of color_only mode,
        // we won't print it because we can't change raw_line structure.
        if !self.config.color_only {
            writeln!(self.painter.writer)?;
        }
        draw_fn(
            self.painter.writer,
            &format!("{}{}", line, if pad { " " } else { "" }),
            &format!("{}{}", raw_line, if pad { " " } else { "" }),
            &self.config.decorations_width,
            self.config.file_style,
            decoration_ansi_term_style,
        )?;
        Ok(())
    }

    /// Emit the hunk header, with any requested decoration.
    fn handle_hunk_header_line(&mut self, line: &str, raw_line: &str) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::HunkHeader;
        self.painter.set_highlighter();
        self.painter.emit()?;

        let (raw_code_fragment, line_numbers) = parse::parse_hunk_header(&line);
        if self.config.line_numbers {
            self.painter
                .line_numbers_data
                .initialize_hunk(&line_numbers, self.plus_file.to_string());
        }

        if self.config.hunk_header_style.is_raw {
            hunk_header::write_hunk_header_raw(&mut self.painter, line, raw_line, self.config)?;
        } else if self.config.hunk_header_style.is_omitted {
            writeln!(self.painter.writer)?;
        } else {
            hunk_header::write_hunk_header(
                &raw_code_fragment,
                &line_numbers,
                &mut self.painter,
                line,
                &self.plus_file,
                self.config,
            )?;
        };
        self.painter.set_highlighter();
        Ok(true)
    }

    /// Handle a hunk line, i.e. a minus line, a plus line, or an unchanged line.
    // In the case of a minus or plus line, we store the line in a
    // buffer. When we exit the changed region we process the collected
    // minus and plus lines jointly, in order to paint detailed
    // highlighting according to inferred edit operations. In the case of
    // an unchanged line, we paint it immediately.
    fn handle_hunk_line(&mut self, line: &str, raw_line: &str) -> std::io::Result<bool> {
        // Don't let the line buffers become arbitrarily large -- if we
        // were to allow that, then for a large deleted/added file we
        // would process the entire file before painting anything.
        if self.painter.minus_lines.len() > self.config.line_buffer_size
            || self.painter.plus_lines.len() > self.config.line_buffer_size
        {
            self.painter.paint_buffered_minus_and_plus_lines();
        }
        self.state = match line.chars().next() {
            Some('-') => {
                if let State::HunkPlus(_) = self.state {
                    self.painter.paint_buffered_minus_and_plus_lines();
                }
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            raw_line,
                            [*style::GIT_DEFAULT_MINUS_STYLE, self.config.git_minus_style].iter(),
                        ) =>
                    {
                        State::HunkMinus(Some(self.painter.prepare_raw_line(raw_line)))
                    }
                    _ => State::HunkMinus(None),
                };
                self.painter
                    .minus_lines
                    .push((self.painter.prepare(&line), state.clone()));
                state
            }
            Some('+') => {
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            raw_line,
                            [*style::GIT_DEFAULT_PLUS_STYLE, self.config.git_plus_style].iter(),
                        ) =>
                    {
                        State::HunkPlus(Some(self.painter.prepare_raw_line(raw_line)))
                    }
                    _ => State::HunkPlus(None),
                };
                self.painter
                    .plus_lines
                    .push((self.painter.prepare(&line), state.clone()));
                state
            }
            Some(' ') => {
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter.paint_zero_line(&line);
                State::HunkZero
            }
            _ => {
                // The first character here could be e.g. '\' from '\ No newline at end of file'. This
                // is not a hunk line, but the parser does not have a more accurate state corresponding
                // to this.
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter
                    .output_buffer
                    .push_str(&self.painter.expand_tabs(raw_line.graphemes(true)));
                self.painter.output_buffer.push('\n');
                State::HunkZero
            }
        };
        self.painter.emit()?;
        Ok(true)
    }
}

/// Try to detect what is producing the input for delta.
///
/// Currently can detect:
/// * git diff
/// * diff -u
fn detect_source(line: &str) -> Source {
    if line.starts_with("commit ") || line.starts_with("diff --git ") {
        Source::GitDiff
    } else if line.starts_with("diff -u")
        || line.starts_with("diff -ru")
        || line.starts_with("diff -r -u")
        || line.starts_with("diff -U")
        || line.starts_with("--- ")
        || line.starts_with("Only in ")
    {
        Source::DiffUnified
    } else {
        Source::Unknown
    }
}
