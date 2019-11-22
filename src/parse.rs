use std::path::Path;

/// Given input like
/// "diff --git a/src/main.rs b/src/main.rs"
/// Return "rs", i.e. a single file extension consistent with both files.
pub fn get_file_extension_from_diff_line(line: &str) -> Option<&str> {
    match get_file_extensions_from_diff_line(line) {
        (Some(_ext1), Some(ext2)) => {
            // If they differ then it's a rename; use the new extension.
            Some(ext2)
        }
        (Some(ext1), None) => Some(ext1),
        (None, Some(ext2)) => Some(ext2),
        (None, None) => None,
    }
}

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
    if line.starts_with("rename") {
        match line.split(' ').nth(2) {
            Some(path) => path,
            _ => "",
        }
        .to_string()
    } else {
        match line.split(' ').nth(1) {
            Some("/dev/null") => "/dev/null",
            Some(path) => {
                if git_diff_name {
                    &path[2..]
                } else {
                    path.split('\t').next().unwrap_or("")
                }
            }
            _ => "",
        }
        .to_string()
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

/// Given input like "diff --git a/src/main.rs b/src/main.rs"
/// return ("rs", "rs").
fn get_file_extensions_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
    let mut iter = line.split(' ').skip(2);
    (
        iter.next().and_then(|s| get_extension(&s[2..])),
        iter.next().and_then(|s| get_extension(&s[2..])),
    )
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
    fn test_get_file_extension_from_diff_line() {
        assert_eq!(
            get_file_extension_from_diff_line("diff --git a/src/main.rs b/src/main.rs"),
            Some("rs")
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
    fn test_get_file_path_from_git_file_meta_line() {
        assert_eq!(
            get_file_path_from_file_meta_line("--- a/src/delta.rs", true),
            "src/delta.rs"
        );
        assert_eq!(
            get_file_path_from_file_meta_line("+++ b/src/delta.rs", true),
            "src/delta.rs"
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
