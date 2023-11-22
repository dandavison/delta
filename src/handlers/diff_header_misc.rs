use crate::delta::{DiffType, Source, State, StateMachine};

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_diff_file_missing(&self) -> bool {
        self.source == Source::DiffUnified && self.line.starts_with("Only in ")
    }

    #[inline]
    fn test_diff_is_binary(&self) -> bool {
        self.line.starts_with("Binary files ")
    }

    pub fn handle_diff_header_misc_line(&mut self) -> std::io::Result<bool> {
        let is_binary: bool = self.test_diff_is_binary();
        let file_missing: bool = self.test_diff_file_missing();

        if !file_missing && !is_binary {
            return Ok(false);
        }

        if is_binary {
            match (self.minus_file.as_str(), self.plus_file.as_str()) {
                ("", "") => {
                    return self.handle_additional_cases(match self.state {
                        State::DiffHeader(_) => self.state.clone(),
                        _ => State::DiffHeader(DiffType::Unified),
                    });
                }
                ("/dev/null", _) => self.plus_file.push_str(" (binary file)"),
                (_, "/dev/null") => self.minus_file.push_str(" (binary file)"),
                (_, _) => (),
            };
            return Ok(true);
        }

        self.handle_additional_cases(match self.state {
            State::DiffHeader(_) => self.state.clone(),
            _ => State::DiffHeader(DiffType::Unified),
        })
    }
}
