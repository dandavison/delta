use chrono::{DateTime, FixedOffset};
use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::delta;
use crate::format;

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
