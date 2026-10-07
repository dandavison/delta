use std::io::{self, Write};

const LESS_HISTORY_HEADER: &str = ".less-history-file:";
const LESS_HISTORY_SEARCH_SECTION: &str = ".search";
const LESS_HISTORY_SHELL_SECTION: &str = ".shell";
const LESS_HISTORY_MARK_SECTION: &str = ".mark";

// New search entries start after this much overlap with the old one
const OLD_NEW_SEARCH_EXPECTED_OVERLAP: usize = 3;

#[derive(Debug, Eq, PartialEq)]
pub enum DeltaNavigate {
    None,
    Add(String),
    Remove(String),
}

#[derive(Default, Debug)]
pub struct LessHistory {
    search: Vec<String>,
    shell: Vec<String>,
    marks: Vec<String>,
}

impl LessHistory {
    #[cfg(test)]
    pub fn to_string(&self) -> String {
        let mut buffer = Vec::new();
        self.write_into(&mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    pub fn from_str(content: &str) -> Option<Self> {
        #[derive(Debug, Clone, Eq, PartialEq)]
        enum Section {
            None,
            Search,
            Shell,
            Mark,
        }

        let mut history = LessHistory::default();
        let mut section = Section::None;
        let mut it = content.lines();

        match it.next() {
            Some(line) if line.starts_with(LESS_HISTORY_HEADER) => {}
            _ => return None,
        }

        for line in it {
            if line.is_empty() {
                continue;
            }

            match (section.clone(), line) {
                (_, LESS_HISTORY_SEARCH_SECTION) => section = Section::Search,
                (_, LESS_HISTORY_SHELL_SECTION) => section = Section::Shell,
                (_, LESS_HISTORY_MARK_SECTION) => section = Section::Mark,
                (Section::Search, line) if line.starts_with('"') => {
                    history.search.push(line.to_string())
                }
                (Section::Shell, line) if line.starts_with('"') => {
                    history.shell.push(line.to_string())
                }
                (Section::Mark, line) if line.starts_with('m') => {
                    history.marks.push(line.to_string())
                }
                _ => {} // TODO: keep unknown (and empty?) lines for forward compatibility
            }
        }

        Some(history)
    }

    fn apply_navigate(&mut self, extra: DeltaNavigate) {
        match extra {
            DeltaNavigate::None => {}
            DeltaNavigate::Add(value) => self.search.push(format!("\"{value}")),
            DeltaNavigate::Remove(value) => {
                let quote_value = format!("\"{value}");
                self.search.retain(|entry| entry != &quote_value);
            }
        }
    }

    /// Compute the searches added to the `newer` history by searching for an overlap
    /// of ` NEW_SEARCHES_MATCH_COUNT` entries. This assumes lesshst file is only
    /// used by a single less instance by setting LESSHISTFILE to that file.
    pub fn new_searches(&self, newer: &LessHistory) -> Vec<String> {
        let self_tail_len = self.search.len().min(OLD_NEW_SEARCH_EXPECTED_OVERLAP);
        if self_tail_len == 0 {
            // All `newer` entries are new
            return newer.search.clone();
        }

        let self_tail = &self.search[self.search.len() - self_tail_len..];

        if newer.search.len() < self_tail_len {
            // `newer` is shorter than the wanted overlap. TODO, dedupe
            return newer.search.clone();
        }

        // Search backwards in `newer` for a matching sequence.
        // TODO, a bit inefficient because the entire slice is compared again and again.
        let mut match_start = None;
        for start in (0..=newer.search.len() - self_tail_len).rev() {
            let other_slice = &newer.search[start..start + self_tail_len];
            if other_slice == self_tail {
                match_start = Some(start);
                break;
            }
        }

        match match_start {
            Some(start) => {
                // Found match, entries after the match are new
                let match_end = start + self_tail_len;
                newer.search[match_end..].to_vec()
            }
            None => {
                // All `newer` entries are new (exceeded LESSHISTSIZE)
                newer.search.clone()
            }
        }
    }

    fn write_into<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "{LESS_HISTORY_HEADER}")?;
        Self::write_section(writer, LESS_HISTORY_SEARCH_SECTION, &self.search)?;
        Self::write_section(writer, LESS_HISTORY_SHELL_SECTION, &self.shell)?;
        Self::write_section(writer, LESS_HISTORY_MARK_SECTION, &self.marks)?;
        Ok(())
    }

    fn write_section<W: Write>(
        writer: &mut W,
        section: &str,
        entries: &[String],
    ) -> io::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        writeln!(writer, "{section}")?;
        for entry in entries {
            writeln!(writer, "{entry}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DeltaNavigate, LessHistory};
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;

    const HIST_FILE_EXAMPLE: &str = r#".less-history-file:
.search
"haystack
"needle
"haystack
.shell
"pwd
"pwd2
"pwd
.mark
m marked/abc
m marked/defgh
"#;

    #[test]
    fn test_less_hist_delta_regex_round_trip() {
        let mut hist = LessHistory::from_str(HIST_FILE_EXAMPLE).unwrap();
        hist.apply_navigate(DeltaNavigate::Add("DeltaRegex".into()));
        let inserted_regex = hist.to_string();
        assert_snapshot!(inserted_regex, @r#"
        .less-history-file:
        .search
        "haystack
        "needle
        "haystack
        "DeltaRegex
        .shell
        "pwd
        "pwd2
        "pwd
        .mark
        m marked/abc
        m marked/defgh
        "#);

        let mut hist = LessHistory::from_str(&inserted_regex).unwrap();
        hist.apply_navigate(DeltaNavigate::Remove("DeltaRegex".into()));
        let removed_regex = hist.to_string();
        assert_eq!(HIST_FILE_EXAMPLE, removed_regex);
    }

    #[test]
    fn test_less_hist_detect_new_searches() {
        let orig = LessHistory::from_str(HIST_FILE_EXAMPLE).unwrap();
        let mut hist = LessHistory::from_str(HIST_FILE_EXAMPLE).unwrap();
        hist.apply_navigate(DeltaNavigate::Add("search one".into()));
        hist.apply_navigate(DeltaNavigate::None);
        hist.apply_navigate(DeltaNavigate::Add("search II".into()));
        hist.apply_navigate(DeltaNavigate::Add("search 3".into()));

        assert_eq!(
            orig.new_searches(&hist),
            ["\"search one", "\"search II", "\"search 3",]
        );
    }
}
