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
        // A submodule row is a header of the submodule, not of the file whose
        // section precedes it, so the record is built from the path the row
        // itself states -- `minus_file`/`plus_file` never hold it, and tagging
        // the row with those would name the previous file.
        let osc = self.submodule_log_osc();
        self.handle_additional_cases(State::SubmoduleLog, osc)
    }

    /// The `f` record for a submodule row, or an empty string when no host
    /// negotiated emission or the row is no header we can read a path out of.
    fn submodule_log_osc(&self) -> String {
        match (
            self.painter.diff_line_metadata.as_ref(),
            get_submodule_log_path(&self.line),
        ) {
            (Some(md), Some(path)) => md.osc_for_file_header(path),
            _ => String::new(),
        }
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
                        .paint(minus_commit.chars().take(12).collect::<String>()),
                    self.config
                        .plus_style
                        .paint(commit.chars().take(12).collect::<String>()),
                )?;
            }
        }
        Ok(true)
    }
}

lazy_static! {
    static ref SUBMODULE_SHORT_LINE_REGEX: Regex =
        Regex::new("^[-+]Subproject commit ([0-9a-f]{40})(-dirty)?$").unwrap();

    /// Matches the row git opens a submodule's section with, capturing the
    /// submodule's path. There are two kinds: the commit the submodule is
    /// checked out at has moved ("Submodule sub a32f27c..2d9f921:", with "..."
    /// in place of ".." where the move is no fast-forward, " (rewind)" where it
    /// goes backwards, and a message in brackets in place of the colon where
    /// the two commits cannot both be read), or its working tree is dirty
    /// ("Submodule sub contains untracked content").
    ///
    /// The path is captured greedily: git writes it unquoted, so a path that
    /// itself ends in something reading like a range of commits is told apart
    /// by matching the last such range on the row.
    static ref SUBMODULE_LOG_LINE_REGEX: Regex = Regex::new(
        r"^Submodule (.+) (?:contains (?:untracked|modified) content|[0-9a-f]+\.{2,3}[0-9a-f]+(?: \(.*\))?:?)$"
    ).unwrap();
}

pub fn get_submodule_short_commit(line: &str) -> Option<&str> {
    match SUBMODULE_SHORT_LINE_REGEX.captures(line) {
        Some(caps) => Some(caps.get(1).unwrap().as_str()),
        None => None,
    }
}

/// The submodule a "Submodule …" row names, for the rows that are the header of
/// a submodule's section (see `SUBMODULE_LOG_LINE_REGEX`).
pub fn get_submodule_log_path(line: &str) -> Option<&str> {
    SUBMODULE_LOG_LINE_REGEX
        .captures(line)
        .map(|caps| caps.get(1).unwrap().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_submodule_log_path() {
        for (line, expected) in [
            ("Submodule sub/mod a32f27c..2d9f921:", Some("sub/mod")),
            (
                "Submodule sub/mod 2d9f921..a32f27c (rewind):",
                Some("sub/mod"),
            ),
            ("Submodule sub/mod a32f27c...2d9f921:", Some("sub/mod")),
            (
                "Submodule sub/mod 0000000...2d9f921 (new submodule)",
                Some("sub/mod"),
            ),
            (
                "Submodule sub/mod a32f27c...0000000 (submodule deleted)",
                Some("sub/mod"),
            ),
            (
                "Submodule sub/mod a32f27c...2d9f921 (commits not present)",
                Some("sub/mod"),
            ),
            (
                "Submodule sub/mod contains untracked content",
                Some("sub/mod"),
            ),
            (
                "Submodule sub/mod contains modified content",
                Some("sub/mod"),
            ),
            // git writes the path unquoted, so one with a space in it, or one
            // ending in something that reads like a range of commits, is told
            // apart by matching the last range on the row.
            ("Submodule my sub/mod a32f27c..2d9f921:", Some("my sub/mod")),
            (
                "Submodule a32f27c..2d9f921 deadbee..fa1afe1:",
                Some("a32f27c..2d9f921"),
            ),
            ("Submodule support was added", None),
        ] {
            assert_eq!(get_submodule_log_path(line), expected, "{line}");
        }
    }
}
