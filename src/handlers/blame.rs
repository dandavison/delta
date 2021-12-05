use chrono::{DateTime, FixedOffset};
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

use crate::ansi::measure_text_width;
use crate::color;
use crate::config;
use crate::config::delta_unreachable;
use crate::delta::{self, State, StateMachine};
use crate::format::{self, Placeholder};
use crate::paint::{self, BgShouldFill, StyleSectionSpecifier};
use crate::style::Style;
use crate::utils;

impl<'a> StateMachine<'a> {
    /// If this is a line of git blame output then render it accordingly. If
    /// this is the first blame line, then set the syntax-highlighter language
    /// according to delta.default-language.
    pub fn handle_blame_line(&mut self) -> std::io::Result<bool> {
        // TODO: It should be possible to eliminate some of the .clone()s and
        // .to_owned()s.
        let mut handled_line = false;
        self.painter.emit()?;
        let (previous_commit, mut repeat_blame_line, try_parse) = match &self.state {
            State::Blame(commit, repeat_blame_line) => {
                (Some(commit.as_str()), repeat_blame_line.clone(), true)
            }
            State::Unknown => (None, None, true),
            _ => (None, None, false),
        };
        if try_parse {
            if let Some(blame) =
                parse_git_blame_line(&self.line, &self.config.blame_timestamp_format)
            {
                let is_repeat = previous_commit == Some(blame.commit);

                let mut style =
                    match paint::parse_style_sections(&self.raw_line, self.config).first() {
                        Some((style, _)) if style != &Style::default() => {
                            // Something like `blame.coloring = highlightRecent` is in effect; honor
                            // the color from git, subject to map-styles.
                            *style
                        }
                        _ => {
                            // Compute the color ourselves.
                            let color = self.get_color(blame.commit, previous_commit, is_repeat);
                            // TODO: This will often be pointlessly updating a key with the
                            // value it already has. It might be nicer to do this (and
                            // compute the style) in get_color(), but as things stand the
                            // borrow checker won't permit that.
                            let style = Style::from_colors(
                                None,
                                color::parse_color(&color, true, self.config.git_config.as_ref()),
                            );
                            self.blame_commit_colors
                                .insert(blame.commit.to_owned(), color);
                            style
                        }
                    };

                style.is_syntax_highlighted = true;

                // Construct commit metadata, paint, and emit
                let format_data = format::parse_line_number_format(
                    &self.config.blame_format,
                    &*BLAME_PLACEHOLDER_REGEX,
                    false,
                );
                let blame_line = match (is_repeat, &repeat_blame_line) {
                    (false, _) => Cow::from(format_blame_metadata(
                        &format_data,
                        &blame,
                        false,
                        self.config,
                    )),
                    (true, None) => {
                        repeat_blame_line = Some(format_blame_metadata(
                            &format_data,
                            &blame,
                            true,
                            self.config,
                        ));
                        Cow::from(repeat_blame_line.as_ref().unwrap())
                    }
                    (true, Some(repeat_blame_line)) => Cow::from(repeat_blame_line),
                };
                write!(self.painter.writer, "{}", style.paint(blame_line))?;

                // Emit syntax-highlighted code
                if matches!(self.state, State::Unknown) {
                    if let Some(lang) = utils::process::git_blame_filename_extension()
                        .as_ref()
                        .or_else(|| self.config.default_language.as_ref())
                    {
                        self.painter.set_syntax(Some(lang));
                        self.painter.set_highlighter();
                    }
                }
                self.state = State::Blame(blame.commit.to_owned(), repeat_blame_line.to_owned());
                self.painter.syntax_highlight_and_paint_line(
                    &format!("{}\n", blame.code),
                    StyleSectionSpecifier::Style(style),
                    self.state.clone(),
                    BgShouldFill::default(),
                );
                handled_line = true
            }
        }
        Ok(handled_line)
    }

    fn get_color(
        &self,
        this_commit: &str,
        previous_commit: Option<&str>,
        is_repeat: bool,
    ) -> String {
        // Determine color for this line
        let previous_commit_color = match previous_commit {
            Some(previous_commit) => self.blame_commit_colors.get(previous_commit),
            None => None,
        };

        match (
            self.blame_commit_colors.get(this_commit),
            previous_commit_color,
            is_repeat,
        ) {
            (Some(commit_color), Some(previous_commit_color), true) => {
                debug_assert!(commit_color == previous_commit_color);
                // Repeated commit: assign same color
                commit_color.to_owned()
            }
            (None, Some(previous_commit_color), false) => {
                // The commit has no color: assign the next color that differs
                // from previous commit.
                self.get_next_color(Some(previous_commit_color))
            }
            (None, None, false) => {
                // The commit has no color, and there is no previous commit:
                // Just assign the next color. is_repeat is necessarily false.
                self.get_next_color(None)
            }
            (Some(commit_color), Some(previous_commit_color), false) => {
                if commit_color != previous_commit_color {
                    // Consecutive commits differ without a collision
                    commit_color.to_owned()
                } else {
                    // Consecutive commits differ; prevent color collision
                    self.get_next_color(Some(commit_color))
                }
            }
            (None, _, true) => {
                delta_unreachable("is_repeat cannot be true when commit has no color.")
            }
            (Some(_), None, _) => {
                delta_unreachable("There must be a previous commit if the commit has a color.")
            }
        }
    }

    fn get_next_color(&self, other_than_color: Option<&str>) -> String {
        let n_commits = self.blame_commit_colors.len();
        let n_colors = self.config.blame_palette.len();
        let color = self.config.blame_palette[n_commits % n_colors].clone();
        if Some(color.as_str()) != other_than_color {
            color
        } else {
            self.config.blame_palette[(n_commits + 1) % n_colors].clone()
        }
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
    \^?[0-9a-f]{4,40} # commit hash (^ is 'boundary commit' marker)
)
(?: .+)?           # optional file name (unused; present if file has been renamed; TODO: inefficient?)
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
    let caps = BLAME_LINE_REGEX.captures(line)?;

    let commit = caps.get(1).unwrap().as_str();
    let author = caps.get(2).unwrap().as_str();
    let timestamp = caps.get(3).unwrap().as_str();

    let time = DateTime::parse_from_str(timestamp, timestamp_format).ok()?;

    let line_number = caps.get(4).unwrap().as_str().parse::<usize>().ok()?;

    let code = caps.get(5).unwrap().as_str();

    Some(BlameLine {
        commit,
        author,
        time,
        line_number,
        code,
    })
}

lazy_static! {
    pub static ref BLAME_PLACEHOLDER_REGEX: Regex =
        format::make_placeholder_regex(&["timestamp", "author", "commit"]);
}

pub fn format_blame_metadata(
    format_data: &[format::FormatStringPlaceholderData],
    blame: &BlameLine,
    is_repeat: bool,
    config: &config::Config,
) -> String {
    let mut s = String::new();
    let mut suffix = "";
    for placeholder in format_data {
        s.push_str(placeholder.prefix.as_str());

        let alignment_spec = placeholder
            .alignment_spec
            .as_ref()
            .unwrap_or(&format::Align::Left);
        let width = placeholder.width.unwrap_or(15);

        let pad = |s| format::pad(s, width, alignment_spec, placeholder.precision);
        let field = match placeholder.placeholder {
            Some(Placeholder::Str("timestamp")) => Some(Cow::from(
                chrono_humanize::HumanTime::from(blame.time).to_string(),
            )),
            Some(Placeholder::Str("author")) => Some(Cow::from(blame.author)),
            Some(Placeholder::Str("commit")) => Some(delta::format_raw_line(blame.commit, config)),
            None => None,
            _ => unreachable!("Unexpected `git blame` input"),
        };
        if let Some(field) = field {
            let field = pad(&field);
            if is_repeat {
                s.push_str(&" ".repeat(measure_text_width(&field)));
            } else {
                s.push_str(&field)
            }
        }
        suffix = placeholder.suffix.as_str();
    }
    s.push_str(suffix);
    s
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use std::{collections::HashMap, io::Cursor};

    use crate::tests::integration_test_utils;

    use super::*;

    #[test]
    fn test_blame_line_regex() {
        for line in &[
            "ea82f2d0 (Dan Davison       2021-08-22 18:20:19 -0700 120)             let mut handled_line = self.handle_commit_meta_header_line()?",
            "b2257cfa (Dan Davison  2020-07-18 15:34:43 -0400   1) use std::borrow::Cow;",
            "^35876eaa (Nicholas Marriott 2009-06-01 22:58:49 +0000   38) /* Default grid cell data. */",
        ] {
            let caps = BLAME_LINE_REGEX.captures(line);
            assert!(caps.is_some());
            assert!(parse_git_blame_line(line, "%Y-%m-%d %H:%M:%S %z").is_some());
        }
    }

    #[test]
    fn test_color_assignment() {
        let mut writer = Cursor::new(vec![0; 512]);
        let config = integration_test_utils::make_config_from_args(&["--blame-palette", "1 2"]);
        let mut machine = StateMachine::new(&mut writer, &config);

        let blame_lines: HashMap<&str, &str> = vec![
            (
                "A",
                "aaaaaaa (Dan Davison       2021-08-22 18:20:19 -0700 120) A",
            ),
            (
                "B",
                "bbbbbbb (Dan Davison  2020-07-18 15:34:43 -0400   1) B",
            ),
            (
                "C",
                "ccccccc (Dan Davison  2020-07-18 15:34:43 -0400   1) C",
            ),
        ]
        .into_iter()
        .collect();

        // First commit gets first color
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_commit_colors),
            &[("aaaaaaa", "1")]
        );

        // Repeat commit: same color
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_commit_colors),
            &[("aaaaaaa", "1")]
        );

        // Second distinct commit gets second color
        machine.line = blame_lines["B"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_commit_colors),
            &[("aaaaaaa", "1"), ("bbbbbbb", "2")]
        );

        // Third distinct commit gets first color (we only have 2 colors)
        machine.line = blame_lines["C"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_commit_colors),
            &[("aaaaaaa", "1"), ("bbbbbbb", "2"), ("ccccccc", "1")]
        );

        // Now the first commit appears again. It would get the first color, but
        // that would be a consecutive-commit-color-collision. So it gets the
        // second color.
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_commit_colors),
            &[("aaaaaaa", "2"), ("bbbbbbb", "2"), ("ccccccc", "1")]
        );
    }

    fn hashmap_items(hashmap: &HashMap<String, String>) -> Vec<(&str, &str)> {
        hashmap
            .iter()
            .sorted()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
