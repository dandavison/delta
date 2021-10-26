use std::convert::{TryFrom, TryInto};

use regex::Regex;
use smol_str::SmolStr;
use unicode_segmentation::UnicodeSegmentation;

use crate::features::side_by_side::ansifill::ODD_PAD_CHAR;

#[derive(Debug, PartialEq)]
pub enum Placeholder<'a> {
    NumberMinus,
    NumberPlus,
    Str(&'a str),
}

impl<'a> TryFrom<Option<&'a str>> for Placeholder<'a> {
    type Error = ();
    fn try_from(from: Option<&'a str>) -> Result<Self, Self::Error> {
        match from {
            Some(placeholder) if placeholder == "nm" => Ok(Placeholder::NumberMinus),
            Some(placeholder) if placeholder == "np" => Ok(Placeholder::NumberPlus),
            Some(placeholder) => Ok(Placeholder::Str(placeholder)),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl TryFrom<Option<&str>> for Align {
    type Error = ();
    fn try_from(from: Option<&str>) -> Result<Self, Self::Error> {
        match from {
            Some(alignment) if alignment == "<" => Ok(Align::Left),
            Some(alignment) if alignment == ">" => Ok(Align::Right),
            Some(alignment) if alignment == "^" => Ok(Align::Center),
            Some(alignment) => {
                debug_assert!(false, "Unknown Alignment: {}", alignment);
                Err(())
            }
            None => Err(()),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct FormatStringPlaceholderData<'a> {
    pub prefix: SmolStr,
    pub prefix_len: usize,
    pub placeholder: Option<Placeholder<'a>>,
    pub alignment_spec: Option<Align>,
    pub width: Option<usize>,
    pub suffix: SmolStr,
    pub suffix_len: usize,
}

impl<'a> FormatStringPlaceholderData<'a> {
    pub fn width(&self, hunk_max_line_number_width: usize) -> (usize, usize) {
        // Only if Some(placeholder) is present will there be a number formatted
        // by this placeholder, if not width is also None.
        (
            self.prefix_len
                + std::cmp::max(
                    self.placeholder
                        .as_ref()
                        .map_or(0, |_| hunk_max_line_number_width),
                    self.width.unwrap_or(0),
                ),
            self.suffix_len,
        )
    }
}

pub type FormatStringData<'a> = Vec<FormatStringPlaceholderData<'a>>;

pub fn make_placeholder_regex(labels: &[&str]) -> Regex {
    Regex::new(&format!(
        r"(?x)
    \{{
    ({})            # 1: Placeholder labels
    (?:             # Start optional format spec (non-capturing)
      :             #     Literal colon
      (?:           #     Start optional fill/alignment spec (non-capturing)
        ([^<^>])?   #         2: Optional fill character (ignored)
        ([<^>])     #         3: Alignment spec
      )?            #
      (\d+)         #     4: Width
    )?              #
    \}}
    ",
        labels.join("|")
    ))
    .unwrap()
}

pub fn parse_line_number_format<'a>(
    format_string: &'a str,
    placeholder_regex: &Regex,
    mut prefix_with_space: bool,
) -> FormatStringData<'a> {
    let mut format_data = Vec::new();
    let mut offset = 0;

    let mut expand_first_prefix = |prefix: SmolStr| {
        // Only prefix the first placeholder with a space, also see `UseFullPanelWidth`
        if prefix_with_space {
            let prefix = SmolStr::new(format!("{}{}", ODD_PAD_CHAR, prefix));
            prefix_with_space = false;
            prefix
        } else {
            prefix
        }
    };

    for captures in placeholder_regex.captures_iter(format_string) {
        let match_ = captures.get(0).unwrap();
        let prefix = SmolStr::new(&format_string[offset..match_.start()]);
        let prefix = expand_first_prefix(prefix);
        let prefix_len = prefix.graphemes(true).count();
        let suffix = SmolStr::new(&format_string[match_.end()..]);
        let suffix_len = suffix.graphemes(true).count();
        format_data.push(FormatStringPlaceholderData {
            prefix,
            prefix_len,
            placeholder: captures.get(1).map(|m| m.as_str()).try_into().ok(),
            alignment_spec: captures.get(3).map(|m| m.as_str()).try_into().ok(),
            width: captures.get(4).map(|m| {
                m.as_str()
                    .parse()
                    .unwrap_or_else(|_| panic!("Invalid width in format string: {}", format_string))
            }),
            suffix,
            suffix_len,
        });
        offset = match_.end();
    }
    if offset == 0 {
        let prefix = SmolStr::new("");
        let prefix = expand_first_prefix(prefix);
        let prefix_len = prefix.graphemes(true).count();
        // No placeholders
        format_data.push(FormatStringPlaceholderData {
            prefix,
            prefix_len,
            suffix: SmolStr::new(format_string),
            suffix_len: format_string.graphemes(true).count(),
            ..Default::default()
        })
    }

    format_data
}

pub fn pad(s: &str, width: usize, alignment: &Align) -> String {
    match alignment {
        Align::Left => format!("{0:<1$}", s, width),
        Align::Center => format!("{0:^1$}", s, width),
        Align::Right => format!("{0:>1$}", s, width),
    }
}
