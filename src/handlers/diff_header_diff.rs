use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};
use crate::handlers::diff_header::{get_repeated_file_path_from_diff_line, FileEvent};

impl StateMachine<'_> {
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
        self.painter.emit()?;
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
        self.diff_line.clone_from(&self.line);

        // Pre-fill header fields from the diff line. For added, removed or renamed files
        // these are updated precisely on actual header minus and header plus lines.
        // But for modified binary files which are not added, removed or renamed, there
        // are no minus and plus lines. Without the code below, in such cases the file names
        // would remain unchanged from the previous diff, or empty for the very first diff.
        let name = get_repeated_file_path_from_diff_line(&self.line).unwrap_or_default();
        self.minus_file.clone_from(&name);
        self.plus_file.clone_from(&name);
        self.minus_file_event = FileEvent::Change;
        self.plus_file_event = FileEvent::Change;
        self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));

        if !self.should_skip_line() {
            self.emit_line_unchanged()?;
        }
        Ok(true)
    }
}
