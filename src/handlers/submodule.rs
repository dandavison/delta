use lazy_static::lazy_static;
use regex::Regex;

use crate::delta::{State, StateMachine};

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_submodule_log(&self) -> bool {
        self.line.starts_with("Submodule ")
    }

    pub fn handle_submodule_log_line(&mut self) -> std::io::Result<bool> {
        if !self.test_submodule_log() {
            return Ok(false);
        }
        self.handle_additional_cases(State::SubmoduleLog)
    }

    #[inline]
    fn test_submodule_short_line(&self) -> bool {
        matches!(self.state, State::HunkHeader(_, _, _, _))
            && self.line.starts_with("-Subproject commit ")
            || matches!(self.state, State::SubmoduleShort(_))
                && self.line.starts_with("+Subproject commit ")
    }

    pub fn handle_submodule_short_line(&mut self) -> std::io::Result<bool> {
        if !self.test_submodule_short_line() || self.config.color_only {
            return Ok(false);
        }
        if let Some(commit) = get_submodule_short_commit(&self.line) {
            if let State::HunkHeader(_, _, _, _) = self.state {
                self.state = State::SubmoduleShort(commit.to_owned());
            } else if let State::SubmoduleShort(minus_commit) = &self.state {
                self.painter.emit()?;
                writeln!(
                    self.painter.writer,
                    "{}..{}",
                    self.config
                        .minus_style
                        .paint(minus_commit.chars().take(7).collect::<String>()),
                    self.config
                        .plus_style
                        .paint(commit.chars().take(7).collect::<String>()),
                )?;
            }
        }
        Ok(true)
    }
}

lazy_static! {
    static ref SUBMODULE_SHORT_LINE_REGEX: Regex =
        Regex::new("^[-+]Subproject commit ([0-9a-f]{40})$").unwrap();
}

pub fn get_submodule_short_commit(line: &str) -> Option<&str> {
    match SUBMODULE_SHORT_LINE_REGEX.captures(line) {
        Some(caps) => Some(caps.get(1).unwrap().as_str()),
        None => None,
    }
}
