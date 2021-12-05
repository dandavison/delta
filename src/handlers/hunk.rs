use std::cmp::min;

use lazy_static::lazy_static;

use crate::cli;
use crate::config::delta_unreachable;
use crate::delta::{DiffType, State, StateMachine};
use crate::style;
use crate::utils::process::{self, CallingProcess};
use unicode_segmentation::UnicodeSegmentation;

lazy_static! {
    static ref IS_WORD_DIFF: bool = match process::calling_process().as_deref() {
        Some(
            CallingProcess::GitDiff(cmd_line)
            | CallingProcess::GitShow(cmd_line, _)
            | CallingProcess::GitLog(cmd_line)
            | CallingProcess::GitReflog(cmd_line),
        ) =>
            cmd_line.long_options.contains("--word-diff")
                || cmd_line.long_options.contains("--word-diff-regex")
                || cmd_line.long_options.contains("--color-words"),
        _ => false,
    };
}

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_hunk_line(&self) -> bool {
        matches!(
            self.state,
            State::HunkHeader(_, _, _)
                | State::HunkZero(_)
                | State::HunkMinus(_, _)
                | State::HunkPlus(_, _)
        ) && !&*IS_WORD_DIFF
    }

    /// Handle a hunk line, i.e. a minus line, a plus line, or an unchanged line.
    // In the case of a minus or plus line, we store the line in a
    // buffer. When we exit the changed region we process the collected
    // minus and plus lines jointly, in order to paint detailed
    // highlighting according to inferred edit operations. In the case of
    // an unchanged line, we paint it immediately.
    pub fn handle_hunk_line(&mut self) -> std::io::Result<bool> {
        use State::*;
        // A true hunk line should start with one of: '+', '-', ' '. However, handle_hunk_line
        // handles all lines until the state transitions away from the hunk states.
        if !self.test_hunk_line() {
            return Ok(false);
        }
        // Don't let the line buffers become arbitrarily large -- if we
        // were to allow that, then for a large deleted/added file we
        // would process the entire file before painting anything.
        if self.painter.minus_lines.len() > self.config.line_buffer_size
            || self.painter.plus_lines.len() > self.config.line_buffer_size
        {
            self.painter.paint_buffered_minus_and_plus_lines();
        }
        if let State::HunkHeader(_, line, raw_line) = &self.state.clone() {
            self.emit_hunk_header_line(line, raw_line)?;
        }
        self.state = match new_line_state(&self.line, &self.state) {
            Some(HunkMinus(prefix, _)) => {
                if let HunkPlus(_, _) = self.state {
                    // We have just entered a new subhunk; process the previous one
                    // and flush the line buffers.
                    self.painter.paint_buffered_minus_and_plus_lines();
                }
                let line = self.painter.prepare(&self.line, prefix.as_deref());
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            &self.raw_line,
                            [*style::GIT_DEFAULT_MINUS_STYLE, self.config.git_minus_style].iter(),
                        ) =>
                    {
                        let raw_line = self
                            .painter
                            .prepare_raw_line(&self.raw_line, prefix.as_deref());
                        HunkMinus(prefix, Some(raw_line))
                    }
                    _ => HunkMinus(prefix, None),
                };
                self.painter.minus_lines.push((line, state.clone()));
                state
            }
            Some(HunkPlus(prefix, _)) => {
                let line = self.painter.prepare(&self.line, prefix.as_deref());
                let state = match self.config.inspect_raw_lines {
                    cli::InspectRawLines::True
                        if style::line_has_style_other_than(
                            &self.raw_line,
                            [*style::GIT_DEFAULT_PLUS_STYLE, self.config.git_plus_style].iter(),
                        ) =>
                    {
                        let raw_line = self
                            .painter
                            .prepare_raw_line(&self.raw_line, prefix.as_deref());
                        HunkPlus(prefix, Some(raw_line))
                    }
                    _ => HunkPlus(prefix, None),
                };
                self.painter.plus_lines.push((line, state.clone()));
                state
            }
            Some(HunkZero(prefix)) => {
                // We are in a zero (unchanged) line, therefore we have just exited a subhunk (a
                // sequence of consecutive minus (removed) and/or plus (added) lines). Process that
                // subhunk and flush the line buffers.
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter.paint_zero_line(&self.line, prefix.clone());
                HunkZero(prefix)
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
                State::HunkZero(None)
            }
        };
        self.painter.emit()?;
        Ok(true)
    }
}

fn new_line_state(new_line: &str, prev_state: &State) -> Option<State> {
    use State::*;
    let diff_type = match prev_state {
        HunkMinus(None, _) | HunkZero(None) | HunkPlus(None, _) => DiffType::Unified,
        HunkHeader(diff_type, _, _) => diff_type.clone(),
        HunkMinus(Some(prefix), _) | HunkZero(Some(prefix)) | HunkPlus(Some(prefix), _) => {
            DiffType::Combined(prefix.len())
        }
        _ => delta_unreachable(&format!("diff_type: unexpected state: {:?}", prev_state)),
    };

    let (prefix_char, prefix) = match diff_type {
        DiffType::Unified => (new_line.chars().next(), None),
        DiffType::Combined(n_parents) => {
            let prefix = &new_line[..min(n_parents, new_line.len())];
            let prefix_char = match prefix.chars().find(|c| c == &'-' || c == &'+') {
                Some(c) => Some(c),
                None => match prefix.chars().find(|c| c != &' ') {
                    None => Some(' '),
                    Some(_) => None,
                },
            };
            (prefix_char, Some(prefix.to_string()))
        }
    };
    match prefix_char {
        Some('-') => Some(HunkMinus(prefix, None)),
        Some(' ') => Some(HunkZero(prefix)),
        Some('+') => Some(HunkPlus(prefix, None)),
        _ => None,
    }
}
