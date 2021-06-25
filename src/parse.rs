use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

use crate::config::Config;
use crate::features;

// https://git-scm.com/docs/git-config#Documentation/git-config.txt-diffmnemonicPrefix
const DIFF_PREFIXES: [&str; 6] = ["a/", "b/", "c/", "i/", "o/", "w/"];

#[allow(clippy::tabs_in_doc_comments)]
/// Given input like
/// "--- one.rs	2019-11-20 06:16:08.000000000 +0100"
/// Return "rs"
pub fn get_file_extension_from_marker_line(line: &str) -> Option<&str> {
    line.split('\t')
        .next()
        .and_then(|column| column.split(' ').nth(1))
        .and_then(|file| file.split('.').last())
}

#[derive(Debug, PartialEq)]
pub enum FileEvent {
    Change,
    Copy,
    Rename,
    ModeChange(String),
    NoEvent,
}

pub fn parse_file_meta_line(
    line: &str,
    git_diff_name: bool,
    relative_path_base: Option<&str>,
) -> (String, FileEvent) {
    let (mut path_or_mode, file_event) = match line {
        line if line.starts_with("--- ") || line.starts_with("+++ ") => {
            let offset = 4;
            let file = _parse_file_path(&line[offset..], git_diff_name);
            (file, FileEvent::Change)
        }
        line if line.starts_with("rename from ") => {
            (line[12..].to_string(), FileEvent::Rename) // "rename from ".len()
        }
        line if line.starts_with("rename to ") => {
            (line[10..].to_string(), FileEvent::Rename) // "rename to ".len()
        }
        line if line.starts_with("copy from ") => {
            (line[10..].to_string(), FileEvent::Copy) // "copy from ".len()
        }
        line if line.starts_with("copy to ") => {
            (line[8..].to_string(), FileEvent::Copy) // "copy to ".len()
        }
        line if line.starts_with("old mode ") => {
            ("".to_string(), FileEvent::ModeChange(line[9..].to_string())) // "old mode ".len()
        }
        line if line.starts_with("new mode ") => {
            ("".to_string(), FileEvent::ModeChange(line[9..].to_string())) // "new mode ".len()
        }
        _ => ("".to_string(), FileEvent::NoEvent),
    };

    if let Some(base) = relative_path_base {
        if let FileEvent::ModeChange(_) = file_event {
        } else if let Some(relative_path) = pathdiff::diff_paths(&path_or_mode, base) {
            if let Some(relative_path) = relative_path.to_str() {
                path_or_mode = relative_path.to_owned();
            }
        }
    }

    (path_or_mode, file_event)
}

/// Given input like "diff --git a/src/my file.rs b/src/my file.rs"
/// return Some("src/my file.rs")
pub fn get_repeated_file_path_from_diff_line(line: &str) -> Option<String> {
    if let Some(line) = line.strip_prefix("diff --git ") {
        let line: Vec<&str> = line.graphemes(true).collect();
        let midpoint = line.len() / 2;
        if line[midpoint] == " " {
            let first_path = _parse_file_path(&line[..midpoint].join(""), true);
            let second_path = _parse_file_path(&line[midpoint + 1..].join(""), true);
            if first_path == second_path {
                return Some(first_path);
            }
        }
    }
    None
}

fn _parse_file_path(s: &str, git_diff_name: bool) -> String {
    // It appears that, if the file name contains a space, git appends a tab
    // character in the diff metadata lines, e.g.
    // $ git diff --no-index "a b" "c d" | cat -A
    // diff·--git·a/a·b·b/c·d␊
    // index·d00491f..0cfbf08·100644␊
    // ---·a/a·b├──┤␊
    // +++·b/c·d├──┤␊
    match s.strip_suffix("\t").unwrap_or(s) {
        path if path == "/dev/null" => "/dev/null",
        path if git_diff_name && DIFF_PREFIXES.iter().any(|s| path.starts_with(s)) => &path[2..],
        path if git_diff_name => &path,
        path => path.split('\t').next().unwrap_or(""),
    }
    .to_string()
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
    diff_stat_align_width: usize,
) -> Option<String> {
    if let Some(caps) = DIFF_STAT_LINE_REGEX.captures(line) {
        let path_relative_to_repo_root = caps.get(1).unwrap().as_str();
        if let Some(relative_path) =
            pathdiff::diff_paths(path_relative_to_repo_root, cwd_relative_to_repo_root)
        {
            if let Some(relative_path) = relative_path.to_str() {
                let suffix = caps.get(2).unwrap().as_str();
                let pad_width = diff_stat_align_width.saturating_sub(relative_path.len());
                let padding = " ".repeat(pad_width);
                return Some(format!(" {}{}{}", relative_path, padding, suffix));
            }
        }
    }
    None
}

pub fn get_file_extension_from_file_meta_line_file_path(path: &str) -> Option<&str> {
    if path.is_empty() || path == "/dev/null" {
        None
    } else {
        get_extension(&path).map(|ex| ex.trim())
    }
}

pub fn get_file_change_description_from_file_paths(
    minus_file: &str,
    plus_file: &str,
    comparing: bool,
    minus_file_event: &FileEvent,
    plus_file_event: &FileEvent,
    config: &Config,
) -> String {
    if comparing {
        format!("comparing: {} ⟶   {}", minus_file, plus_file)
    } else {
        let format_label = |label: &str| {
            if !label.is_empty() {
                format!("{} ", label)
            } else {
                "".to_string()
            }
        };
        let format_file = |file| {
            if config.hyperlinks {
                features::hyperlinks::format_osc8_file_hyperlink(file, None, file, config)
            } else {
                Cow::from(file)
            }
        };
        match (minus_file, plus_file, minus_file_event, plus_file_event) {
            (
                minus_file,
                plus_file,
                FileEvent::ModeChange(old_mode),
                FileEvent::ModeChange(new_mode),
            ) if minus_file == plus_file => match (old_mode.as_str(), new_mode.as_str()) {
                // 100755 for executable and 100644 for non-executable are the only file modes Git records.
                // https://medium.com/@tahteche/how-git-treats-changes-in-file-permissions-f71874ca239d
                ("100644", "100755") => format!("{}: mode +x", plus_file),
                ("100755", "100644") => format!("{}: mode -x", plus_file),
                _ => format!("{}: {} ⟶   {}", plus_file, old_mode, new_mode),
            },
            (minus_file, plus_file, _, _) if minus_file == plus_file => format!(
                "{}{}",
                format_label(&config.file_modified_label),
                format_file(minus_file)
            ),
            (minus_file, "/dev/null", _, _) => format!(
                "{}{}",
                format_label(&config.file_removed_label),
                format_file(minus_file)
            ),
            ("/dev/null", plus_file, _, _) => format!(
                "{}{}",
                format_label(&config.file_added_label),
                format_file(plus_file)
            ),
            // minus_file_event == plus_file_event, except in the ModeChange
            // case above.
            (minus_file, plus_file, file_event, _) => format!(
                "{}{} ⟶   {}",
                format_label(match file_event {
                    FileEvent::Rename => &config.file_renamed_label,
                    FileEvent::Copy => &config.file_copied_label,
                    _ => "",
                }),
                format_file(minus_file),
                format_file(plus_file)
            ),
        }
    }
}

lazy_static! {
    static ref HUNK_HEADER_REGEX: Regex = Regex::new(r"@+ ([^@]+)@+(.*\s?)").unwrap();
}

// Parse unified diff hunk header format. See
// https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html
// https://www.artima.com/weblogs/viewpost.jsp?thread=164293
lazy_static! {
    static ref HUNK_HEADER_FILE_COORDINATE_REGEX: Regex = Regex::new(
        r"(?x)
[-+]
(\d+)            # 1. Hunk start line number
(?:              # Start optional hunk length section (non-capturing)
  ,              #   Literal comma
  (\d+)          #   2. Optional hunk length (defaults to 1)
)?"
    )
    .unwrap();
}

/// Given input like
/// "@@ -74,15 +74,14 @@ pub fn delta("
/// Return " pub fn delta(" and a vector of (line_number, hunk_length) tuples.
pub fn parse_hunk_header(line: &str) -> (String, Vec<(usize, usize)>) {
    let caps = HUNK_HEADER_REGEX.captures(line).unwrap();
    let file_coordinates = &caps[1];
    let line_numbers_and_hunk_lengths = HUNK_HEADER_FILE_COORDINATE_REGEX
        .captures_iter(file_coordinates)
        .map(|caps| {
            (
                caps[1].parse::<usize>().unwrap(),
                caps.get(2)
                    .map(|m| m.as_str())
                    // Per the specs linked above, if the hunk length is absent then it is 1.
                    .unwrap_or("1")
                    .parse::<usize>()
                    .unwrap(),
            )
        })
        .collect();
    let code_fragment = &caps[2];
    (code_fragment.to_string(), line_numbers_and_hunk_lengths)
}

/// Attempt to parse input as a file path and return extension as a &str.
fn get_extension(s: &str) -> Option<&str> {
    let path = Path::new(s);
    path.extension()
        .and_then(|e| e.to_str())
        // E.g. 'Makefile' is the file name and also the extension
        .or_else(|| path.file_name().and_then(|s| s.to_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repeated_file_path_from_diff_line() {
        assert_eq!(
            get_repeated_file_path_from_diff_line("diff --git a/src/main.rs b/src/main.rs"),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            get_repeated_file_path_from_diff_line("diff --git a/a b/a"),
            Some("a".to_string())
        );
        assert_eq!(
            get_repeated_file_path_from_diff_line("diff --git a/a b b/a b"),
            Some("a b".to_string())
        );
        assert_eq!(
            get_repeated_file_path_from_diff_line("diff --git a/a b/aa"),
            None
        );
        assert_eq!(
            get_repeated_file_path_from_diff_line("diff --git a/.config/Code - Insiders/User/settings.json b/.config/Code - Insiders/User/settings.json"),
            Some(".config/Code - Insiders/User/settings.json".to_string())
        );
    }

    #[test]
    fn test_get_file_extension_from_marker_line() {
        assert_eq!(
            get_file_extension_from_marker_line(
                "--- src/one.rs	2019-11-20 06:47:56.000000000 +0100"
            ),
            Some("rs")
        );
    }

    #[test]
    fn test_get_file_extension_from_file_meta_line() {
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("a/src/parse.rs"),
            Some("rs")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("b/src/pa rse.rs"),
            Some("rs")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("src/pa rse.rs"),
            Some("rs")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("wat hello.rs"),
            Some("rs")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("/dev/null"),
            None
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("Dockerfile"),
            Some("Dockerfile")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("Makefile"),
            Some("Makefile")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("a/src/Makefile"),
            Some("Makefile")
        );
        assert_eq!(
            get_file_extension_from_file_meta_line_file_path("src/Makefile"),
            Some("Makefile")
        );
    }

    // We should only strip the prefixes if they are "a/" or "b/". This will be correct except for
    // the case of a user with `diff.noprefix = true` who has directories named "a" or "b", which
    // is an irresolvable ambiguity. Ideally one would only strip the prefixes if we have confirmed
    // that we are looking at something like
    //
    // --- a/src/parse.rs
    // +++ b/src/parse.rs
    //
    // as opposed to something like
    //
    // --- a/src/parse.rs
    // +++ sibling_of_a/src/parse.rs
    //
    // but we don't attempt that currently.
    #[test]
    fn test_get_file_path_from_git_file_meta_line() {
        assert_eq!(
            parse_file_meta_line("--- /dev/null", true, None),
            ("/dev/null".to_string(), FileEvent::Change)
        );
        for prefix in &DIFF_PREFIXES {
            assert_eq!(
                parse_file_meta_line(&format!("--- {}src/delta.rs", prefix), true, None),
                ("src/delta.rs".to_string(), FileEvent::Change)
            );
        }
        assert_eq!(
            parse_file_meta_line("--- src/delta.rs", true, None),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ src/delta.rs", true, None),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_containing_spaces() {
        assert_eq!(
            parse_file_meta_line("+++ a/my src/delta.rs", true, None),
            ("my src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ my src/delta.rs", true, None),
            ("my src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ a/src/my delta.rs", true, None),
            ("src/my delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ a/my src/my delta.rs", true, None),
            ("my src/my delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ b/my src/my enough/my delta.rs", true, None),
            (
                "my src/my enough/my delta.rs".to_string(),
                FileEvent::Change
            )
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_rename() {
        assert_eq!(
            parse_file_meta_line("rename from nospace/file2.el", true, None),
            ("nospace/file2.el".to_string(), FileEvent::Rename)
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_rename_containing_spaces() {
        assert_eq!(
            parse_file_meta_line("rename from with space/file1.el", true, None),
            ("with space/file1.el".to_string(), FileEvent::Rename)
        );
    }

    #[test]
    fn test_parse_file_meta_line() {
        assert_eq!(
            parse_file_meta_line("--- src/delta.rs", false, None),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_file_meta_line("+++ src/delta.rs", false, None),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
    }

    #[test]
    fn test_parse_hunk_header() {
        let parsed = parse_hunk_header("@@ -74,15 +75,14 @@ pub fn delta(\n");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 14),);
    }

    #[test]
    fn test_parse_hunk_header_with_omitted_hunk_lengths() {
        let parsed = parse_hunk_header("@@ -74 +75,2 @@ pub fn delta(\n");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " pub fn delta(\n");
        assert_eq!(line_numbers_and_hunk_lengths[0], (74, 1),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (75, 2),);
    }

    #[test]
    fn test_parse_hunk_header_added_file() {
        let parsed = parse_hunk_header("@@ -1,22 +0,0 @@");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (1, 22),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (0, 0),);
    }

    #[test]
    fn test_parse_hunk_header_deleted_file() {
        let parsed = parse_hunk_header("@@ -0,0 +1,3 @@");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers_and_hunk_lengths[0], (0, 0),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (1, 3),);
    }

    #[test]
    fn test_parse_hunk_header_merge() {
        let parsed = parse_hunk_header("@@@ -293,11 -358,15 +358,16 @@@ dependencies =");
        let code_fragment = parsed.0;
        let line_numbers_and_hunk_lengths = parsed.1;
        assert_eq!(code_fragment, " dependencies =");
        assert_eq!(line_numbers_and_hunk_lengths[0], (293, 11),);
        assert_eq!(line_numbers_and_hunk_lengths[1], (358, 15),);
        assert_eq!(line_numbers_and_hunk_lengths[2], (358, 16),);
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
}
