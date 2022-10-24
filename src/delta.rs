use std::borrow::Cow;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Write;

use bytelines::ByteLines;

use crate::ansi;
use crate::config::delta_unreachable;
use crate::config::Config;
use crate::features;
use crate::handlers::hunk_header::ParsedHunkHeader;
use crate::handlers::{self, merge_conflict};
use crate::paint::Painter;
use crate::style::DecorationStyle;
use crate::utils;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    CommitMeta,                                             // In commit metadata section
    DiffHeader(DiffType), // In diff metadata section, between (possible) commit metadata and first hunk
    HunkHeader(DiffType, ParsedHunkHeader, String, String), // In hunk metadata line (diff_type, parsed, line, raw_line)
    HunkZero(DiffType, Option<String>), // In hunk; unchanged line (prefix, raw_line)
    HunkMinus(DiffType, Option<String>), // In hunk; removed line (diff_type, raw_line)
    HunkPlus(DiffType, Option<String>), // In hunk; added line (diff_type, raw_line)
    MergeConflict(MergeParents, merge_conflict::MergeConflictCommit),
    SubmoduleLog, // In a submodule section, with gitconfig diff.submodule = log
    SubmoduleShort(String), // In a submodule section, with gitconfig diff.submodule = short
    Blame(String), // In a line of `git blame` output (key).
    GitShowFile,  // In a line of `git show $revision:./path/to/file.ext` output
    Grep,         // In a line of `git grep` output
    Unknown,
    // The following elements are created when a line is wrapped to display it:
    HunkZeroWrapped,  // Wrapped unchanged line
    HunkMinusWrapped, // Wrapped removed line
    HunkPlusWrapped,  // Wrapped added line
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffType {
    Unified,
    // https://git-scm.com/docs/git-diff#_combined_diff_format
    Combined(MergeParents, InMergeConflict),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeParents {
    Number(usize),  // Number of parent commits == (number of @s in hunk header) - 1
    Prefix(String), // Hunk line prefix, length == number of parent commits
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InMergeConflict {
    Yes,
    No,
}

impl DiffType {
    pub fn n_parents(&self) -> usize {
        use DiffType::*;
        use MergeParents::*;
        match self {
            Combined(Prefix(prefix), _) => prefix.len(),
            Combined(Number(n_parents), _) => *n_parents,
            Unified => 1,
            Combined(Unknown, _) => delta_unreachable("Number of merge parents must be known."),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    GitDiff,     // Coming from a `git diff` command
    DiffUnified, // Coming from a `diff -u` command
    Unknown,
}

// Possible transitions, with actions on entry:
//
//
// | from \ to   | CommitMeta  | DiffHeader  | HunkHeader  | HunkZero    | HunkMinus   | HunkPlus |
// |-------------+-------------+-------------+-------------+-------------+-------------+----------|
// | CommitMeta  | emit        | emit        |             |             |             |          |
// | DiffHeader  |             | emit        | emit        |             |             |          |
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
    pub minus_file_event: handlers::diff_header::FileEvent,
    pub plus_file_event: handlers::diff_header::FileEvent,
    pub diff_line: String,
    pub mode_info: String,
    pub painter: Painter<'a>,
    pub config: &'a Config,

    // When a file is modified, we use lines starting with '---' or '+++' to obtain the file name.
    // When a file is renamed without changes, we use lines starting with 'rename' to obtain the
    // file name (there is no diff hunk and hence no lines starting with '---' or '+++'). But when
    // a file is renamed with changes, both are present, and we rely on the following variables to
    // avoid emitting the file meta header line twice (#245).
    pub current_file_pair: Option<(String, String)>,
    pub handled_diff_header_header_line_file_pair: Option<(String, String)>,
    pub blame_key_colors: HashMap<String, String>,
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
            minus_file_event: handlers::diff_header::FileEvent::NoEvent,
            plus_file_event: handlers::diff_header::FileEvent::NoEvent,
            diff_line: "".to_string(),
            mode_info: "".to_string(),
            current_file_pair: None,
            handled_diff_header_header_line_file_pair: None,
            painter: Painter::new(writer, config),
            config,
            blame_key_colors: HashMap::new(),
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

            // Every method named handle_* must return std::io::Result<bool>.
            // The bool indicates whether the line has been handled by that
            // method (in which case no subsequent handlers are permitted to
            // handle it).
            let _ = self.handle_commit_meta_header_line()?
                || self.handle_diff_stat_line()?
                || self.handle_diff_header_diff_line()?
                || self.handle_diff_header_file_operation_line()?
                || self.handle_diff_header_minus_line()?
                || self.handle_diff_header_plus_line()?
                || self.handle_hunk_header_line()?
                || self.handle_diff_header_mode_line()?
                || self.handle_diff_header_misc_line()?
                || self.handle_submodule_log_line()?
                || self.handle_submodule_short_line()?
                || self.handle_merge_conflict_line()?
                || self.handle_hunk_line()?
                || self.handle_git_show_file_line()?
                || self.handle_blame_line()?
                || self.handle_grep_line()?
                || self.should_skip_line()
                || self.emit_line_unchanged()?;
        }

        self.handle_pending_line_with_diff_name()?;
        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.emit()?;
        Ok(())
    }

    fn ingest_line(&mut self, raw_line_bytes: &[u8]) {
        match String::from_utf8(raw_line_bytes.to_vec()) {
            Ok(utf8) => self.ingest_line_utf8(utf8),
            Err(_) => {
                let raw_line = String::from_utf8_lossy(raw_line_bytes);
                let truncated_len = utils::round_char_boundary::floor_char_boundary(
                    &raw_line,
                    self.config.max_line_length,
                );
                self.raw_line = raw_line[..truncated_len].to_string();
                self.line = self.raw_line.clone();
            }
        }
    }

    fn ingest_line_utf8(&mut self, raw_line: String) {
        self.raw_line = raw_line;
        // When a file has \r\n line endings, git sometimes adds ANSI escape sequences between the
        // \r and \n, in which case byte_lines does not remove the \r. Remove it now.
        // TODO: Limit the number of characters we examine when looking for the \r?
        if let Some(cr_index) = self.raw_line.rfind('\r') {
            if ansi::measure_text_width(&self.raw_line[cr_index + 1..]) == 0 {
                self.raw_line = format!(
                    "{}{}",
                    &self.raw_line[..cr_index],
                    &self.raw_line[cr_index + 1..]
                );
            }
        }
        if self.config.max_line_length > 0
            && self.raw_line.len() > self.config.max_line_length
            // Do not truncate long hunk headers
            && !self.raw_line.starts_with("@@")
            // Do not truncate ripgrep --json output
            && !self.raw_line.starts_with('{')
        {
            self.raw_line = ansi::truncate_str(
                &self.raw_line,
                self.config.max_line_length,
                &self.config.truncation_symbol,
            )
            .to_string()
        };
        self.line = ansi::strip_ansi_codes(&self.raw_line);
    }

    /// Skip file metadata lines unless a raw diff style has been requested.
    pub fn should_skip_line(&self) -> bool {
        matches!(self.state, State::DiffHeader(_))
            && self.should_handle()
            && !self.config.color_only
    }

    /// Emit unchanged any line that delta does not handle.
    pub fn emit_line_unchanged(&mut self) -> std::io::Result<bool> {
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
    if line.starts_with("commit ")
        || line.starts_with("diff --git ")
        || line.starts_with("diff --cc ")
        || line.starts_with("diff --combined ")
    {
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
