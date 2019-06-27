use std::path::Path;

/// Given input like
/// "diff --git a/src/main.rs b/src/main.rs"
/// Return "rs", i.e. a single file extension consistent with both files.
pub fn get_file_extension_from_diff_line(line: &str) -> Option<&str> {
    match get_file_extensions_from_diff_line(line) {
        (Some(ext1), Some(ext2)) => {
            if ext1 == ext2 {
                Some(ext1)
            } else {
                // Unexpected: old and new files have different extensions.
                None
            }
        }
        (Some(ext1), None) => Some(ext1),
        (None, Some(ext2)) => Some(ext2),
        (None, None) => None,
    }
}

/// Given input like "diff --git a/src/main.rs b/src/main.rs"
/// return ("rs", "rs").
fn get_file_extensions_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
    let mut iter = line.split(" ");
    iter.next(); // diff
    iter.next(); // --git
    (
        iter.next().and_then(|s| get_extension(&s[2..])),
        iter.next().and_then(|s| get_extension(&s[2..])),
    )
}

/// Attempt to parse input as a file path and return extension as a &str.
fn get_extension(s: &str) -> Option<&str> {
    Path::new(s).extension().and_then(|e| e.to_str())
}
