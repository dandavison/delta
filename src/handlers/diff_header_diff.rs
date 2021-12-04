use crate::delta::{State, StateMachine};

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
        self.state = State::DiffHeader;
        self.handled_diff_header_header_line_file_pair = None;
        self.diff_line = self.line.clone();
        if !self.should_skip_line() {
            self.emit_line_unchanged()?;
        }
        Ok(true)
    }
}
