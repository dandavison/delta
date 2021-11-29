use crate::delta::{DiffType, State, StateMachine};

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_diff_header_diff_line(&self) -> bool {
        self.line.starts_with("diff ")
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn handle_diff_header_diff_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_diff_line() {
            return Ok(false);
        }
        self.painter.paint_buffered_minus_and_plus_lines();
        self.state =
            if self.line.starts_with("diff --cc ") || self.line.starts_with("diff --combined ") {
                State::DiffHeader(DiffType::Combined(2)) // We will confirm the number of parents when we see the hunk header
            } else {
                State::DiffHeader(DiffType::Unified)
            };
        self.handled_diff_header_header_line_file_pair = None;
        self.diff_line = self.line.clone();
        Ok(false)
    }
}
