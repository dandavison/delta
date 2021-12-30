/// This module contains functions handling input lines encountered during the
/// main `StateMachine::consume()` loop.
pub mod blame;
pub mod commit_meta;
pub mod diff_header;
pub mod diff_header_diff;
pub mod diff_header_misc;
pub mod diff_stat;
pub mod draw;
pub mod git_show_file;
pub mod grep;
pub mod hunk;
pub mod hunk_header;
pub mod merge_conflict;
mod ripgrep_json;
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
            diff_header::write_generic_diff_header_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                &mut self.mode_info,
                self.config,
            )?;
            handled_line = true;
        }

        Ok(handled_line)
    }
}
