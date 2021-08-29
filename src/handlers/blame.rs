use chrono::{DateTime, FixedOffset};
use lazy_static::lazy_static;
use regex::Regex;

use crate::color;
use crate::config;
use crate::delta::{self, State, StateMachine};
use crate::format;
use crate::style::Style;

impl<'a> StateMachine<'a> {
    /// If this is a line of git blame output then render it accordingly. If
    /// this is the first blame line, then set the syntax-highlighter language
    /// according to delta.default-language.
    pub fn handle_blame_line(&mut self) -> std::io::Result<bool> {
        let mut handled_line = false;
        self.painter.emit()?;
        if matches!(self.state, State::Unknown | State::Blame(_)) {
            if let Some(blame) =
                parse_git_blame_line(&self.line, &self.config.blame_timestamp_format)
            {
                // Determine color for this line
                let color = if let Some(color) = self.blame_commit_colors.get(blame.commit) {
                    color
                } else {
                    let n_commits = self.blame_commit_colors.len();
                    let n_colors = self.config.blame_palette.len();
                    let new_color = &self.config.blame_palette[(n_commits + 1) % n_colors];
                    self.blame_commit_colors
                        .insert(blame.commit.to_owned(), new_color.to_owned());
                    new_color
                };
                let mut style = Style::from_colors(None, color::parse_color(color, true));
                style.is_syntax_highlighted = true;

                // Construct commit metadata, paint, and emit
                let format_data = format::parse_line_number_format(
                    &self.config.blame_format,
                    &*BLAME_PLACEHOLDER_REGEX,
                );
                write!(
                    self.painter.writer,
                    "{}",
                    style.paint(format_blame_metadata(&format_data, &blame, self.config))
                )?;

                // Emit syntax-highlighted code
                if matches!(self.state, State::Unknown) {
                    if let Some(lang) = self.config.default_language.as_ref() {
                        self.painter.set_syntax(Some(lang));
                        self.painter.set_highlighter();
                    }
                    self.state = State::Blame(blame.commit.to_owned());
                }
                self.painter.syntax_highlight_and_paint_line(
                    blame.code,
                    style,
                    self.state.clone(),
                    true,
                );
                handled_line = true
            }
        }
        Ok(handled_line)
    }
}

#[derive(Debug)]
pub struct BlameLine<'a> {
    pub commit: &'a str,
    pub author: &'a str,
    pub time: DateTime<FixedOffset>,
    pub line_number: usize,
    pub code: &'a str,
}

// E.g.
//ea82f2d0 (Dan Davison       2021-08-22 18:20:19 -0700 120)             let mut handled_line = self.handle_commit_meta_header_line()?

lazy_static! {
    static ref BLAME_LINE_REGEX: Regex = Regex::new(
        r"(?x)
^
(
    [0-9a-f]{8}    # commit hash
)
[\ ]
\(                 # open (
(
    [^\ ].*[^\ ]   # author name
)
[\ ]+
(                  # timestamp
    [0-9]{4}-[0-9]{2}-[0-9]{2}\ [0-9]{2}:[0-9]{2}:[0-9]{2}\ [-+][0-9]{4}
)
[\ ]+
(
    [0-9]+        # line number
)
\)                # close )
(
    .*            # code, with leading space
)
$
"
    )
    .unwrap();
}

pub fn parse_git_blame_line<'a>(line: &'a str, timestamp_format: &str) -> Option<BlameLine<'a>> {
    if let Some(caps) = BLAME_LINE_REGEX.captures(line) {
        let commit = caps.get(1).unwrap().as_str();
        let author = caps.get(2).unwrap().as_str();
        let timestamp = caps.get(3).unwrap().as_str();
        if let Ok(time) = DateTime::parse_from_str(timestamp, timestamp_format) {
            let line_number_str = caps.get(4).unwrap().as_str();
            if let Ok(line_number) = line_number_str.parse::<usize>() {
                let code = caps.get(5).unwrap().as_str();
                Some(BlameLine {
                    commit,
                    author,
                    time,
                    line_number,
                    code,
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

lazy_static! {
    pub static ref BLAME_PLACEHOLDER_REGEX: Regex =
        format::make_placeholder_regex(&["timestamp", "author", "commit"]);
}

pub fn format_blame_metadata(
    format_data: &[format::FormatStringPlaceholderData],
    blame: &BlameLine,
    config: &config::Config,
) -> String {
    let mut s = String::new();
    let mut suffix = "";
    for placeholder in format_data {
        s.push_str(placeholder.prefix);

        let alignment_spec = placeholder.alignment_spec.unwrap_or("<");
        let width = placeholder.width.unwrap_or(15);

        let pad = |s| format::pad(s, width, alignment_spec);
        match placeholder.placeholder {
            Some("timestamp") => s.push_str(&pad(
                &chrono_humanize::HumanTime::from(blame.time).to_string()
            )),
            Some("author") => s.push_str(&pad(blame.author)),
            Some("commit") => s.push_str(&pad(&delta::format_raw_line(blame.commit, config))),
            None => {}
            Some(_) => unreachable!(),
        }
        suffix = placeholder.suffix;
    }
    s.push_str(suffix);
    s
}

#[test]
fn test_blame_line_regex() {
    for line in &[
        "ea82f2d0 (Dan Davison       2021-08-22 18:20:19 -0700 120)             let mut handled_line = self.handle_commit_meta_header_line()?",
        "b2257cfa (Dan Davison  2020-07-18 15:34:43 -0400   1) use std::borrow::Cow;"
    ] {
        let caps = BLAME_LINE_REGEX.captures(line);
        assert!(caps.is_some());
        assert!(parse_git_blame_line(line, "%Y-%m-%d %H:%M:%S %z").is_some());
    }
}
