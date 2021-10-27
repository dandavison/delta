use std::borrow::Cow;
use std::io::BufRead;
use std::io::Write;

use bytelines::ByteLines;

use crate::ansi;
use crate::config::Config;
use crate::features;
use crate::handlers;
use crate::paint::Painter;
use crate::style::DecorationStyle;

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    CommitMeta,                 // In commit metadata section
    FileMeta, // In diff metadata section, between (possible) commit metadata and first hunk
    HunkHeader(String, String), // In hunk metadata line (line, raw_line)
    HunkZero, // In hunk; unchanged line
    HunkMinus(Option<String>), // In hunk; removed line (raw_line)
    HunkPlus(Option<String>), // In hunk; added line (raw_line)
    SubmoduleLog, // In a submodule section, with gitconfig diff.submodule = log
    SubmoduleShort(String), // In a submodule section, with gitconfig diff.submodule = short
    Unknown,
    // The following elements are created when a line is wrapped to display it:
    HunkZeroWrapped,  // Wrapped unchanged line
    HunkMinusWrapped, // Wrapped removed line
    HunkPlusWrapped,  // Wrapped added line
}

#[derive(Debug, PartialEq)]
pub enum Source {
    GitDiff,     // Coming from a `git diff` command
    DiffUnified, // Coming from a `diff -u` command
    Unknown,
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

pub struct StateMachine<'a> {
    pub line: String,
    pub raw_line: String,
    pub state: State,
    pub source: Source,
    pub minus_file: String,
    pub plus_file: String,
    pub minus_file_event: handlers::file_meta::FileEvent,
    pub plus_file_event: handlers::file_meta::FileEvent,
    pub diff_line: String,
    pub painter: Painter<'a>,
    pub config: &'a Config,

    // When a file is modified, we use lines starting with '---' or '+++' to obtain the file name.
    // When a file is renamed without changes, we use lines starting with 'rename' to obtain the
    // file name (there is no diff hunk and hence no lines starting with '---' or '+++'). But when
    // a file is renamed with changes, both are present, and we rely on the following variables to
    // avoid emitting the file meta header line twice (#245).
    pub current_file_pair: Option<(String, String)>,
    pub handled_file_meta_header_line_file_pair: Option<(String, String)>,
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
            minus_file_event: handlers::file_meta::FileEvent::NoEvent,
            plus_file_event: handlers::file_meta::FileEvent::NoEvent,
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

            if self.source == Source::Unknown {
                self.source = detect_source(&self.line);
            }

            let _ = self.handle_commit_meta_header_line()?
                || self.handle_diff_stat_line()?
                || self.handle_file_meta_diff_line()?
                || self.handle_file_meta_minus_line()?
                || self.handle_file_meta_plus_line()?
                || self.handle_hunk_header_line()?
                || self.handle_file_meta_misc_line()?
                || self.handle_submodule_log_line()?
                || self.handle_submodule_short_line()?
                || self.handle_hunk_line()?
                || self.should_skip_line()
                || self.emit_line_unchanged()?;
        }

        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.emit()?;
        Ok(())
    }

    fn ingest_line(&mut self, raw_line_bytes: &[u8]) {
        // TODO: retain raw_line as Cow
        self.raw_line = String::from_utf8_lossy(raw_line_bytes).to_string();
        if self.config.max_line_length > 0 && self.raw_line.len() > self.config.max_line_length {
            self.raw_line = ansi::truncate_str(
                &self.raw_line,
                self.config.max_line_length,
                &self.config.truncation_symbol,
            )
            .to_string()
        };
        self.line = ansi::strip_ansi_codes(&self.raw_line);

        // Strip the neglected CR.
        // (CR-LF is unfortunately split by git because it adds ansi escapes between them.
        //  Thus byte_lines library can't remove the CR properly.)
        if let Some(b'\r') = self.line.bytes().nth_back(0) {
            self.line.truncate(self.line.len() - 1);
        }
    }

    /// Skip file metadata lines unless a raw diff style has been requested.
    fn should_skip_line(&self) -> bool {
        self.state == State::FileMeta && self.should_handle() && !self.config.color_only
    }

    /// Emit unchanged any line that delta does not handle.
    fn emit_line_unchanged(&mut self) -> std::io::Result<bool> {
        self.painter.emit()?;
        writeln!(
            self.painter.writer,
            "{}",
            format_raw_line(&self.raw_line, self.config)
        )?;
        let handled_line = true;
        Ok(handled_line)
    }

    /// Should a handle_* function be called on this element?
    // TODO: I'm not sure the above description is accurate; I think this
    // function needs a more accurate name.
    pub fn should_handle(&self) -> bool {
        let style = self.config.get_style(&self.state);
        !(style.is_raw && style.decoration_style == DecorationStyle::NoDecoration)
    }
}

/// If output is going to a tty, emit hyperlinks if requested.
// Although raw output should basically be emitted unaltered, we do this.
pub fn format_raw_line<'a>(line: &'a str, config: &Config) -> Cow<'a, str> {
    if config.hyperlinks && atty::is(atty::Stream::Stdout) {
        features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(line, config)
    } else {
        Cow::from(line)
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
