/// This module contains functions handling input lines encountered during the
/// main `StateMachine::consume()` loop.
pub mod commit_meta;
pub mod diff_stat;
pub mod draw;
pub mod file_meta;
pub mod file_meta_diff;
pub mod file_meta_misc;
pub mod hunk;
pub mod hunk_header;
pub mod submodule;

use crate::delta::{State, StateMachine};

impl<'a> StateMachine<'a> {
    pub fn handle_additional_cases(&mut self, to_state: State) -> std::io::Result<bool> {
        let mut handled_line = false;

        // Additional cases:
        //
        // 1. When comparing directories with diff -u, if filenames match between the
        //    directories, the files themselves will be compared. However, if an equivalent
        //    filename is not present, diff outputs a single line (Only in...) starting
        //    indicating that the file is present in only one of the directories.
        //
        // 2. Git diff emits lines describing submodule state such as "Submodule x/y/z contains
        //    untracked content"
        //
        // See https://github.com/dandavison/delta/issues/60#issuecomment-557485242 for a
        // proposal for more robust parsing logic.

        self.painter.paint_buffered_minus_and_plus_lines();
        self.state = to_state;
        if self.should_handle() {
            self.painter.emit()?;
            file_meta::write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
            handled_line = true;
        }

        Ok(handled_line)
    }
}
