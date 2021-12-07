use std::cmp::min;

use lazy_static::lazy_static;

use crate::cli;
use crate::config::delta_unreachable;
use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};
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
            State::HunkHeader(_, _, _, _)
                | State::HunkZero(_, _)
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
        use DiffType::*;
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
        if let State::HunkHeader(_, parsed_hunk_header, line, raw_line) = &self.state.clone() {
            self.emit_hunk_header_line(parsed_hunk_header, line, raw_line)?;
        }
        self.state = match new_line_state(&self.line, &self.state) {
            Some(HunkMinus(diff_type, _)) => {
                if let HunkPlus(_, _) = self.state {
                    // We have just entered a new subhunk; process the previous one
                    // and flush the line buffers.
                    self.painter.paint_buffered_minus_and_plus_lines();
                }
                let n_parents = diff_type.n_parents();
                let line = self.painter.prepare(&self.line, n_parents);
                let raw_line = self.maybe_raw_line(
                    n_parents,
                    &[*style::GIT_DEFAULT_MINUS_STYLE, self.config.git_minus_style],
                );
                let state = HunkMinus(diff_type, raw_line);
                self.painter.minus_lines.push((line, state.clone()));
                state
            }
            Some(HunkPlus(diff_type, _)) => {
                let n_parents = diff_type.n_parents();
                let line = self.painter.prepare(&self.line, n_parents);
                let raw_line = self.maybe_raw_line(
                    n_parents,
                    &[*style::GIT_DEFAULT_PLUS_STYLE, self.config.git_plus_style],
                );
                let state = HunkPlus(diff_type, raw_line);
                self.painter.plus_lines.push((line, state.clone()));
                state
            }
            Some(HunkZero(diff_type, _)) => {
                // We are in a zero (unchanged) line, therefore we have just exited a subhunk (a
                // sequence of consecutive minus (removed) and/or plus (added) lines). Process that
                // subhunk and flush the line buffers.
                self.painter.paint_buffered_minus_and_plus_lines();
                let n_parents = diff_type.n_parents();
                let line = self.painter.prepare(&self.line, n_parents);
                let raw_line = self.maybe_raw_line(n_parents, &[]);
                let state = State::HunkZero(diff_type, raw_line);
                self.painter.paint_zero_line(&line, state.clone());
                state
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
                State::HunkZero(Unified, None)
            }
        };
        self.painter.emit()?;
        Ok(true)
    }

    fn maybe_raw_line(&self, n_parents: usize, non_raw_styles: &[style::Style]) -> Option<String> {
        let emit_raw_line = self.config.inspect_raw_lines == cli::InspectRawLines::True
            && style::line_has_style_other_than(&self.raw_line, non_raw_styles);
        if emit_raw_line {
            Some(self.painter.prepare_raw_line(&self.raw_line, n_parents))
        } else {
            None
        }
    }
}

// Return the new state corresponding to `new_line`, given the previous state. A return value of
// None means that `new_line` is not recognized as a hunk line.
fn new_line_state(new_line: &str, prev_state: &State) -> Option<State> {
    use DiffType::*;
    use MergeParents::*;
    use State::*;

    let diff_type = match prev_state {
        HunkMinus(Unified, _)
        | HunkZero(Unified, _)
        | HunkPlus(Unified, _)
        | HunkHeader(Unified, _, _, _) => Unified,
        HunkHeader(Combined(Number(n), InMergeConflict::No), _, _, _) => {
            Combined(Number(*n), InMergeConflict::No)
        }
        // The prefixes are specific to the previous line, but the number of merge parents remains
        // equal to the prefix length.
        HunkHeader(Combined(Prefix(prefix), InMergeConflict::No), _, _, _) => {
            Combined(Number(prefix.len()), InMergeConflict::No)
        }
        HunkMinus(Combined(Prefix(prefix), in_merge_conflict), _)
        | HunkZero(Combined(Prefix(prefix), in_merge_conflict), _)
        | HunkPlus(Combined(Prefix(prefix), in_merge_conflict), _) => {
            Combined(Number(prefix.len()), in_merge_conflict.clone())
        }
        HunkMinus(Combined(Number(n), in_merge_conflict), _)
        | HunkZero(Combined(Number(n), in_merge_conflict), _)
        | HunkPlus(Combined(Number(n), in_merge_conflict), _) => {
            Combined(Number(*n), in_merge_conflict.clone())
        }
        _ => delta_unreachable(&format!(
            "Unexpected state in new_line_state: {:?}",
            prev_state
        )),
    };

    let (prefix_char, prefix, in_merge_conflict) = match diff_type {
        Unified => (new_line.chars().next(), None, None),
        Combined(Number(n_parents), in_merge_conflict) => {
            let prefix = &new_line[..min(n_parents, new_line.len())];
            let prefix_char = match prefix.chars().find(|c| c == &'-' || c == &'+') {
                Some(c) => Some(c),
                None => match prefix.chars().find(|c| c != &' ') {
                    None => Some(' '),
                    Some(_) => None,
                },
            };
            (
                prefix_char,
                Some(prefix.to_string()),
                Some(in_merge_conflict),
            )
        }
        _ => delta_unreachable(""),
    };

    match (prefix_char, prefix, in_merge_conflict) {
        (Some('-'), None, None) => Some(HunkMinus(Unified, None)),
        (Some(' '), None, None) => Some(HunkZero(Unified, None)),
        (Some('+'), None, None) => Some(HunkPlus(Unified, None)),
        (Some('-'), Some(prefix), Some(in_merge_conflict)) => {
            Some(HunkMinus(Combined(Prefix(prefix), in_merge_conflict), None))
        }
        (Some(' '), Some(prefix), Some(in_merge_conflict)) => {
            Some(HunkZero(Combined(Prefix(prefix), in_merge_conflict), None))
        }
        (Some('+'), Some(prefix), Some(in_merge_conflict)) => {
            Some(HunkPlus(Combined(Prefix(prefix), in_merge_conflict), None))
        }
        _ => None,
    }
}
