use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};

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
                // We will determine the number of parents when we see the hunk header.
                State::DiffHeader(DiffType::Combined(
                    MergeParents::Unknown,
                    InMergeConflict::No,
                ))
            } else {
                State::DiffHeader(DiffType::Unified)
            };
        self.handle_pending_line_with_diff_name()?;
        self.handled_diff_header_header_line_file_pair = None;
        self.diff_line = self.line.clone();
        if !self.should_skip_line() {
            self.emit_line_unchanged()?;
        }
        Ok(true)
    }
}
