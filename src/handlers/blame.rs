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
        let (previous_key, try_parse) = match &self.state {
            State::Blame(key) => (Some(key.clone()), true),
            State::Unknown => (None, true),
            _ => (None, false),
        };
        if try_parse {
            let line = self.line.to_owned();
            if let Some(blame) = parse_git_blame_line(&line, &self.config.blame_timestamp_format) {
                // Format blame metadata
                let format_data = format::parse_line_number_format(
                    &self.config.blame_format,
                    &*BLAME_PLACEHOLDER_REGEX,
                    false,
                );
                let mut formatted_blame_metadata =
                    format_blame_metadata(&format_data, &blame, self.config);
                let key = blame.commit;
                let is_repeat = previous_key.as_deref() == Some(key);
                if is_repeat {
                    formatted_blame_metadata =
                        " ".repeat(measure_text_width(&formatted_blame_metadata))
                };
                let metadata_style =
                    self.blame_metadata_style(&key, previous_key.as_deref(), is_repeat);
                let code_style = self.config.blame_code_style.unwrap_or(metadata_style);

                write!(
                    self.painter.writer,
                    "{}",
                    metadata_style.paint(&formatted_blame_metadata),
                )?;

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
                self.state = State::Blame(key);
                self.painter.syntax_highlight_and_paint_line(
                    &format!("{}\n", blame.code),
                    StyleSectionSpecifier::Style(code_style),
                    self.state.clone(),
                    BgShouldFill::default(),
                );
                handled_line = true
            }
        }
        Ok(handled_line)
    }

    fn blame_metadata_style(
        &mut self,
        key: &str,
        previous_key: Option<&str>,
        is_repeat: bool,
    ) -> Style {
        let mut style = match paint::parse_style_sections(&self.raw_line, self.config).first() {
            Some((style, _)) if style != &Style::default() => {
                // Something like `blame.coloring = highlightRecent` is in effect; honor
                // the color from git, subject to map-styles.
                *style
            }
            _ => {
                // Compute the color ourselves.
                let color = self.get_color(key, previous_key, is_repeat);
                // TODO: This will often be pointlessly updating a key with the
                // value it already has. It might be nicer to do this (and
                // compute the style) in get_color(), but as things stand the
                // borrow checker won't permit that.
                let style = Style::from_colors(
                    None,
                    color::parse_color(&color, true, self.config.git_config.as_ref()),
                );
                self.blame_key_colors.insert(key.to_owned(), color);
                style
            }
        };

        style.is_syntax_highlighted = true;
        style
    }

    fn get_color(&self, this_key: &str, previous_key: Option<&str>, is_repeat: bool) -> String {
        // Determine color for this line
        let previous_key_color = match previous_key {
            Some(previous_key) => self.blame_key_colors.get(previous_key),
            None => None,
        };

        match (
            self.blame_key_colors.get(this_key),
            previous_key_color,
            is_repeat,
        ) {
            (Some(key_color), Some(previous_key_color), true) => {
                debug_assert!(key_color == previous_key_color);
                // Repeated key: assign same color
                key_color.to_owned()
            }
            (None, Some(previous_key_color), false) => {
                // The key has no color: assign the next color that differs
                // from previous key.
                self.get_next_color(Some(previous_key_color))
            }
            (None, None, false) => {
                // The key has no color, and there is no previous key:
                // Just assign the next color. is_repeat is necessarily false.
                self.get_next_color(None)
            }
            (Some(key_color), Some(previous_key_color), false) => {
                if key_color != previous_key_color {
                    // Consecutive keys differ without a collision
                    key_color.to_owned()
                } else {
                    // Consecutive keys differ; prevent color collision
                    self.get_next_color(Some(key_color))
                }
            }
            (None, _, true) => delta_unreachable("is_repeat cannot be true when key has no color."),
            (Some(_), None, _) => {
                delta_unreachable("There must be a previous key if the key has a color.")
            }
        }
    }

    fn get_next_color(&self, other_than_color: Option<&str>) -> String {
        let n_keys = self.blame_key_colors.len();
        let n_colors = self.config.blame_palette.len();
        let color = self.config.blame_palette[n_keys % n_colors].clone();
        if Some(color.as_str()) != other_than_color {
            color
        } else {
            self.config.blame_palette[(n_keys + 1) % n_colors].clone()
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
//ea82f2d0 (Dan Davison       2021-08-22 18:20:19 -0700 120)             let mut handled_line = self.handle_key_meta_header_line()?

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
            s.push_str(&format::pad(
                &field,
                width,
                alignment_spec,
                placeholder.precision,
            ))
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

        // First key gets first color
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_key_colors),
            &[("aaaaaaa", "1")]
        );

        // Repeat key: same color
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_key_colors),
            &[("aaaaaaa", "1")]
        );

        // Second distinct key gets second color
        machine.line = blame_lines["B"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_key_colors),
            &[("aaaaaaa", "1"), ("bbbbbbb", "2")]
        );

        // Third distinct key gets first color (we only have 2 colors)
        machine.line = blame_lines["C"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_key_colors),
            &[("aaaaaaa", "1"), ("bbbbbbb", "2"), ("ccccccc", "1")]
        );

        // Now the first key appears again. It would get the first color, but
        // that would be a consecutive-key-color-collision. So it gets the
        // second color.
        machine.line = blame_lines["A"].into();
        machine.handle_blame_line().unwrap();
        assert_eq!(
            hashmap_items(&machine.blame_key_colors),
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
