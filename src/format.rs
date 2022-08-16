use std::convert::{TryFrom, TryInto};

use regex::Regex;
use smol_str::SmolStr;
use unicode_segmentation::UnicodeSegmentation;

use crate::features::side_by_side::ansifill::ODD_PAD_CHAR;

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FormatStringPlaceholderDataAnyPlaceholder<T> {
    pub prefix: SmolStr,
    pub prefix_len: usize,
    pub placeholder: Option<T>,
    pub alignment_spec: Option<Align>,
    pub width: Option<usize>,
    pub precision: Option<usize>,
    pub fmt_type: SmolStr,
    pub suffix: SmolStr,
    pub suffix_len: usize,
}

impl<T> Default for FormatStringPlaceholderDataAnyPlaceholder<T> {
    fn default() -> Self {
        Self {
            prefix: SmolStr::default(),
            prefix_len: 0,
            placeholder: None,
            alignment_spec: None,
            width: None,
            precision: None,
            fmt_type: SmolStr::default(),
            suffix: SmolStr::default(),
            suffix_len: 0,
        }
    }
}

impl<T> FormatStringPlaceholderDataAnyPlaceholder<T> {
    pub fn only_string(s: &str) -> Self {
        Self {
            suffix: s.into(),
            suffix_len: s.graphemes(true).count(),
            ..Self::default()
        }
    }
}

pub type FormatStringPlaceholderData<'a> =
    FormatStringPlaceholderDataAnyPlaceholder<Placeholder<'a>>;

pub type FormatStringSimple = FormatStringPlaceholderDataAnyPlaceholder<()>;

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
    pub fn into_simple(self) -> FormatStringSimple {
        FormatStringSimple {
            prefix: self.prefix,
            prefix_len: self.prefix_len,
            placeholder: None,
            alignment_spec: self.alignment_spec,
            width: self.width,
            precision: self.precision,
            fmt_type: self.fmt_type,
            suffix: self.suffix,
            suffix_len: self.suffix_len,
        }
    }
}

pub type FormatStringData<'a> = Vec<FormatStringPlaceholderData<'a>>;

pub fn make_placeholder_regex(labels: &[&str]) -> Regex {
    Regex::new(&format!(
        r"(?x)
    \{{
    ({})                             # 1: Placeholder labels
    (?:                              # Start optional format spec (non-capturing)
      :                              #     Literal colon
      (?:                            #     Start optional fill/alignment spec (non-capturing)
        ([^<^>])?                    #         2: Optional fill character (ignored)
        ([<^>])                      #         3: Alignment spec
      )?                             #
      (\d+)?                         #     4: Width (optional)
      (?:                            #     Start optional precision (non-capturing)
        \.(\d+)                      #         5: Precision
      )?                             #
      (?:                            #     Start optional format type (non-capturing)
        _?([A-Za-z][0-9A-Za-z_-]*)   #         6: Format type, optional leading _
      )?                             #
    )?                               #
    \}}
    ",
        labels.join("|")
    ))
    .unwrap()
}

// The resulting vector is never empty
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
            fmt_type: captures
                .get(6)
                .map(|m| SmolStr::from(m.as_str()))
                .unwrap_or_default(),
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
    // There is no such thing as "Center Align" with discrete terminal cells. In
    // some cases a decision has to be made whether to use the left or the right
    // cell, e.g. when centering one char in 4 cells: "_X__" or "__X_".
    //
    // The format!() center/^ default is center left, but when padding numbers
    // these are now aligned to the center right by having this trait return " "
    // instead of "". This is prepended to the format string. In the case of " "
    // the trailing " " must then be removed so everything is shifted to the right.
    // This assumes no special padding characters, i.e. the default of space.
    fn center_right_space(&self, alignment: Align, width: usize) -> &'static str;
}

impl CenterRightNumbers for &str {
    fn center_right_space(&self, _alignment: Align, _width: usize) -> &'static str {
        // Disables center-right formatting and aligns strings center-left
        ""
    }
}

impl CenterRightNumbers for String {
    fn center_right_space(&self, alignment: Align, width: usize) -> &'static str {
        self.as_str().center_right_space(alignment, width)
    }
}

impl<'a> CenterRightNumbers for &std::borrow::Cow<'a, str> {
    fn center_right_space(&self, alignment: Align, width: usize) -> &'static str {
        self.as_ref().center_right_space(alignment, width)
    }
}

// Returns the base-10 width of `n`, i.e. `floor(log10(n)) + 1` and 0 is treated as 1.
pub fn log10_plus_1(mut n: usize) -> usize {
    let mut len = 0;
    // log10 for integers is only in nightly and this is faster than
    // casting to f64 and back.
    loop {
        if n <= 9 {
            break len + 1;
        }
        if n <= 99 {
            break len + 2;
        }
        if n <= 999 {
            break len + 3;
        }
        if n <= 9999 {
            break len + 4;
        }

        len += 4;
        n /= 10000;
    }
}

impl CenterRightNumbers for usize {
    fn center_right_space(&self, alignment: Align, width: usize) -> &'static str {
        if alignment != Align::Center {
            return "";
        }

        let width_of_number = log10_plus_1(*self);
        if width > width_of_number && (width % 2 != width_of_number % 2) {
            " "
        } else {
            ""
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
    let space = s.center_right_space(alignment, width);
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
    if space == " " {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log10_plus_1() {
        let nrs = [
            1, 9, 10, 11, 99, 100, 101, 999, 1_000, 1_001, 9_999, 10_000, 10_001, 99_999, 100_000,
            100_001, 0,
        ];
        let widths = [1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 1];
        for (n, w) in nrs.iter().zip(widths.iter()) {
            assert_eq!(log10_plus_1(*n), *w);
        }

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(log10_plus_1(744_073_709_551_615), 5 * 3);
            assert_eq!(log10_plus_1(18_446_744_073_709_551_615), 2 + 6 * 3);
        }
    }

    #[test]
    fn test_center_right_space_trait() {
        assert_eq!("abc".center_right_space(Align::Center, 6), "");
        assert_eq!("abc".center_right_space(Align::Center, 7), "");
        assert_eq!(123.center_right_space(Align::Center, 6), " ");
        assert_eq!(123.center_right_space(Align::Center, 7), "");
    }

    #[test]
    fn test_pad_center_align() {
        assert_eq!(pad("abc", 6, Align::Center, None), " abc  ");
        assert_eq!(pad(1, 1, Align::Center, None), "1");
        assert_eq!(pad(1, 2, Align::Center, None), " 1");
        assert_eq!(pad(1, 3, Align::Center, None), " 1 ");
        assert_eq!(pad(1, 4, Align::Center, None), "  1 ");

        assert_eq!(pad(1001, 3, Align::Center, None), "1001");
        assert_eq!(pad(1001, 4, Align::Center, None), "1001");
        assert_eq!(pad(1001, 5, Align::Center, None), " 1001");

        assert_eq!(pad(1, 4, Align::Left, None), "1   ");
        assert_eq!(pad(1, 4, Align::Right, None), "   1");
        assert_eq!(pad("abc", 5, Align::Left, None), "abc  ");
        assert_eq!(pad("abc", 5, Align::Right, None), "  abc");
    }

    #[test]
    fn test_placeholder_with_notype() {
        let regex = make_placeholder_regex(&["placeholder"]);
        assert_eq!(
            parse_line_number_format("{placeholder:^4}", &regex, false),
            vec![FormatStringPlaceholderData {
                placeholder: Some(Placeholder::Str("placeholder")),
                alignment_spec: Some(Align::Center),
                width: Some(4),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_placeholder_with_only_type_dash_number() {
        let regex = make_placeholder_regex(&["placeholder"]);
        assert_eq!(
            parse_line_number_format("{placeholder:a_type-b-12}", &regex, false),
            vec![FormatStringPlaceholderData {
                placeholder: Some(Placeholder::Str("placeholder")),
                fmt_type: "a_type-b-12".into(),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_placeholder_with_empty_formatting() {
        let regex = make_placeholder_regex(&["placeholder"]);
        assert_eq!(
            parse_line_number_format("{placeholder:}", &regex, false),
            vec![FormatStringPlaceholderData {
                placeholder: Some(Placeholder::Str("placeholder")),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_placeholder_with_type_and_more() {
        let regex = make_placeholder_regex(&["placeholder"]);
        assert_eq!(
            parse_line_number_format("prefix {placeholder:<15.14type} suffix", &regex, false),
            vec![FormatStringPlaceholderData {
                prefix: "prefix ".into(),
                placeholder: Some(Placeholder::Str("placeholder")),
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                fmt_type: "type".into(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );

        assert_eq!(
            parse_line_number_format("prefix {placeholder:<15.14_type} suffix", &regex, false),
            vec![FormatStringPlaceholderData {
                prefix: "prefix ".into(),
                placeholder: Some(Placeholder::Str("placeholder")),
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                fmt_type: "type".into(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
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
                fmt_type: SmolStr::default(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
    }

    #[test]
    fn test_placeholder_regex_empty_placeholder() {
        let regex = make_placeholder_regex(&[""]);
        assert_eq!(
            parse_line_number_format("prefix {:<15.14} suffix", &regex, false),
            vec![FormatStringPlaceholderData {
                prefix: "prefix ".into(),
                placeholder: Some(Placeholder::Str("")),
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                fmt_type: SmolStr::default(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
    }
    #[test]
    fn test_format_string_simple() {
        let regex = make_placeholder_regex(&["foo"]);
        let f = parse_line_number_format("prefix {foo:<15.14} suffix", &regex, false);

        assert_eq!(
            f,
            vec![FormatStringPlaceholderData {
                prefix: "prefix ".into(),
                placeholder: Some(Placeholder::Str("foo")),
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                fmt_type: SmolStr::default(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
        let simple: Vec<_> = f
            .into_iter()
            .map(FormatStringPlaceholderData::into_simple)
            .collect();
        assert_eq!(
            simple,
            vec![FormatStringSimple {
                prefix: "prefix ".into(),
                placeholder: None,
                alignment_spec: Some(Align::Left),
                width: Some(15),
                precision: Some(14),
                fmt_type: SmolStr::default(),
                suffix: " suffix".into(),
                prefix_len: 7,
                suffix_len: 7,
            }]
        );
    }

    #[test]
    fn test_line_number_format_only_string() {
        let f = FormatStringSimple::only_string("abc");
        assert_eq!(f.suffix_len, 3);
    }

    #[test]
    fn test_parse_line_number_format_not_empty() {
        let regex = make_placeholder_regex(&["abc"]);
        assert!(!parse_line_number_format(" abc ", &regex, false).is_empty());
        assert!(!parse_line_number_format("", &regex, false).is_empty());
        let regex = make_placeholder_regex(&[""]);
        assert!(!parse_line_number_format(" abc ", &regex, false).is_empty());
        assert!(!parse_line_number_format("", &regex, false).is_empty());
    }
}
