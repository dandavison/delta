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
        matches!(
            *self,
            State::HunkHeader | State::HunkZero | State::HunkMinus(_) | State::HunkPlus(_)
        )
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
    line: String,
    raw_line: String,
    state: State,
    source: Source,
    minus_file: String,
    plus_file: String,
    minus_file_event: parse::FileEvent,
    plus_file_event: parse::FileEvent,
    diff_line: String,
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

pub fn delta<I>(lines: ByteLines<I>, writer: &mut dyn Write, config: &Config) -> std::io::Result<()>
where
    I: BufRead,
{
    StateMachine::new(writer, config).consume(lines)
}

impl<'a> StateMachine<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a Config) -> Self {
        Self {
            line: "".to_string(),
            raw_line: "".to_string(),
            state: State::Unknown,
            source: Source::Unknown,
            minus_file: "".to_string(),
            plus_file: "".to_string(),
            minus_file_event: parse::FileEvent::NoEvent,
            plus_file_event: parse::FileEvent::NoEvent,
            diff_line: "".to_string(),
            current_file_pair: None,
            handled_file_meta_header_line_file_pair: None,
            painter: Painter::new(writer, config),
            config,
        }
    }

    fn consume<I>(&mut self, mut lines: ByteLines<I>) -> std::io::Result<()>
    where
        I: BufRead,
    {
        while let Some(Ok(raw_line_bytes)) = lines.next() {
            self.ingest_line(raw_line_bytes);
            let line = &self.line;

            if self.source == Source::Unknown {
                self.source = detect_source(&line);
            }

            let mut handled_line = if self.config.commit_regex.is_match(line) {
                self.handle_commit_meta_header_line()?
            } else if (self.state == State::CommitMeta || self.state == State::Unknown)
                && line.starts_with(' ')
            {
                self.handle_diff_stat_line()?
            } else if line.starts_with("diff ") {
                self.handle_file_meta_diff_line()?
            } else if (self.state == State::FileMeta || self.source == Source::DiffUnified)
                && (line.starts_with("--- ")
                    || line.starts_with("rename from ")
                    || line.starts_with("copy from ")
                    || line.starts_with("old mode "))
            {
                self.handle_file_meta_minus_line()?
            } else if (self.state == State::FileMeta || self.source == Source::DiffUnified)
                && (line.starts_with("+++ ")
                    || line.starts_with("rename to ")
                    || line.starts_with("copy to ")
                    || line.starts_with("new mode "))
            {
                self.handle_file_meta_plus_line()?
            } else if line.starts_with("@@") {
                self.handle_hunk_header_line()?
            } else if self.source == Source::DiffUnified && line.starts_with("Only in ")
                || line.starts_with("Submodule ")
                || line.starts_with("Binary files ")
            {
                self.handle_additional_file_meta_cases()?
            } else if self.state.is_in_hunk() {
                // A true hunk line should start with one of: '+', '-', ' '. However, handle_hunk_line
                // handles all lines until the state transitions away from the hunk states.
                self.handle_hunk_line()?
            } else {
                false
            };
            if self.state == State::FileMeta && self.should_handle() && !self.config.color_only {
                // The file metadata section is 4 lines. Skip them under non-plain file-styles.
                // However in the case of color_only mode,
                // we won't skip because we can't change raw_line structure.
                handled_line = true
            }
            if !handled_line {
                self.painter.emit()?;
                writeln!(
                    self.painter.writer,
                    "{}",
                    format::format_raw_line(&self.raw_line, self.config)
                )?;
            }
        }

        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.emit()?;
        Ok(())
    }

    fn ingest_line(&mut self, raw_line_bytes: &[u8]) {
        // TODO: retain raw_line as Cow
        self.raw_line = String::from_utf8_lossy(&raw_line_bytes).to_string();
        if self.config.max_line_length > 0 && self.raw_line.len() > self.config.max_line_length {
            self.raw_line = ansi::truncate_str(
                &self.raw_line,
                self.config.max_line_length,
                &self.config.truncation_symbol,
            )
            .to_string()
        };
        self.line = ansi::strip_ansi_codes(&self.raw_line);
    }

    /// Should a handle_* function be called on this element?
    fn should_handle(&self) -> bool {
        let style = self.config.get_style(&self.state);
        !(style.is_raw && style.decoration_style == DecorationStyle::NoDecoration)
    }

    fn handle_commit_meta_header_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::CommitMeta;
        if self.should_handle() {
            self.painter.emit()?;
            self._handle_commit_meta_header_line()?;
            handled_line = true
        }
        Ok(handled_line)
    }

    fn _handle_commit_meta_header_line(&mut self) -> std::io::Result<()> {
        if self.config.commit_style.is_omitted {
            return Ok(());
        }
        let (mut draw_fn, pad, decoration_ansi_term_style) =
            draw::get_draw_function(self.config.commit_style.decoration_style);
        let (formatted_line, formatted_raw_line) = if self.config.hyperlinks {
            (
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    &self.line,
                    self.config,
                ),
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    &self.raw_line,
                    self.config,
                ),
            )
        } else {
            (Cow::from(&self.line), Cow::from(&self.raw_line))
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

    fn handle_diff_stat_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;
        if self.config.relative_paths {
            if let Some(cwd) = self.config.cwd_relative_to_repo_root.as_deref() {
                if let Some(replacement_line) = parse::relativize_path_in_diff_stat_line(
                    &self.raw_line,
                    cwd,
                    self.config.diff_stat_align_width,
                ) {
                    self.painter.emit()?;
                    writeln!(self.painter.writer, "{}", replacement_line)?;
                    handled_line = true
                }
            }
        }
        Ok(handled_line)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn handle_file_meta_diff_line(&mut self) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::FileMeta;
        self.handled_file_meta_header_line_file_pair = None;
        self.diff_line = self.line.clone();
        Ok(false)
    }

    fn handle_file_meta_minus_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;

        let (path_or_mode, file_event) = parse::parse_file_meta_line(
            &self.line,
            self.source == Source::GitDiff,
            if self.config.relative_paths {
                self.config.cwd_relative_to_repo_root.as_deref()
            } else {
                None
            },
        );
        // In the case of ModeChange only, the file path is taken from the diff
        // --git line (since that is the only place the file path occurs);
        // otherwise it is taken from the --- / +++ line.
        self.minus_file = if let parse::FileEvent::ModeChange(_) = &file_event {
            parse::get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or(path_or_mode)
        } else {
            path_or_mode
        };
        self.minus_file_event = file_event;

        if self.source == Source::DiffUnified {
            self.state = State::FileMeta;
            self.painter
                .set_syntax(parse::get_file_extension_from_marker_line(&self.line));
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
            _write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
            handled_line = true;
        }
        Ok(handled_line)
    }

    fn handle_file_meta_plus_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;
        let (path_or_mode, file_event) = parse::parse_file_meta_line(
            &self.line,
            self.source == Source::GitDiff,
            if self.config.relative_paths {
                self.config.cwd_relative_to_repo_root.as_deref()
            } else {
                None
            },
        );
        // In the case of ModeChange only, the file path is taken from the diff
        // --git line (since that is the only place the file path occurs);
        // otherwise it is taken from the --- / +++ line.
        self.plus_file = if let parse::FileEvent::ModeChange(_) = &file_event {
            parse::get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or(path_or_mode)
        } else {
            path_or_mode
        };
        self.plus_file_event = file_event;
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
            _write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
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
            &self.minus_file_event,
            &self.plus_file_event,
            self.config,
        );
        // FIXME: no support for 'raw'
        _write_generic_file_meta_header_line(&line, &line, &mut self.painter, self.config)
    }

    fn handle_additional_file_meta_cases(&mut self) -> std::io::Result<bool> {
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
            _write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
            handled_line = true;
        }

        Ok(handled_line)
    }

    /// Emit the hunk header, with any requested decoration.
    fn handle_hunk_header_line(&mut self) -> std::io::Result<bool> {
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = State::HunkHeader;
        self.painter.set_highlighter();
        self.painter.emit()?;

        let (code_fragment, line_numbers) = parse::parse_hunk_header(&self.line);
        if self.config.line_numbers {
            self.painter
                .line_numbers_data
                .initialize_hunk(&line_numbers, self.plus_file.to_string());
        }

        if self.config.hunk_header_style.is_raw {
            hunk_header::write_hunk_header_raw(
                &mut self.painter,
                &self.line,
                &self.raw_line,
                self.config,
            )?;
        } else if self.config.hunk_header_style.is_omitted {
            writeln!(self.painter.writer)?;
        } else {
            // Add a blank line below the hunk-header-line for readability, unless
            // color_only mode is active.
            if !self.config.color_only {
                writeln!(self.painter.writer)?;
            }

            hunk_header::write_hunk_header(
                &code_fragment,
                &line_numbers,
                &mut self.painter,
                &self.line,
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
    fn handle_hunk_line(&mut self) -> std::io::Result<bool> {
        // Don't let the line buffers become arbitrarily large -- if we
        // were to allow that, then for a large deleted/added file we
        // would process the entire file before painting anything.
        if self.painter.minus_lines.len() > self.config.line_buffer_size
            || self.painter.plus_lines.len() > self.config.line_buffer_size
        {
            self.painter.paint_buffered_minus_and_plus_lines();
        }
        self.state = match self.line.chars().next() {
            Some('-') => {
                if let State::HunkPlus(_) = self.state {
                    self.painter.paint_buffered_minus_and_plus_lines();
                }
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            &self.raw_line,
                            [*style::GIT_DEFAULT_MINUS_STYLE, self.config.git_minus_style].iter(),
                        ) =>
                    {
                        State::HunkMinus(Some(self.painter.prepare_raw_line(&self.raw_line)))
                    }
                    _ => State::HunkMinus(None),
                };
                self.painter
                    .minus_lines
                    .push((self.painter.prepare(&self.line), state.clone()));
                state
            }
            Some('+') => {
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            &self.raw_line,
                            [*style::GIT_DEFAULT_PLUS_STYLE, self.config.git_plus_style].iter(),
                        ) =>
                    {
                        State::HunkPlus(Some(self.painter.prepare_raw_line(&self.raw_line)))
                    }
                    _ => State::HunkPlus(None),
                };
                self.painter
                    .plus_lines
                    .push((self.painter.prepare(&self.line), state.clone()));
                state
            }
            Some(' ') => {
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter.paint_zero_line(&self.line);
                State::HunkZero
            }
            _ => {
                // The first character here could be e.g. '\' from '\ No newline at end of file'. This
                // is not a hunk line, but the parser does not have a more accurate state corresponding
                // to this.
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter
                    .output_buffer
                    .push_str(&self.painter.expand_tabs(self.raw_line.graphemes(true)));
                self.painter.output_buffer.push('\n');
                State::HunkZero
            }
        };
        self.painter.emit()?;
        Ok(true)
    }
}

/// Write `line` with FileMeta styling.
fn _write_generic_file_meta_header_line(
    line: &str,
    raw_line: &str,
    painter: &mut Painter,
    config: &Config,
) -> std::io::Result<()> {
    // If file_style is "omit", we'll skip the process and print nothing.
    // However in the case of color_only mode,
    // we won't skip because we can't change raw_line structure.
    if config.file_style.is_omitted && !config.color_only {
        return Ok(());
    }
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(config.file_style.decoration_style);
    // Prints the new line below file-meta-line.
    // However in the case of color_only mode,
    // we won't print it because we can't change raw_line structure.
    if !config.color_only {
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        &config.decorations_width,
        config.file_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
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
