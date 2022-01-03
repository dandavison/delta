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

#[derive(Debug, Copy, Clone, PartialEq)]
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
    pub precision: Option<usize>,
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
      (?:           #     Start optional precision (non-capturing)
        \.(\d+)     #         5: Precision
      )?            #
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
            precision: captures.get(5).map(|m| {
                m.as_str().parse().unwrap_or_else(|_| {
                    panic!("Invalid precision in format string: {}", format_string)
                })
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

pub trait CenterRightNumbers {
    fn width_for_center_right(&self) -> usize;
}

impl CenterRightNumbers for &str {
    fn width_for_center_right(&self) -> usize {
        // Disables the center-right formatting and aligns strings center-left
        usize::MAX
    }
}

impl CenterRightNumbers for String {
    fn width_for_center_right(&self) -> usize {
        self.as_str().width_for_center_right()
    }
}

impl<'a> CenterRightNumbers for &std::borrow::Cow<'a, str> {
    fn width_for_center_right(&self) -> usize {
        self.as_ref().width_for_center_right()
    }
}

impl CenterRightNumbers for usize {
    fn width_for_center_right(&self) -> usize {
        // log10 for integers is only in nightly and this is faster than
        // casting to f64 and back.
        let mut n = *self;
        let mut len = 1;
        loop {
            if n <= 9 {
                break len;
            }
            len += 1;
            n /= 10;
        }
    }
}

// Note that in this case of a string `s`, `precision` means "max width".
// See https://doc.rust-lang.org/std/fmt/index.html
pub fn pad<T: std::fmt::Display + CenterRightNumbers>(
    s: T,
    width: usize,
    alignment: Align,
    precision: Option<usize>,
) -> String {
    let center_left_to_right_align_fix = || {
        let q = s.width_for_center_right();
        width > q && (width % 2 != q % 2)
    };
    let space = if alignment == Align::Center && center_left_to_right_align_fix() {
        " "
    } else {
        ""
    };
    let mut result = match precision {
        None => match alignment {
            Align::Left => format!("{0}{1:<2$}", space, s, width),
            Align::Center => format!("{0}{1:^2$}", space, s, width),
            Align::Right => format!("{0}{1:>2$}", space, s, width),
        },
        Some(precision) => match alignment {
            Align::Left => format!("{0}{1:<2$.3$}", space, s, width, precision),
            Align::Center => format!("{0}{1:^2$.3$}", space, s, width, precision),
            Align::Right => format!("{0}{1:>2$.3$}", space, s, width, precision),
        },
    };
    if !space.is_empty() {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_width_trait() {
        dbg!("asdf".to_string().width_for_center_right());
        dbg!(3_usize.width_for_center_right());
        dbg!(99_999_usize.width_for_center_right());
        dbg!(100_000_usize.width_for_center_right());
        dbg!(100_003_usize.width_for_center_right());
        dbg!(700_003_usize.width_for_center_right());
        dbg!(999_999_usize.width_for_center_right());
        dbg!(1_000_000_usize.width_for_center_right());
        dbg!(1_000_001_usize.width_for_center_right());
        dbg!(9876654321_usize.width_for_center_right());
    }

    #[test]
    fn test_pad_center_align() {
        for i in (1..1001_usize)
            .into_iter()
            .filter(|&i| i < 20 || (i > 90 && i < 120) || i > 990)
        {
            println!(
                "string: │{}│     num: │{}│",
                pad(i.to_string(), 4, Align::Center, None),
                pad(i, 4, Align::Center, None),
            );
        }
    }

    #[test]
    fn test_placeholder_regex() {
        let regex = make_placeholder_regex(&["placeholder"]);
        assert_eq!(
            parse_line_number_format("prefix {placeholder:<15.14} suffix", &regex, false),
            vec![FormatStringPlaceholderData {
                prefix: "prefix ".into(),
                placeholder: Some(Placeholder::Str("placeholder")),
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
    }
}
