use crate::delta::{DiffType, Source, State, StateMachine};
use crate::utils::path::relativize_path_maybe;

/// Display decoration appended in place to a binary file's `minus_file`/
/// `plus_file`; `diff_header_osc` strips it again so the metadata record
/// carries the real path.
pub const BINARY_FILE_SUFFIX: &str = " (binary file)";

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
                self.minus_file.push_str(BINARY_FILE_SUFFIX);
            }
            if self.plus_file != "/dev/null" {
                relativize_path_maybe(&mut self.plus_file, self.config);
                self.plus_file.push_str(BINARY_FILE_SUFFIX);
            }
            return Ok(true);
        }

        // A "Binary files ..." row in a git diff is about the current file:
        // the paths are fresh, (re-)set at the `diff --git` line just above.
        // An "Only in dir: file" row names a file delta never parses, so it
        // carries no record -- and in this format (plain `diff -ur`; git
        // never emits it) a binary row's paths may be the previous file's,
        // since no `diff --git` line resets them between sections.
        let osc = if self.test_diff_is_binary() && self.source == Source::GitDiff {
            self.diff_header_osc()
        } else {
            String::new()
        };
        self.handle_additional_cases(
            match self.state {
                State::DiffHeader(_) => self.state.clone(),
                _ => State::DiffHeader(DiffType::Unified),
            },
            osc,
        )
    }
}
