use crate::delta::{
    is_diff_header_diff_line_for_source, DiffType, InMergeConflict, MergeParents, Source, State,
    StateMachine,
};
use crate::handlers::diff_header::{get_repeated_file_path_from_diff_line, FileEvent};

impl StateMachine<'_> {
    #[inline]
    fn test_diff_header_diff_line(&self) -> bool {
        is_diff_header_diff_line_for_source(&self.parse_line, &self.source)
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn handle_diff_header_diff_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_diff_line() {
            if self.source == Source::GitDiff
                && self.parse_line.starts_with("diff ")
                && matches!(
                    self.state,
                    State::HunkHeader(_, _, _, _)
                        | State::HunkMinus(_, _)
                        | State::HunkPlus(_, _)
                        | State::HunkZero(_, _)
                )
            {
                if let State::HunkHeader(_, parsed_hunk_header, line, raw_line) =
                    &self.state.clone()
                {
                    self.emit_hunk_header_line(parsed_hunk_header, line, raw_line)?;
                }
                self.painter.paint_buffered_minus_and_plus_lines();
                self.painter.emit()?;
                self.state = State::Unknown;
                self.source = Source::Unknown;
                self.emit_line_unchanged()?;
                return Ok(true);
            }
            return Ok(false);
        }
        self.painter.paint_buffered_minus_and_plus_lines();
        self.painter.emit()?;
        self.source = if self.parse_line.starts_with("diff --git ")
            || self.parse_line.starts_with("diff --cc ")
            || self.parse_line.starts_with("diff --combined ")
        {
            Source::GitDiff
        } else {
            Source::DiffUnified
        };
        self.state = if self.parse_line.starts_with("diff --cc ")
            || self.parse_line.starts_with("diff --combined ")
        {
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
        self.diff_line.clone_from(&self.parse_line);

        // Pre-fill header fields from the diff line. For added, removed or renamed files
        // these are updated precisely on actual header minus and header plus lines.
        // But for modified binary files which are not added, removed or renamed, there
        // are no minus and plus lines. Without the code below, in such cases the file names
        // would remain unchanged from the previous diff, or empty for the very first diff.
        let name = get_repeated_file_path_from_diff_line(&self.parse_line).unwrap_or_default();
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
