use std::path::Path;

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
                path if git_diff_name && (path.starts_with("a/") || path.starts_with("b/")) => {
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
) -> String {
    if comparing {
        format!("comparing: {} ⟶   {}", minus_file, plus_file)
    } else {
        match (minus_file, plus_file) {
            (minus_file, plus_file) if minus_file == plus_file => minus_file.to_string(),
            (minus_file, "/dev/null") => format!("deleted: {}", minus_file),
            ("/dev/null", plus_file) => format!("added: {}", plus_file),
            (minus_file, plus_file) => format!("renamed: {} ⟶   {}", minus_file, plus_file),
        }
    }
}

/// Given input like
/// "@@ -74,15 +74,14 @@ pub fn delta("
/// Return " pub fn delta("
pub fn parse_hunk_metadata(line: &str) -> (&str, &str) {
    let mut iter = line.split("@@").skip(1);
    let line_number = iter
        .next()
        .and_then(|s| s.split('+').nth(1).and_then(|s| s.split(',').next()))
        .unwrap_or("");
    let code_fragment = iter.next().unwrap_or("");
    (code_fragment, line_number)
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
        assert_eq!(
            get_file_path_from_file_meta_line("--- a/src/delta.rs", true),
            "src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ b/src/delta.rs", true),
            "src/delta.rs"
        );
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
        assert_eq!(
            parse_hunk_metadata("@@ -74,15 +75,14 @@ pub fn delta(\n"),
            (" pub fn delta(\n", "75")
        );
    }
}
