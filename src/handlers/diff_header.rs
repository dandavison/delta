use std::borrow::Cow;
use std::path::Path;

use unicode_segmentation::UnicodeSegmentation;

use super::draw;
use crate::config::Config;
use crate::delta::{DiffType, Source, State, StateMachine};
use crate::paint::Painter;
use crate::{features, utils};

// https://git-scm.com/docs/git-config#Documentation/git-config.txt-diffmnemonicPrefix
const DIFF_PREFIXES: [&str; 6] = ["a/", "b/", "c/", "i/", "o/", "w/"];

#[derive(Debug, PartialEq, Eq)]
pub enum FileEvent {
    Added,
    Change,
    Copy,
    Rename,
    Removed,
    NoEvent,
}

impl StateMachine<'_> {
    /// Check for the old mode|new mode lines and cache their info for later use.
    pub fn handle_diff_header_mode_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;
        if let Some(line_suf) = self.line.strip_prefix("old mode ") {
            self.state = State::DiffHeader(DiffType::Unified);
            if self.should_handle() && !self.config.color_only {
                self.mode_info = line_suf.to_string();
                handled_line = true;
            }
        } else if let Some(line_suf) = self.line.strip_prefix("new mode ") {
            self.state = State::DiffHeader(DiffType::Unified);
            if self.should_handle() && !self.config.color_only && !self.mode_info.is_empty() {
                self.mode_info = match (self.mode_info.as_str(), line_suf) {
                    // 100755 for executable and 100644 for non-executable are the only file modes Git records.
                    // https://medium.com/@tahteche/how-git-treats-changes-in-file-permissions-f71874ca239d
                    ("100644", "100755") => "mode +x".to_string(),
                    ("100755", "100644") => "mode -x".to_string(),
                    _ => format!(
                        "mode {} {} {}",
                        self.mode_info, self.config.right_arrow, line_suf
                    ),
                };
                handled_line = true;
            }
        }
        Ok(handled_line)
    }

    fn should_write_generic_diff_header_header_line(&mut self) -> std::io::Result<bool> {
        // In color_only mode, raw_line's structure shouldn't be changed.
        // So it needs to avoid fn _handle_diff_header_header_line
        // (it connects the plus_file and minus_file),
        // and to call fn handle_generic_diff_header_header_line directly.
        if self.config.color_only {
            write_generic_diff_header_header_line(
                &self.line,
                &self.raw_line,
                &mut self.painter,
                &mut self.mode_info,
                self.config,
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[inline]
    fn test_diff_header_minus_line(&self) -> bool {
        (matches!(self.state, State::DiffHeader(_)) || self.source == Source::DiffUnified)
            && ((self.line.starts_with("--- ") && self.minus_line_counter.three_dashes_expected())
                || self.line.starts_with("rename from ")
                || self.line.starts_with("copy from "))
    }

    /// Check for and handle the "--- filename ..." line.
    pub fn handle_diff_header_minus_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_minus_line() {
            return Ok(false);
        }

        let (mut path_or_mode, file_event) =
            parse_diff_header_line(&self.line, self.source == Source::GitDiff);

        utils::path::relativize_path_maybe(&mut path_or_mode, self.config);
        self.minus_file = path_or_mode;
        self.minus_file_event = file_event;

        if self.source == Source::DiffUnified {
            self.state = State::DiffHeader(DiffType::Unified);
            self.painter
                .set_syntax(get_filename_from_marker_line(&self.line));
        } else {
            self.painter
                .set_syntax(get_filename_from_diff_header_line_file_path(
                    &self.minus_file,
                ));
        }

        self.painter.paint_buffered_minus_and_plus_lines();
        self.should_write_generic_diff_header_header_line()
    }

    #[inline]
    fn test_diff_header_plus_line(&self) -> bool {
        (matches!(self.state, State::DiffHeader(_)) || self.source == Source::DiffUnified)
            && (self.line.starts_with("+++ ")
                || self.line.starts_with("rename to ")
                || self.line.starts_with("copy to "))
    }

    /// Check for and handle the "+++ filename ..." line.
    pub fn handle_diff_header_plus_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_plus_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        let (mut path_or_mode, file_event) =
            parse_diff_header_line(&self.line, self.source == Source::GitDiff);

        utils::path::relativize_path_maybe(&mut path_or_mode, self.config);
        self.plus_file = path_or_mode;
        self.plus_file_event = file_event;
        self.painter
            .set_syntax(get_filename_from_diff_header_line_file_path(
                &self.plus_file,
            ));
        self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));

        self.painter.paint_buffered_minus_and_plus_lines();
        if self.should_write_generic_diff_header_header_line()? {
            handled_line = true;
        } else if self.should_handle()
            && self.handled_diff_header_header_line_file_pair != self.current_file_pair
        {
            self.painter.emit()?;
            self._handle_diff_header_header_line(self.source == Source::DiffUnified)?;
            self.handled_diff_header_header_line_file_pair
                .clone_from(&self.current_file_pair);
        }
        Ok(handled_line)
    }

    #[inline]
    fn test_diff_header_file_operation_line(&self) -> bool {
        (matches!(self.state, State::DiffHeader(_)) || self.source == Source::DiffUnified)
            && (self.line.starts_with("deleted file mode ")
                || self.line.starts_with("new file mode "))
    }

    /// Check for and handle the "deleted file ..."  line.
    pub fn handle_diff_header_file_operation_line(&mut self) -> std::io::Result<bool> {
        if !self.test_diff_header_file_operation_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        let (_mode_info, file_event) =
            parse_diff_header_line(&self.line, self.source == Source::GitDiff);
        let name = get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or_default();
        match file_event {
            FileEvent::Removed => {
                self.minus_file = name;
                self.plus_file = "/dev/null".into();
                self.minus_file_event = FileEvent::Change;
                self.plus_file_event = FileEvent::Change;
                self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));
            }
            FileEvent::Added => {
                self.minus_file = "/dev/null".into();
                self.plus_file = name;
                self.minus_file_event = FileEvent::Change;
                self.plus_file_event = FileEvent::Change;
                self.current_file_pair = Some((self.minus_file.clone(), self.plus_file.clone()));
            }
            _ => (),
        }

        if self.should_write_generic_diff_header_header_line()?
            || (self.should_handle()
                && self.handled_diff_header_header_line_file_pair != self.current_file_pair)
        {
            handled_line = true;
        }
        Ok(handled_line)
    }

    /// Construct file change line from minus and plus file and write with DiffHeader styling.
    fn _handle_diff_header_header_line(&mut self, comparing: bool) -> std::io::Result<()> {
        let line = get_file_change_description_from_file_paths(
            self.config
                .minus_file
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(&self.minus_file),
            self.config
                .plus_file
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(&self.plus_file),
            comparing,
            &self.minus_file_event,
            &self.plus_file_event,
            self.config,
        );
        // FIXME: no support for 'raw'
        write_generic_diff_header_header_line(
            &line,
            &line,
            &mut self.painter,
            &mut self.mode_info,
            self.config,
        )
    }

    #[inline]
    fn test_pending_line_with_diff_name(&self) -> bool {
        matches!(self.state, State::DiffHeader(_)) || self.source == Source::DiffUnified
    }

    pub fn handle_pending_line_with_diff_name(&mut self) -> std::io::Result<()> {
        if !self.test_pending_line_with_diff_name() {
            return Ok(());
        }

        if !self.mode_info.is_empty() {
            let format_label = |label: &str| {
                if !label.is_empty() {
                    format!("{label} ")
                } else {
                    "".to_string()
                }
            };
            let format_file = |file| match (
                self.config.hyperlinks,
                utils::path::absolute_path(file, self.config),
            ) {
                (true, Some(absolute_path)) => features::hyperlinks::format_osc8_file_hyperlink(
                    absolute_path,
                    None,
                    file,
                    self.config,
                ),
                _ => Cow::from(file),
            };
            let label = format_label(&self.config.file_modified_label);
            let name = get_repeated_file_path_from_diff_line(&self.diff_line).unwrap_or_default();
            let line = format!("{}{}", label, format_file(&name));
            write_generic_diff_header_header_line(
                &line,
                &line,
                &mut self.painter,
                &mut self.mode_info,
                self.config,
            )
        } else if !self.config.color_only
            && self.should_handle()
            && self.handled_diff_header_header_line_file_pair != self.current_file_pair
        {
            self._handle_diff_header_header_line(self.source == Source::DiffUnified)?;
            self.handled_diff_header_header_line_file_pair
                .clone_from(&self.current_file_pair);
            Ok(())
        } else {
            Ok(())
        }
    }
}

/// Write `line` with DiffHeader styling.
pub fn write_generic_diff_header_header_line(
    line: &str,
    raw_line: &str,
    painter: &mut Painter,
    mode_info: &mut String,
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
    if !config.color_only {
        // Maintain 1-1 correspondence between input and output lines.
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        mode_info,
        &config.decorations_width,
        config.file_style,
        decoration_ansi_term_style,
    )?;
    if !mode_info.is_empty() {
        mode_info.truncate(0);
    }
    Ok(())
}

#[allow(clippy::tabs_in_doc_comments)]
/// Given input like
/// "--- a/zero/one.rs	2019-11-20 06:16:08.000000000 +0100"
/// Return "one.rs"
fn get_filename_from_marker_line(line: &str) -> Option<&str> {
    line.split('\t')
        .next()
        .and_then(|column| column.split(' ').nth(1))
        .and_then(get_filename_from_diff_header_line_file_path)
}

fn get_filename_from_diff_header_line_file_path(path: &str) -> Option<&str> {
    Path::new(path).file_name().and_then(|filename| {
        if path != "/dev/null" {
            filename.to_str()
        } else {
            None
        }
    })
}

fn parse_diff_header_line(line: &str, git_diff_name: bool) -> (String, FileEvent) {
    match line {
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
        line if line.starts_with("new file mode ") => {
            (line[14..].to_string(), FileEvent::Added) // "new file mode ".len()
        }
        line if line.starts_with("deleted file mode ") => {
            (line[18..].to_string(), FileEvent::Removed) // "deleted file mode ".len()
        }
        _ => ("".to_string(), FileEvent::NoEvent),
    }
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

fn remove_surrounding_quotes(path: &str) -> &str {
    if path.starts_with('"') && path.ends_with('"') {
        // Indexing into the UTF-8 string is safe because of the previous test
        &path[1..path.len() - 1]
    } else {
        path
    }
}

fn _parse_file_path(path: &str, git_diff_name: bool) -> String {
    // When git config 'core.quotepath = true' (the default), and `path` contains
    // non-ASCII characters, a backslash, or a quote; then it is quoted, so remove
    // these quotes. Characters may also be escaped, but these are left as-is.
    let path = remove_surrounding_quotes(path);
    // It appears that, if the file name contains a space, git appends a tab
    // character in the diff metadata lines, e.g.
    // $ git diff --no-index "a b" "c d" | cat -A
    // diff·--git·a/a·b·b/c·d␊
    // index·d00491f..0cfbf08·100644␊
    // ---·a/a·b├──┤␊
    // +++·b/c·d├──┤␊
    match path.strip_suffix('\t').unwrap_or(path) {
        "/dev/null" => "/dev/null",
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
            format!("{label} ")
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
            let formatted_file = if let Some(regex_replacement) = &config.file_regex_replacement {
                regex_replacement.execute(file)
            } else {
                Cow::from(file)
            };
            match (config.hyperlinks, utils::path::absolute_path(file, config)) {
                (true, Some(absolute_path)) => features::hyperlinks::format_osc8_file_hyperlink(
                    absolute_path,
                    None,
                    &formatted_file,
                    config,
                ),
                _ => formatted_file,
            }
        };
        match (minus_file, plus_file, minus_file_event, plus_file_event) {
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
            // minus_file_event == plus_file_event
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
    use crate::tests::integration_test_utils::{make_config_from_args, DeltaTest};
    use insta::assert_snapshot;

    #[test]
    fn test_get_filename_from_marker_line() {
        assert_eq!(
            get_filename_from_marker_line("--- src/one.rs	2019-11-20 06:47:56.000000000 +0100"),
            Some("one.rs")
        );
    }

    #[test]
    fn test_get_filename_from_diff_header_line() {
        assert_eq!(
            get_filename_from_diff_header_line_file_path("a/src/parse.rs"),
            Some("parse.rs")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("b/src/pa rse.rs"),
            Some("pa rse.rs")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("src/pa rse.rs"),
            Some("pa rse.rs")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("wat hello.rs"),
            Some("wat hello.rs")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("/dev/null"),
            None
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("Dockerfile"),
            Some("Dockerfile")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("Makefile"),
            Some("Makefile")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("a/src/Makefile"),
            Some("Makefile")
        );
        assert_eq!(
            get_filename_from_diff_header_line_file_path("src/Makefile"),
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
    fn test_get_file_path_from_git_diff_header_line() {
        assert_eq!(
            parse_diff_header_line("--- /dev/null", true),
            ("/dev/null".to_string(), FileEvent::Change)
        );
        for prefix in &DIFF_PREFIXES {
            assert_eq!(
                parse_diff_header_line(&format!("--- {prefix}src/delta.rs"), true),
                ("src/delta.rs".to_string(), FileEvent::Change)
            );
        }
        assert_eq!(
            parse_diff_header_line("--- src/delta.rs", true),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ src/delta.rs", true),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );

        assert_eq!(
            parse_diff_header_line("+++ \".\\delta.rs\"", true),
            (".\\delta.rs".to_string(), FileEvent::Change)
        );
    }

    #[test]
    fn test_get_file_path_from_git_diff_header_line_containing_spaces() {
        assert_eq!(
            parse_diff_header_line("+++ a/my src/delta.rs", true),
            ("my src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ my src/delta.rs", true),
            ("my src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ a/src/my delta.rs", true),
            ("src/my delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ a/my src/my delta.rs", true),
            ("my src/my delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ b/my src/my enough/my delta.rs", true),
            (
                "my src/my enough/my delta.rs".to_string(),
                FileEvent::Change
            )
        );
    }

    #[test]
    fn test_get_file_path_from_git_diff_header_line_rename() {
        assert_eq!(
            parse_diff_header_line("rename from nospace/file2.el", true),
            ("nospace/file2.el".to_string(), FileEvent::Rename)
        );
    }

    #[test]
    fn test_get_file_path_from_git_diff_header_line_rename_containing_spaces() {
        assert_eq!(
            parse_diff_header_line("rename from with space/file1.el", true),
            ("with space/file1.el".to_string(), FileEvent::Rename)
        );
    }

    #[test]
    fn test_parse_diff_header_line() {
        assert_eq!(
            parse_diff_header_line("--- src/delta.rs", false),
            ("src/delta.rs".to_string(), FileEvent::Change)
        );
        assert_eq!(
            parse_diff_header_line("+++ src/delta.rs", false),
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
            get_repeated_file_path_from_diff_line(
                "diff --git a/.config/Code - Insiders/User/settings.json b/.config/Code - Insiders/User/settings.json"),
            Some(".config/Code - Insiders/User/settings.json".to_string())
        );
        assert_eq!(
            get_repeated_file_path_from_diff_line(r#"diff --git "a/quoted" "b/quoted""#),
            Some("quoted".to_string())
        );
    }

    pub const BIN_AND_TXT_FILE_ADDED: &str = "\
diff --git a/BIN b/BIN
new file mode 100644
index 0000000..a5d0c46
Binary files /dev/null and b/BIN differ
diff --git a/TXT b/TXT
new file mode 100644
index 0000000..323fae0
--- /dev/null
+++ b/TXT
@@ -0,0 +1 @@
+plain text";

    #[test]
    fn test_diff_header_relative_paths() {
        // rustfmt ignores the assert macro arguments, so do the setup outside
        let mut cfg = make_config_from_args(&["--relative-paths", "-s"]);
        cfg.cwd_relative_to_repo_root = Some("src/utils/".into());
        let result = DeltaTest::with_config(&cfg)
            .with_input(BIN_AND_TXT_FILE_ADDED)
            .output;
        // convert windows '..\' to unix '../' paths
        insta::with_settings!({filters => vec![(r"\.\.\\", "../")]}, {
            assert_snapshot!(result, @r###"

            added: ../../BIN (binary file)
            ───────────────────────────────────────────

            added: ../../TXT
            ───────────────────────────────────────────

            ───┐
            1: │
            ───┘
            │    │                │  1 │plain text
            "###)
        });
    }

    pub const DIFF_AMBIGUOUS_HEADER_3X_MINUS: &str = r#"--- a.lua
+++ b.lua
@@ -1,5 +1,4 @@
 #!/usr/bin/env lua
 
 print("Hello")
--- World?
 print("..")
"#;
    pub const DIFF_AMBIGUOUS_HEADER_3X_MINUS_LAST_LINE: &str = r#"--- c.lua
+++ d.lua
@@ -1,4 +1,3 @@
 #!/usr/bin/env lua
 
 print("Hello")
--- World?
"#;

    pub const DIFF_AMBIGUOUS_HEADER_MULTIPLE_HUNKS: &str = r#"--- e.lua	2024-08-04 20:50:27.257726606 +0200
+++ f.lua	2024-08-04 20:50:35.345795405 +0200
@@ -3,3 +3,2 @@
 print("Hello")
--- World?
 print("")
@@ -7,2 +6,3 @@
 print("")
+print("World")
 print("")
@@ -10,2 +10 @@
 print("")
--- End
"#;

    #[test]
    fn test_diff_header_ambiguous_3x_minus() {
        // check ansi output to ensure output is highlighted
        let result = DeltaTest::with_args(&[])
            .explain_ansi()
            .with_input(DIFF_AMBIGUOUS_HEADER_3X_MINUS);

        assert_snapshot!(result.output, @r###"
        (normal)
        (blue)a.lua ⟶   b.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)1(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (203)#(231)!(203)/(231)usr(203)/(231)bin(203)/(231)env lua(normal)

        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        (81)print(231)((186)".."(231))(normal)

        "###);
    }

    #[test]
    fn test_diff_header_ambiguous_3x_minus_concatenated() {
        let result = DeltaTest::with_args(&[])
            .explain_ansi()
            .with_input(&format!(
                "{}{}{}",
                DIFF_AMBIGUOUS_HEADER_MULTIPLE_HUNKS,
                DIFF_AMBIGUOUS_HEADER_3X_MINUS,
                DIFF_AMBIGUOUS_HEADER_3X_MINUS_LAST_LINE
            ));

        assert_snapshot!(result.output, @r###"
        (normal)
        (blue)e.lua ⟶   f.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)3(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        (81)print(231)((186)""(231))(normal)

        (blue)───(blue)┐(normal)
        (blue)6(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (81)print(231)((186)""(231))(normal)
        (81 22)print(231)((186)"World"(231))(normal)
        (81)print(231)((186)""(231))(normal)

        (blue)────(blue)┐(normal)
        (blue)10(normal): (blue)│(normal)
        (blue)────(blue)┘(normal)
        (81)print(231)((186)""(231))(normal)
        (normal 52)-- End(normal)

        (blue)a.lua ⟶   b.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)1(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (203)#(231)!(203)/(231)usr(203)/(231)bin(203)/(231)env lua(normal)

        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        (81)print(231)((186)".."(231))(normal)

        (blue)c.lua ⟶   d.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)1(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (203)#(231)!(203)/(231)usr(203)/(231)bin(203)/(231)env lua(normal)

        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        "###);
    }

    #[test]
    fn test_diff_header_ambiguous_3x_minus_extra_and_concatenated() {
        let result = DeltaTest::with_args(&[])
            .explain_ansi()
            .with_input(&format!(
                "extra 1\n\n{}\nextra 2\n{}\nextra 3\n{}",
                DIFF_AMBIGUOUS_HEADER_MULTIPLE_HUNKS,
                DIFF_AMBIGUOUS_HEADER_3X_MINUS,
                DIFF_AMBIGUOUS_HEADER_3X_MINUS_LAST_LINE
            ));

        assert_snapshot!(result.output, @r###"
        (normal)extra 1


        (blue)e.lua ⟶   f.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)3(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        (81)print(231)((186)""(231))(normal)

        (blue)───(blue)┐(normal)
        (blue)6(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (81)print(231)((186)""(231))(normal)
        (81 22)print(231)((186)"World"(231))(normal)
        (81)print(231)((186)""(231))(normal)

        (blue)────(blue)┐(normal)
        (blue)10(normal): (blue)│(normal)
        (blue)────(blue)┘(normal)
        (81)print(231)((186)""(231))(normal)
        (normal 52)-- End(normal)

        extra 2

        (blue)a.lua ⟶   b.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)1(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (203)#(231)!(203)/(231)usr(203)/(231)bin(203)/(231)env lua(normal)

        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        (81)print(231)((186)".."(231))(normal)

        extra 3

        (blue)c.lua ⟶   d.lua(normal)
        (blue)───────────────────────────────────────────(normal)

        (blue)───(blue)┐(normal)
        (blue)1(normal): (blue)│(normal)
        (blue)───(blue)┘(normal)
        (203)#(231)!(203)/(231)usr(203)/(231)bin(203)/(231)env lua(normal)

        (81)print(231)((186)"Hello"(231))(normal)
        (normal 52)-- World?(normal)
        "###);
    }
}
