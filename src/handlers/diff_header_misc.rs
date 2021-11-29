use crate::delta::{DiffType, Source, State, StateMachine};

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_diff_header_misc_cases(&self) -> bool {
        self.source == Source::DiffUnified && self.line.starts_with("Only in ")
            || self.line.starts_with("Binary files ")
    }

    pub fn handle_diff_header_misc_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_misc_cases() {
            return Ok(false);
        }
        self.handle_additional_cases(match self.state {
            State::DiffHeader(_) => self.state.clone(),
            _ => State::DiffHeader(DiffType::Unified),
        })
    }
}
