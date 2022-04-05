use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

use crate::config::Config;
use crate::delta::{State, StateMachine};
use crate::features;
use crate::utils;

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_diff_stat_line(&self) -> bool {
        (self.state == State::CommitMeta || self.state == State::Unknown)
            && self.line.starts_with(' ')
    }

    pub fn handle_diff_stat_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_stat_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        if self.config.relative_paths {
            if let Some(cwd) = self.config.cwd_relative_to_repo_root.as_deref() {
                if let Some(replacement_line) =
                    relativize_path_in_diff_stat_line(&self.raw_line, cwd, self.config)
                {
                    self.painter.emit()?;
                    writeln!(self.painter.writer, "{}", replacement_line)?;
                    handled_line = true
                }
            }
        }
        Ok(handled_line)
    }
}

// A regex to capture the path, and the content from the pipe onwards, in lines
// like these:
// " src/delta.rs  | 14 ++++++++++----"
// " src/config.rs |  2 ++"
lazy_static! {
    static ref DIFF_STAT_LINE_REGEX: Regex =
        Regex::new(r" ([^\| ][^\|]+[^\| ]) +(\| +[0-9]+ .+)").unwrap();
}

pub fn relativize_path_in_diff_stat_line(
    line: &str,
    cwd_relative_to_repo_root: &str,
    config: &Config,
) -> Option<String> {
    let caps = DIFF_STAT_LINE_REGEX.captures(line)?;
    let path_relative_to_repo_root = caps.get(1).unwrap().as_str();

    let relative_path =
        pathdiff::diff_paths(path_relative_to_repo_root, cwd_relative_to_repo_root)?;
    let relative_path = relative_path.to_str()?;
    let formatted_path = match (
        config.hyperlinks,
        utils::path::absolute_path(path_relative_to_repo_root, config),
    ) {
        (true, Some(absolute_path)) => features::hyperlinks::format_osc8_file_hyperlink(
            absolute_path,
            None,
            relative_path,
            config,
        ),
        _ => Cow::from(relative_path),
    };
    let suffix = caps.get(2).unwrap().as_str();
    let pad_width = config
        .diff_stat_align_width
        .saturating_sub(relative_path.len());
    let padding = " ".repeat(pad_width);
    Some(format!(" {}{}{}", formatted_path, padding, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_stat_line_regex_1() {
        let caps = DIFF_STAT_LINE_REGEX.captures(" src/delta.rs  | 14 ++++++++++----");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "src/delta.rs");
        assert_eq!(caps.get(2).unwrap().as_str(), "| 14 ++++++++++----");
    }

    #[test]
    fn test_diff_stat_line_regex_2() {
        let caps = DIFF_STAT_LINE_REGEX.captures(" src/config.rs |  2 ++");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "src/config.rs");
        assert_eq!(caps.get(2).unwrap().as_str(), "|  2 ++");
    }

    #[test]
    fn test_relative_path() {
        for (path, cwd_relative_to_repo_root, expected) in &[
            ("file.rs", "", "file.rs"),
            ("file.rs", "a/", "../file.rs"),
            ("a/file.rs", "a/", "file.rs"),
            ("a/b/file.rs", "a", "b/file.rs"),
            ("c/d/file.rs", "a/b/", "../../c/d/file.rs"),
        ] {
            assert_eq!(
                pathdiff::diff_paths(path, cwd_relative_to_repo_root),
                Some(expected.into())
            )
        }
    }
}
