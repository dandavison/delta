use std::borrow::Cow;
use std::path::Path;

use unicode_segmentation::UnicodeSegmentation;

use super::draw;
use crate::config::Config;
use crate::delta::{Source, State, StateMachine};
use crate::features;
use crate::paint::Painter;

// https://git-scm.com/docs/git-config#Documentation/git-config.txt-diffmnemonicPrefix
const DIFF_PREFIXES: [&str; 6] = ["a/", "b/", "c/", "i/", "o/", "w/"];

#[derive(Debug, PartialEq)]
pub enum FileEvent {
    Change,
    Copy,
    Rename,
    ModeChange(String),
    NoEvent,
}

impl<'a> StateMachine<'a> {
    #[inline]
    fn test_file_meta_minus_line(&self) -> bool {
        (self.state == State::FileMeta || self.source == Source::DiffUnified)
            && (self.line.starts_with("--- ")
                || self.line.starts_with("rename from ")
                || self.line.starts_with("copy from ")
                || self.line.starts_with("old mode "))
    }

    pub fn handle_file_meta_minus_line(&mut self) -> std::io::Result<bool> {
        if !self.test_file_meta_minus_line() {
            return Ok(false);
        }
        let mut handled_line = false;

        let (path_or_mode, file_event) = parse_file_meta_line(
            &self.line,
            self.source == Source::GitDiff,
            if self.config.relative_paths {
                self.config.cwd_relative_to_repo_root.as_deref()
            } else {
                None
            },
        );
        // In the case of ModeChange only, the file path is taken from the diff
        // --git line (since that is the only place the file path occurs);
        // otherwise it is taken from the --- / +++ line.
        self.minus_file = if let FileEvent::ModeChange(_) = &file_event {
            get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or(path_or_mode)
        } else {
            path_or_mode
        };
        self.minus_file_event = file_event;

        if self.source == Source::DiffUnified {
            self.state = State::FileMeta;
            self.painter
                .set_syntax(get_file_extension_from_marker_line(&self.line));
        } else {
            self.painter
                .set_syntax(get_file_extension_from_file_meta_line_file_path(
                    &self.minus_file,
                ));
        }

        // In color_only mode, raw_line's structure shouldn't be changed.
        // So it needs to avoid fn _handle_file_meta_header_line
        // (it connects the plus_file and minus_file),
        // and to call fn handle_generic_file_meta_header_line directly.
        if self.config.color_only {
            write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
            handled_line = true;
        }
        Ok(handled_line)
    }

    #[inline]
    fn test_file_meta_plus_line(&self) -> bool {
        (self.state == State::FileMeta || self.source == Source::DiffUnified)
            && (self.line.starts_with("+++ ")
                || self.line.starts_with("rename to ")
                || self.line.starts_with("copy to ")
                || self.line.starts_with("new mode "))
    }

    pub fn handle_file_meta_plus_line(&mut self) -> std::io::Result<bool> {
        if !self.test_file_meta_plus_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        let (path_or_mode, file_event) = parse_file_meta_line(
            &self.line,
            self.source == Source::GitDiff,
            if self.config.relative_paths {
                self.config.cwd_relative_to_repo_root.as_deref()
            } else {
                None
            },
        );
        // In the case of ModeChange only, the file path is taken from the diff
        // --git line (since that is the only place the file path occurs);
        // otherwise it is taken from the --- / +++ line.
        self.plus_file = if let FileEvent::ModeChange(_) = &file_event {
            get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or(path_or_mode)
        } else {
            path_or_mode
        };
        self.plus_file_event = file_event;
        self.painter
            .set_syntax(get_file_extension_from_file_meta_line_file_path(
                &self.plus_file,
            ));
        self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));

        // In color_only mode, raw_line's structure shouldn't be changed.
        // So it needs to avoid fn _handle_file_meta_header_line
        // (it connects the plus_file and minus_file),
        // and to call fn handle_generic_file_meta_header_line directly.
        if self.config.color_only {
            write_generic_file_meta_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                self.config,
            )?;
            handled_line = true
        } else if self.should_handle()
            && self.handled_file_meta_header_line_file_pair != self.current_file_pair
        {
            self.painter.emit()?;
            self._handle_file_meta_header_line(self.source == Source::DiffUnified)?;
            self.handled_file_meta_header_line_file_pair = self.current_file_pair.clone()
        }
        Ok(handled_line)
    }

    /// Construct file change line from minus and plus file and write with FileMeta styling.
    fn _handle_file_meta_header_line(&mut self, comparing: bool) -> std::io::Result<()> {
        let line = get_file_change_description_from_file_paths(
            &self.minus_file,
            &self.plus_file,
            comparing,
            &self.minus_file_event,
            &self.plus_file_event,
            self.config,
        );
        // FIXME: no support for 'raw'
        write_generic_file_meta_header_line(&line, &line, &mut self.painter, self.config)
    }
}

/// Write `line` with FileMeta styling.
pub fn write_generic_file_meta_header_line(
    line: &str,
    raw_line: &str,
    painter: &mut Painter,
    config: &Config,
) -> std::io::Result<()> {
    // If file_style is "omit", we'll skip the process and print nothing.
    // However in the case of color_only mode,
    // we won't skip because we can't change raw_line structure.
    if config.file_style.is_omitted && !config.color_only {
        return Ok(());
    }
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(config.file_style.decoration_style);
    // Prints the new line below file-meta-line.
    // However in the case of color_only mode,
    // we won't print it because we can't change raw_line structure.
    if !config.color_only {
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        &config.decorations_width,
        config.file_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

#[allow(clippy::tabs_in_doc_comments)]
/// Given input like
/// "--- one.rs	2019-11-20 06:16:08.000000000 +0100"
/// Return "rs"
fn get_file_extension_from_marker_line(line: &str) -> Option<&str> {
    line.split('\t')
        .next()
        .and_then(|column| column.split(' ').nth(1))
        .and_then(|file| file.split('.').last())
}

fn get_file_extension_from_file_meta_line_file_path(path: &str) -> Option<&str> {
    if path.is_empty() || path == "/dev/null" {
        None
    } else {
        get_extension(path).map(|ex| ex.trim())
    }
}

/// Attempt to parse input as a file path and return extension as a &str.
pub fn get_extension(s: &str) -> Option<&str> {
    let path = Path::new(s);
    path.extension()
        .and_then(|e| e.to_str())
        // E.g. 'Makefile' is the file name and also the extension
        .or_else(|| path.file_name().and_then(|s| s.to_str()))
}

fn parse_file_meta_line(
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
fn get_repeated_file_path_from_diff_line(line: &str) -> Option<String> {
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
    match s.strip_suffix('\t').unwrap_or(s) {
        path if path == "/dev/null" => "/dev/null",
        path if git_diff_name && DIFF_PREFIXES.iter().any(|s| path.starts_with(s)) => &path[2..],
        path if git_diff_name => path,
        path => path.split('\t').next().unwrap_or(""),
    }
    .to_string()
}

pub fn get_file_change_description_from_file_paths(
    minus_file: &str,
    plus_file: &str,
    comparing: bool,
    minus_file_event: &FileEvent,
    plus_file_event: &FileEvent,
    config: &Config,
) -> String {
    let format_label = |label: &str| {
        if !label.is_empty() {
            format!("{} ", label)
        } else {
            "".to_string()
        }
    };
    if comparing {
        format!(
            "{}{} {} {}",
            format_label(&config.file_modified_label),
            minus_file,
            config.right_arrow,
            plus_file
        )
    } else {
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
                _ => format!(
                    "{}: {} {} {}",
                    plus_file, old_mode, config.right_arrow, new_mode
                ),
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
                "{}{} {} {}",
                format_label(match file_event {
                    FileEvent::Rename => &config.file_renamed_label,
                    FileEvent::Copy => &config.file_copied_label,
                    _ => &config.file_modified_label,
                }),
                format_file(minus_file),
                config.right_arrow,
                format_file(plus_file)
            ),
        }
    }
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
}
