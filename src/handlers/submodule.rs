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
        match self.state {
            State::HunkHeader(_, _, _, _) => {
                self.line.starts_with("-Subproject commit ")
                    || self.line.starts_with("+Subproject commit ")
            }
            State::SubmoduleShort(_) => self.line.starts_with("+Subproject commit "),
            _ => false,
        }
    }

    pub fn handle_submodule_short_line(&mut self) -> std::io::Result<bool> {
        if !self.test_submodule_short_line() || self.config.color_only {
            return Ok(false);
        }
        let Some((commit, dirty)) = get_submodule_short_commit(&self.line) else {
            return Ok(true);
        };
        let commit = commit.to_owned();
        let is_minus_line = self.line.starts_with('-');
        if let State::HunkHeader(_, parsed_hunk_header, _, _) = &self.state {
            // The row drawn for the submodule replaces the hunk, hunk header
            // included, so it carries the hunk header's record.
            if let Some(md) = self.painter.diff_line_metadata.as_mut() {
                let file = if self.plus_file == "/dev/null" {
                    &self.minus_file
                } else {
                    &self.plus_file
                };
                md.initialize_hunk(
                    &parsed_hunk_header.line_numbers_and_hunk_lengths,
                    file.clone(),
                );
            }
            // A removed submodule's hunk has a "-" line and no "+" line
            // ("@@ -1 +0,0 @@"); an added submodule's has only a "+" line.
            let removed = is_minus_line
                && matches!(
                    parsed_hunk_header.line_numbers_and_hunk_lengths.last(),
                    Some((_, 0))
                );
            if removed {
                self.write_submodule_short_row(Some(&commit), None, dirty)?;
            } else if !is_minus_line {
                self.write_submodule_short_row(None, Some(&commit), dirty)?;
            }
            self.state = State::SubmoduleShort(commit);
        } else if let State::SubmoduleShort(minus_commit) = &self.state {
            let minus_commit = minus_commit.clone();
            self.write_submodule_short_row(Some(&minus_commit), Some(&commit), dirty)?;
        }
        Ok(true)
    }

    /// Write the row that stands for a submodule's hunk: the commit it moved
    /// from and the commit it moved to, of which a removed submodule has only
    /// the first and an added submodule only the second.
    fn write_submodule_short_row(
        &mut self,
        minus_commit: Option<&str>,
        plus_commit: Option<&str>,
        dirty: bool,
    ) -> std::io::Result<()> {
        self.painter.emit()?;
        let short = |commit: &str| commit.chars().take(12).collect::<String>();
        let minus = minus_commit.map(|c| self.config.minus_style.paint(short(c)).to_string());
        let plus = plus_commit.map(|c| self.config.plus_style.paint(short(c)).to_string());
        let commits = match (minus, plus) {
            // git writes "-dirty" after the new commit when the submodule's
            // working tree has uncommitted changes or untracked files. When
            // that is the only change, the two commits are the same.
            (Some(_), Some(plus)) if minus_commit == plus_commit => plus,
            (Some(minus), Some(plus)) => format!("{minus}..{plus}"),
            (Some(commit), None) | (None, Some(commit)) => commit,
            (None, None) => String::new(),
        };
        let dirty = if dirty { " (dirty)" } else { "" };
        let osc = self
            .painter
            .diff_line_metadata
            .as_ref()
            .map_or(String::new(), |md| md.osc_for_hunk_header());
        writeln!(self.painter.writer, "{osc}{commits}{dirty}")
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

/// The commit a "Subproject commit" row states, and whether the row marks the
/// submodule's working tree as dirty.
pub fn get_submodule_short_commit(line: &str) -> Option<(&str, bool)> {
    SUBMODULE_SHORT_LINE_REGEX
        .captures(line)
        .map(|caps| (caps.get(1).unwrap().as_str(), caps.get(2).is_some()))
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

    #[test]
    fn test_get_submodule_short_commit_reports_dirty_state() {
        const COMMIT: &str = "ca030fd1a02225a6fc1a834c480276d9c97a8c6f";

        let line = format!("-Subproject commit {COMMIT}");
        assert_eq!(get_submodule_short_commit(&line), Some((COMMIT, false)));

        let line = format!("+Subproject commit {COMMIT}-dirty");
        assert_eq!(get_submodule_short_commit(&line), Some((COMMIT, true)));

        assert_eq!(get_submodule_short_commit("Subproject commit nope"), None);
    }
}
