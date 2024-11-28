use crate::delta::{DiffType, Source, State, StateMachine};
use crate::utils::path::relativize_path_maybe;

impl StateMachine<'_> {
    #[inline]
    fn test_diff_file_missing(&self) -> bool {
        self.source == Source::DiffUnified && self.line.starts_with("Only in ")
    }

    #[inline]
    fn test_diff_is_binary(&self) -> bool {
        self.line.starts_with("Binary files ")
    }

    pub fn handle_diff_header_misc_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_file_missing() && !self.test_diff_is_binary() {
            return Ok(false);
        }

        // Preserve the "Binary files" line when diff lines should be kept unchanged.
        if !self.config.color_only && self.test_diff_is_binary() {
            // Print the "Binary files" line verbatim, if there was no "diff" line, or it
            // listed different files but was not followed by header minus and plus lines.
            // This can happen in output of standalone diff or git diff --no-index.
            if self.minus_file.is_empty() && self.plus_file.is_empty() {
                self.emit_line_unchanged()?;
                self.handled_diff_header_header_line_file_pair
                    .clone_from(&self.current_file_pair);
                return Ok(true);
            }

            if self.minus_file != "/dev/null" {
                relativize_path_maybe(&mut self.minus_file, self.config);
                self.minus_file.push_str(" (binary file)");
            }
            if self.plus_file != "/dev/null" {
                relativize_path_maybe(&mut self.plus_file, self.config);
                self.plus_file.push_str(" (binary file)");
            }
            return Ok(true);
        }

        self.handle_additional_cases(match self.state {
            State::DiffHeader(_) => self.state.clone(),
            _ => State::DiffHeader(DiffType::Unified),
        })
    }
}
