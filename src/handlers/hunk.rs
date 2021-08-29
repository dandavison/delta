use crate::cli;
use crate::delta::{State, StateMachine};
use crate::style;
use unicode_segmentation::UnicodeSegmentation;

impl State {
    fn is_in_hunk(&self) -> bool {
        matches!(
            *self,
            State::HunkHeader(_, _) | State::HunkZero | State::HunkMinus(_) | State::HunkPlus(_)
        )
    }
}

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_hunk_line(&self) -> bool {
        self.state.is_in_hunk()
    }

    /// Handle a hunk line, i.e. a minus line, a plus line, or an unchanged line.
    // In the case of a minus or plus line, we store the line in a
    // buffer. When we exit the changed region we process the collected
    // minus and plus lines jointly, in order to paint detailed
    // highlighting according to inferred edit operations. In the case of
    // an unchanged line, we paint it immediately.
    pub fn handle_hunk_line(&mut self) -> std::io::Result<bool> {
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
        if let State::HunkHeader(line, raw_line) = &self.state.clone() {
            self.emit_hunk_header_line(line, raw_line)?;
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
