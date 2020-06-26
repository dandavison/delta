use lazy_static::lazy_static;
use regex::Regex;
use std::path::Path;

use crate::config::Config;

// https://git-scm.com/docs/git-config#Documentation/git-config.txt-diffmnemonicPrefix
const DIFF_PREFIXES: [&str; 6] = ["a/", "b/", "c/", "i/", "o/", "w/"];

/// Given input like
/// "--- one.rs	2019-11-20 06:16:08.000000000 +0100"
/// Return "rs"
pub fn get_file_extension_from_marker_line(line: &str) -> Option<&str> {
    line.split('\t')
        .next()
        .and_then(|column| column.split(' ').nth(1))
        .and_then(|file| file.split('.').last())
}

pub fn get_file_path_from_file_meta_line(line: &str, git_diff_name: bool) -> String {
    match line {
        line if line.starts_with("rename from ") => {
            let offset = "rename from ".len();
            &line[offset..]
        }
        line if line.starts_with("rename to ") => {
            let offset = "rename to ".len();
            &line[offset..]
        }
        line if line.starts_with("--- ") || line.starts_with("+++ ") => {
            let offset = 4;
            match &line[offset..] {
                path if path == "/dev/null" => "/dev/null",
                path if git_diff_name && DIFF_PREFIXES.iter().any(|s| path.starts_with(s)) => {
                    &path[2..]
                }
                path if git_diff_name => &path,
                path => path.split('\t').next().unwrap_or(""),
            }
        }
        _ => "",
    }
    .to_string()
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
    config: &Config,
) -> String {
    if comparing {
        format!("comparing: {} ⟶   {}", minus_file, plus_file)
    } else {
        let format_label = |label: &str| {
            if label.len() > 0 {
                format!("{} ", label)
            } else {
                "".to_string()
            }
        };
        match (minus_file, plus_file) {
            (minus_file, plus_file) if minus_file == plus_file => format!(
                "{}{}",
                format_label(&config.file_modified_label),
                minus_file
            ),
            (minus_file, "/dev/null") => {
                format!("{}{}", format_label(&config.file_removed_label), minus_file)
            }
            ("/dev/null", plus_file) => {
                format!("{}{}", format_label(&config.file_added_label), plus_file)
            }
            (minus_file, plus_file) => format!(
                "{}{} ⟶   {}",
                format_label(&config.file_renamed_label),
                minus_file,
                plus_file
            ),
        }
    }
}

lazy_static! {
    static ref HUNK_METADATA_REGEXP: Regex =
        Regex::new(r"@+ (?P<lns>([-+]\d+(?:,\d+)? ){2,4})@+(?P<cf>.*\s?)").unwrap();
}

lazy_static! {
    static ref LINE_NUMBER_REGEXP: Regex = Regex::new(r"[-+]").unwrap();
}

fn _make_line_number_vec(line: &str) -> Vec<usize> {
    let mut numbers = Vec::<usize>::new();

    for s in LINE_NUMBER_REGEXP.split(line) {
        let number = s.split(',').nth(0).unwrap().split_whitespace().nth(0);
        match number {
            Some(number) => numbers.push(number.parse::<usize>().unwrap()),
            None => continue,
        }
    }
    return numbers;
}

/// Given input like
/// "@@ -74,15 +74,14 @@ pub fn delta("
/// Return " pub fn delta(" and a vector of line numbers
pub fn parse_hunk_metadata(line: &str) -> (&str, Vec<usize>) {
    let caps = HUNK_METADATA_REGEXP.captures(line).unwrap();
    let line_numbers = _make_line_number_vec(caps.name("lns").unwrap().as_str());
    let code_fragment = caps.name("cf").unwrap().as_str();
    return (code_fragment, line_numbers);
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
            get_file_path_from_file_meta_line("--- /dev/null", true),
            "/dev/null"
        );
        for prefix in &DIFF_PREFIXES {
            assert_eq!(
                get_file_path_from_file_meta_line(&format!("--- {}src/delta.rs", prefix), true),
                "src/delta.rs"
            );
        }
        assert_eq!(
            get_file_path_from_file_meta_line("--- src/delta.rs", true),
            "src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ src/delta.rs", true),
            "src/delta.rs"
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_containing_spaces() {
        assert_eq!(
            get_file_path_from_file_meta_line("+++ a/my src/delta.rs", true),
            "my src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ my src/delta.rs", true),
            "my src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ a/src/my delta.rs", true),
            "src/my delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ a/my src/my delta.rs", true),
            "my src/my delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ b/my src/my enough/my delta.rs", true),
            "my src/my enough/my delta.rs"
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_rename() {
        assert_eq!(
            get_file_path_from_file_meta_line("rename from nospace/file2.el", true),
            "nospace/file2.el"
        );
    }

    #[test]
    fn test_get_file_path_from_git_file_meta_line_rename_containing_spaces() {
        assert_eq!(
            get_file_path_from_file_meta_line("rename from with space/file1.el", true),
            "with space/file1.el"
        );
    }

    #[test]
    fn test_get_file_path_from_file_meta_line() {
        assert_eq!(
            get_file_path_from_file_meta_line("--- src/delta.rs", false),
            "src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ src/delta.rs", false),
            "src/delta.rs"
        );
    }

    #[test]
    fn test_parse_hunk_metadata() {
        let parsed = parse_hunk_metadata("@@ -74,15 +75,14 @@ pub fn delta(\n");
        let code_fragment = parsed.0;
        let line_numbers = parsed.1;
        assert_eq!(code_fragment, " pub fn delta(\n",);
        assert_eq!(line_numbers[0], 74,);
        assert_eq!(line_numbers[1], 75,);
    }

    #[test]
    fn test_parse_hunk_metadata_added_file() {
        let parsed = parse_hunk_metadata("@@ -1,22 +0,0 @@");
        let code_fragment = parsed.0;
        let line_numbers = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers[0], 1,);
        assert_eq!(line_numbers[1], 0,);
    }

    #[test]
    fn test_parse_hunk_metadata_deleted_file() {
        let parsed = parse_hunk_metadata("@@ -0,0 +1,3 @@");
        let code_fragment = parsed.0;
        let line_numbers = parsed.1;
        assert_eq!(code_fragment, "",);
        assert_eq!(line_numbers[0], 0,);
        assert_eq!(line_numbers[1], 1,);
    }

    #[test]
    fn test_parse_hunk_metadata_merge() {
        let parsed = parse_hunk_metadata("@@@ -293,11 -358,15 +358,16 @@@ dependencies =");
        let code_fragment = parsed.0;
        let line_numbers = parsed.1;
        assert_eq!(code_fragment, " dependencies =",);
        assert_eq!(line_numbers[0], 293,);
        assert_eq!(line_numbers[1], 358,);
        assert_eq!(line_numbers[2], 358,);
    }
}
