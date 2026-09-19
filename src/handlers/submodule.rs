use lazy_static::lazy_static;
use regex::Regex;

use crate::delta::{State, StateMachine};

impl StateMachine<'_> {
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
                self.state = State::SubmoduleShort(commit);
            } else if let State::SubmoduleShort(minus_commit) = &self.state {
                self.painter.emit()?;
                writeln!(
                    self.painter.writer,
                    "{}..{}",
                    self.config.minus_style.paint(minus_commit.as_str()),
                    self.config.plus_style.paint(commit.as_str()),
                )?;
            }
        }
        Ok(true)
    }
}

lazy_static! {
    static ref SUBMODULE_SHORT_LINE_REGEX: Regex =
        Regex::new("^[-+]Subproject commit ([0-9a-f]{40})(-dirty)?$").unwrap();
}

pub fn get_submodule_short_commit(line: &str) -> Option<String> {
    SUBMODULE_SHORT_LINE_REGEX.captures(line).map(|caps| {
        let mut commit = caps
            .get(1)
            .unwrap()
            .as_str()
            .chars()
            .take(12)
            .collect::<String>();
        if caps.get(2).is_some() {
            commit.push_str("-dirty");
        }
        commit
    })
}

#[cfg(test)]
mod tests {
    use super::get_submodule_short_commit;

    #[test]
    fn submodule_short_commit_keeps_clean_output_short() {
        assert_eq!(
            get_submodule_short_commit(
                "-Subproject commit ca030fd1a02225a6fc1a834c480276d9c97a8c6f",
            ),
            Some("ca030fd1a022".to_owned()),
        );
    }

    #[test]
    fn submodule_short_commit_preserves_dirty_marker() {
        assert_eq!(
            get_submodule_short_commit(
                "+Subproject commit 803be42ca46af0fbc65b54a9abfb499389516939-dirty",
            ),
            Some("803be42ca46a-dirty".to_owned()),
        );
    }
}
