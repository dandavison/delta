//! User-defined glob → syntax-name mappings.
//!
//! Inspired by `bat`'s `--map-syntax` option. Each entry maps a glob pattern
//! (matched against the full path and the file name) to a syntect syntax name
//! such as `"Bash"` or `"Git Config"`.
//!
//! Later inserts take precedence over earlier ones.

use std::path::Path;

use globset::{Candidate, GlobBuilder, GlobMatcher};

/// Error type for [`SyntaxMapping::insert`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxMappingError {
    /// The supplied `<glob>:<syntax-name>` string did not contain exactly one
    /// `:` separator.
    InvalidFormat(String),
    /// The glob pattern could not be compiled.
    InvalidGlob {
        glob: String,
        message: String,
    },
}

impl std::fmt::Display for SyntaxMappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxMappingError::InvalidFormat(s) => write!(
                f,
                "expected '<glob-pattern>:<syntax-name>' (e.g. '*.cpp:C++'), got '{s}'"
            ),
            SyntaxMappingError::InvalidGlob { glob, message } => {
                write!(f, "invalid glob pattern '{glob}': {message}")
            }
        }
    }
}

impl std::error::Error for SyntaxMappingError {}

/// A collection of user-supplied glob → syntax-name mappings.
///
/// Mappings are stored in insertion order; the first match wins during lookup.
#[derive(Debug, Default, Clone)]
pub struct SyntaxMapping {
    custom_mappings: Vec<(GlobMatcher, String)>,
}

impl SyntaxMapping {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse and insert a `<glob>:<syntax-name>` entry.
    ///
    /// Returns [`SyntaxMappingError::InvalidFormat`] if `entry` does not
    /// contain exactly one `:` separator (matching `bat`'s behavior), or
    /// [`SyntaxMappingError::InvalidGlob`] if the glob fails to compile.
    pub fn insert(&mut self, entry: &str) -> Result<(), SyntaxMappingError> {
        let (glob, syntax_name) = match entry.split(':').collect::<Vec<_>>().as_slice() {
            [g, s] if !g.is_empty() && !s.is_empty() => (g.to_string(), s.to_string()),
            _ => return Err(SyntaxMappingError::InvalidFormat(entry.to_owned())),
        };
        let matcher = GlobBuilder::new(&glob)
            .case_insensitive(true)
            .literal_separator(true)
            .build()
            .map_err(|e| SyntaxMappingError::InvalidGlob {
                glob: glob.clone(),
                message: e.to_string(),
            })?
            .compile_matcher();
        self.custom_mappings.push((matcher, syntax_name));
        Ok(())
    }

    /// Return the syntax name registered for `path`, if any.
    ///
    /// Both the full path and the file-name component are tested against each
    /// pattern; the first match (in insertion order) wins.
    pub fn get_syntax_for(&self, path: impl AsRef<Path>) -> Option<&str> {
        let path = path.as_ref();
        let candidate = Candidate::new(path);
        let candidate_filename = path.file_name().map(Candidate::new);
        for (matcher, syntax) in &self.custom_mappings {
            if matcher.is_match_candidate(&candidate)
                || candidate_filename
                    .as_ref()
                    .is_some_and(|f| matcher.is_match_candidate(f))
            {
                return Some(syntax.as_str());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn empty_returns_none() {
        let m = SyntaxMapping::new();
        assert_eq!(m.get_syntax_for("/path/to/Cargo.lock"), None);
    }

    #[test]
    fn custom_mappings_work() {
        let mut m = SyntaxMapping::new();
        m.insert("/path/to/Cargo.lock:TOML").unwrap();
        m.insert("/path/to/.ignore:Git Ignore").unwrap();

        assert_eq!(m.get_syntax_for("/path/to/Cargo.lock"), Some("TOML"));
        assert_eq!(m.get_syntax_for("/path/to/other.lock"), None);
        assert_eq!(m.get_syntax_for("/path/to/.ignore"), Some("Git Ignore"));
    }

    #[test]
    fn first_insert_wins() {
        let mut m = SyntaxMapping::new();
        m.insert("/path/to/foo:alpha").unwrap();
        m.insert("/path/to/foo:bravo").unwrap();
        assert_eq!(m.get_syntax_for("/path/to/foo"), Some("alpha"));
    }

    #[test]
    fn match_is_case_insensitive() {
        let mut m = SyntaxMapping::new();
        m.insert("MY_SPECIAL_FILE:Python").unwrap();
        assert_eq!(m.get_syntax_for("/path/to/my_special_file"), Some("Python"));
        assert_eq!(m.get_syntax_for("/path/to/MY_SPECIAL_FILE"), Some("Python"));
    }

    #[rstest]
    #[case::extension_in_subdir("*.ino:C++", "project/src/blink.ino", Some("C++"))]
    #[case::extension_in_root("*.ino:C++", "project/blink.ino", Some("C++"))]
    #[case::extension_unrelated("*.ino:C++", "project/blink.cpp", None)]
    #[case::filename_in_subdir("**/vimrc:VimL", "home/user/vimrc", Some("VimL"))]
    #[case::filename_at_root("**/vimrc:VimL", "vimrc", Some("VimL"))]
    fn lookup_resolves_glob(
        #[case] entry: &str,
        #[case] path: &str,
        #[case] expected: Option<&str>,
    ) {
        let mut m = SyntaxMapping::new();
        m.insert(entry).unwrap();
        assert_eq!(m.get_syntax_for(path), expected);
    }

    #[rstest]
    #[case::missing_colon("just-a-glob")]
    #[case::empty_glob(":Bash")]
    #[case::empty_syntax("*.foo:")]
    #[case::multiple_colons("*.foo:Bar:Baz")]
    fn invalid_format_rejected(#[case] entry: &str) {
        let mut m = SyntaxMapping::new();
        assert!(matches!(m.insert(entry), Err(SyntaxMappingError::InvalidFormat(_))));
    }

    #[test]
    fn invalid_glob_rejected() {
        let mut m = SyntaxMapping::new();
        assert!(matches!(
            m.insert("[unclosed:Bash"),
            Err(SyntaxMappingError::InvalidGlob { .. })
        ));
    }
}
