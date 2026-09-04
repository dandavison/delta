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
        if let Some((commit, dirty)) = get_submodule_short_commit(&self.line) {
            if let State::HunkHeader(_, _, _, _) = self.state {
                // Encode the dirty flag in the stored state: the commit hash is
                // always exactly 40 hex characters, so a trailing marker byte
                // can't collide with it.
                self.state = State::SubmoduleShort(encode_submodule_state(commit, dirty));
            } else if let State::SubmoduleShort(minus_state) = &self.state {
                let (minus_commit, minus_dirty) = decode_submodule_state(minus_state);
                self.painter.emit()?;
                writeln!(
                    self.painter.writer,
                    "{}..{}",
                    self.config
                        .minus_style
                        .paint(format_submodule_commit(minus_commit, minus_dirty)),
                    self.config
                        .plus_style
                        .paint(format_submodule_commit(commit, dirty)),
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

/// Returns the submodule's 40-character commit hash, and whether git marked
/// it "-dirty" (i.e. the submodule's working tree has uncommitted changes).
pub fn get_submodule_short_commit(line: &str) -> Option<(&str, bool)> {
    match SUBMODULE_SHORT_LINE_REGEX.captures(line) {
        Some(caps) => Some((caps.get(1).unwrap().as_str(), caps.get(2).is_some())),
        None => None,
    }
}

fn encode_submodule_state(commit: &str, dirty: bool) -> String {
    format!("{}{}", commit, if dirty { "!" } else { "" })
}

fn decode_submodule_state(state: &str) -> (&str, bool) {
    match state.strip_suffix('!') {
        Some(commit) => (commit, true),
        None => (state, false),
    }
}

/// Renders a truncated commit hash with git's own "-dirty" marker preserved,
/// so a submodule with uncommitted local changes is visibly distinguished
/// from one that's merely at a different, clean commit.
fn format_submodule_commit(commit: &str, dirty: bool) -> String {
    format!(
        "{}{}",
        commit.chars().take(12).collect::<String>(),
        if dirty { "-dirty" } else { "" }
    )
}
