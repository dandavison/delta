use crate::delta::{Source, State, StateMachine};

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_file_meta_misc_cases(&self) -> bool {
        self.source == Source::DiffUnified && self.line.starts_with("Only in ")
            || self.line.starts_with("Binary files ")
    }

    pub fn handle_file_meta_misc_lines(&mut self) -> std::io::Result<bool> {
        if !self.test_file_meta_misc_cases() {
            return Ok(false);
        }
        self.handle_additional_cases(State::FileMeta)
    }
}
