use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, BufRead, IsTerminal, Write};

use bytelines::ByteLines;

use crate::ansi;
use crate::config::delta_unreachable;
use crate::config::Config;
use crate::config::GrepType;
use crate::features;
use crate::handlers::grep;
use crate::handlers::hunk_header::{AmbiguousDiffMinusCounter, ParsedHunkHeader};
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
    Grep(GrepType, grep::LineType, String, Option<usize>), // In a line of `git grep` output (grep_type, line_type, path, line_number)
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

#[derive(Debug)]
enum GitBinaryPatchState {
    None,
    ExpectFirstHeader,
    FirstPayload,
    AfterFirstFragment,
    PendingSecondHeader {
        line: String,
        raw_line: String,
        parse_line: String,
    },
    SecondPayload,
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
    pub parse_line: String,
    pub state: State,
    pub source: Source,
    pub source_detected_on_current_line: bool,
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
    pub minus_line_counter: AmbiguousDiffMinusCounter,
    git_binary_patch_state: GitBinaryPatchState,
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
            parse_line: "".to_string(),
            state: State::Unknown,
            source: Source::Unknown,
            source_detected_on_current_line: false,
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
            minus_line_counter: AmbiguousDiffMinusCounter::not_needed(),
            git_binary_patch_state: GitBinaryPatchState::None,
        }
    }

    fn consume<I>(&mut self, mut lines: ByteLines<I>) -> std::io::Result<()>
    where
        I: BufRead,
    {
        while let Some(Ok(raw_line_bytes)) = lines.next() {
            self.ingest_line(raw_line_bytes);
            self.detect_source_for_current_line();

            // Every method named handle_* must return std::io::Result<bool>.
            // The bool indicates whether the line has been handled by that
            // method (in which case no subsequent handlers are permitted to
            // handle it).
            let _ = self.handle_git_binary_patch_line()?
                || self.handle_commit_meta_header_line()?
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
                || self.emit_unrecognized_line_unchanged()?;
        }

        self.flush_pending_git_binary_patch_line()?;
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
                self.parse_line = ansi::strip_ansi_codes(&raw_line);
                let truncated_len = utils::round_char_boundary::floor_char_boundary(
                    &raw_line,
                    self.config.max_line_length,
                );
                self.raw_line = raw_line[..truncated_len].to_string();
                self.line.clone_from(&self.raw_line);
            }
        }
    }

    fn detect_source_for_current_line(&mut self) {
        self.source_detected_on_current_line = false;
        if self.source != Source::Unknown {
            return;
        }

        self.source = detect_source(&self.parse_line);
        self.source_detected_on_current_line = self.source != Source::Unknown;
        // Handle (rare) plain `diff -u file1 file2` header. Done here to avoid having
        // to introduce and handle a Source::DiffUnifiedAmbiguous variant everywhere.
        if self.parse_line.starts_with("--- ") {
            self.minus_line_counter = AmbiguousDiffMinusCounter::prepare_to_count();
        }
    }

    fn ingest_line_utf8(&mut self, raw_line: String) {
        self.raw_line = raw_line;
        // When a file has \r\n line endings, git sometimes adds ANSI escape sequences between the
        // \r and \n, in which case byte_lines does not remove the \r. Remove it now. [EndCRLF]
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
        self.parse_line = ansi::strip_ansi_codes(&self.raw_line);
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

    /// Skip recognized file metadata lines unless a raw diff style has been requested.
    pub fn should_skip_line(&self) -> bool {
        matches!(self.state, State::DiffHeader(_))
            && is_recognized_diff_header_line(&self.parse_line, &self.source)
            && self.should_handle()
            && !self.config.color_only
    }

    fn handle_git_binary_patch_line(&mut self) -> std::io::Result<bool> {
        if !matches!(self.state, State::DiffHeader(_))
            || !self.should_handle()
            || self.config.color_only
        {
            self.git_binary_patch_state = GitBinaryPatchState::None;
            return Ok(false);
        }

        let parse_line = self.parse_line.clone();
        let state = std::mem::replace(&mut self.git_binary_patch_state, GitBinaryPatchState::None);
        let handled = match state {
            GitBinaryPatchState::None => {
                if parse_line == "GIT binary patch" {
                    self.git_binary_patch_state = GitBinaryPatchState::ExpectFirstHeader;
                    true
                } else {
                    false
                }
            }
            GitBinaryPatchState::ExpectFirstHeader => {
                if is_git_binary_patch_header(&parse_line) {
                    self.git_binary_patch_state = GitBinaryPatchState::FirstPayload;
                    true
                } else {
                    false
                }
            }
            GitBinaryPatchState::FirstPayload => {
                if is_git_binary_patch_payload(&parse_line) {
                    self.git_binary_patch_state = GitBinaryPatchState::FirstPayload;
                    true
                } else if parse_line.is_empty() {
                    self.git_binary_patch_state = GitBinaryPatchState::AfterFirstFragment;
                    true
                } else {
                    false
                }
            }
            GitBinaryPatchState::AfterFirstFragment => {
                if is_git_binary_patch_header(&parse_line) {
                    self.git_binary_patch_state = GitBinaryPatchState::PendingSecondHeader {
                        line: self.line.clone(),
                        raw_line: self.raw_line.clone(),
                        parse_line,
                    };
                    true
                } else {
                    false
                }
            }
            GitBinaryPatchState::PendingSecondHeader {
                line,
                raw_line,
                parse_line: pending_parse_line,
            } => {
                if is_git_binary_patch_payload(&parse_line) {
                    self.git_binary_patch_state = GitBinaryPatchState::SecondPayload;
                    true
                } else {
                    self.replay_unrecognized_diff_header_line(line, raw_line, pending_parse_line)?;
                    false
                }
            }
            GitBinaryPatchState::SecondPayload => {
                if is_git_binary_patch_payload(&parse_line) {
                    self.git_binary_patch_state = GitBinaryPatchState::SecondPayload;
                    true
                } else {
                    parse_line.is_empty()
                }
            }
        };
        Ok(handled)
    }

    fn flush_pending_git_binary_patch_line(&mut self) -> std::io::Result<()> {
        let state = std::mem::replace(&mut self.git_binary_patch_state, GitBinaryPatchState::None);
        if let GitBinaryPatchState::PendingSecondHeader {
            line,
            raw_line,
            parse_line,
        } = state
        {
            self.replay_unrecognized_diff_header_line(line, raw_line, parse_line)?;
        }
        Ok(())
    }

    fn replay_unrecognized_diff_header_line(
        &mut self,
        line: String,
        raw_line: String,
        parse_line: String,
    ) -> std::io::Result<()> {
        let current_line = std::mem::replace(&mut self.line, line);
        let current_raw_line = std::mem::replace(&mut self.raw_line, raw_line);
        let current_parse_line = std::mem::replace(&mut self.parse_line, parse_line);
        self.emit_unrecognized_line_unchanged()?;
        self.line = current_line;
        self.raw_line = current_raw_line;
        self.parse_line = current_parse_line;
        self.source = Source::Unknown;
        self.detect_source_for_current_line();
        Ok(())
    }

    fn emit_unrecognized_line_unchanged(&mut self) -> std::io::Result<bool> {
        if matches!(self.state, State::DiffHeader(_))
            && self.should_handle()
            && !self.config.color_only
            && !is_recognized_diff_header_line(&self.parse_line, &self.source)
        {
            self.handle_pending_line_with_diff_name()?;
            self.git_binary_patch_state = GitBinaryPatchState::None;
            self.state = State::Unknown;
        }
        self.emit_line_unchanged()
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

fn is_recognized_diff_header_line(line: &str, source: &Source) -> bool {
    line.is_empty()
        || is_diff_header_diff_line_for_source(line, source)
        || is_index_line(line)
        || is_similarity_line(line)
        || is_mode_line(line)
        || is_file_operation_line(line)
        || has_nonempty_suffix(line, "copy from ")
        || has_nonempty_suffix(line, "copy to ")
        || has_nonempty_suffix(line, "rename from ")
        || has_nonempty_suffix(line, "rename to ")
        || has_nonempty_suffix(line, "--- ")
        || has_nonempty_suffix(line, "+++ ")
}

pub(crate) fn is_diff_header_diff_line(line: &str) -> bool {
    is_git_diff_header_line(line) || is_standalone_diff_header_line(line)
}

pub(crate) fn is_diff_header_diff_line_for_source(line: &str, source: &Source) -> bool {
    match source {
        Source::GitDiff => is_git_diff_header_line(line),
        Source::DiffUnified | Source::Unknown => is_diff_header_diff_line(line),
    }
}

fn is_git_diff_header_line(line: &str) -> bool {
    has_diff_paths(line, "diff --git ")
        || has_nonempty_suffix(line, "diff --cc ")
        || has_nonempty_suffix(line, "diff --combined ")
}

fn is_standalone_diff_header_line(line: &str) -> bool {
    line.starts_with("diff -u")
        || line.starts_with("diff -ru")
        || line.starts_with("diff -r -u")
        || line.starts_with("diff -U")
}

fn has_diff_paths(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .is_some_and(|paths| paths.split_whitespace().count() >= 2)
}

pub(crate) fn is_file_mode(mode: &str) -> bool {
    mode.len() == 6 && mode.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

pub(crate) fn is_mode_line(line: &str) -> bool {
    for prefix in ["old mode ", "new mode "] {
        if let Some(mode) = line.strip_prefix(prefix) {
            return is_file_mode(mode);
        }
    }
    let Some(modes) = line.strip_prefix("mode ") else {
        return false;
    };
    let Some((parents, result)) = modes.split_once("..") else {
        return false;
    };
    let mut parent_count = 0;
    let valid_parents = parents.split(',').all(|mode| {
        parent_count += 1;
        is_file_mode(mode)
    });
    valid_parents && parent_count >= 2 && is_file_mode(result)
}

pub(crate) fn is_file_operation_line(line: &str) -> bool {
    line.strip_prefix("deleted file mode ")
        .or_else(|| line.strip_prefix("new file mode "))
        .is_some_and(is_file_mode)
}

pub(crate) fn has_nonempty_suffix(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .is_some_and(|suffix| !suffix.is_empty())
}

fn is_index_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("index ") else {
        return false;
    };
    let mut fields = rest.split_whitespace();
    let Some(hashes) = fields.next() else {
        return false;
    };
    let mode = fields.next();
    if fields.next().is_some() || mode.is_some_and(|mode| !is_file_mode(mode)) {
        return false;
    }
    let Some((parents, result)) = hashes.split_once("..") else {
        return false;
    };
    parents.split(',').all(is_hex_hash) && is_hex_hash(result)
}

fn is_hex_hash(hash: &str) -> bool {
    !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_similarity_line(line: &str) -> bool {
    ["similarity index ", "dissimilarity index "]
        .iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .and_then(|percentage| percentage.strip_suffix('%'))
        .and_then(|percentage| percentage.parse::<u8>().ok())
        .is_some_and(|percentage| percentage <= 100)
}

fn is_git_binary_patch_header(line: &str) -> bool {
    line.strip_prefix("literal ")
        .or_else(|| line.strip_prefix("delta "))
        .is_some_and(|size| !size.is_empty() && size.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_git_binary_patch_payload(line: &str) -> bool {
    const BASE85_ALPHABET: &[u8] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";

    let Some((&length_byte, payload)) = line.as_bytes().split_first() else {
        return false;
    };
    let decoded_length = match length_byte {
        b'A'..=b'Z' => usize::from(length_byte - b'A' + 1),
        b'a'..=b'z' => usize::from(length_byte - b'a' + 27),
        _ => return false,
    };
    let encoded_length = decoded_length.div_ceil(4) * 5;
    payload.len() == encoded_length && payload.iter().all(|byte| BASE85_ALPHABET.contains(byte))
}

/// If output is going to a tty, emit hyperlinks if requested.
// Although raw output should basically be emitted unaltered, we do this.
pub fn format_raw_line<'a>(line: &'a str, config: &Config) -> Cow<'a, str> {
    if config.hyperlinks && io::stdout().is_terminal() {
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
        || (is_diff_header_diff_line(line)
            && (line.starts_with("diff --git ")
                || line.starts_with("diff --cc ")
                || line.starts_with("diff --combined ")))
    {
        Source::GitDiff
    } else if (is_diff_header_diff_line(line) && line.starts_with("diff -"))
        || line.starts_with("--- ")
        || line.starts_with("Only in ")
    {
        Source::DiffUnified
    } else {
        Source::Unknown
    }
}
