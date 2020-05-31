use std::process;

use ansi_term;

use crate::cli::unreachable;
use crate::color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub ansi_term_style: ansi_term::Style,
    pub is_emph: bool,
    pub is_omitted: bool,
    pub is_raw: bool,
    pub is_syntax_highlighted: bool,
    pub decoration_style: DecorationStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecorationStyle {
    Box(ansi_term::Style),
    Underline(ansi_term::Style),
    Overline(ansi_term::Style),
    Underoverline(ansi_term::Style),
    NoDecoration,
}

impl Style {
    pub fn new() -> Self {
        Self {
            ansi_term_style: ansi_term::Style::new(),
            is_emph: false,
            is_omitted: false,
            is_raw: false,
            is_syntax_highlighted: false,
            decoration_style: DecorationStyle::NoDecoration,
        }
    }

    /// Construct Style from style and decoration-style strings supplied on command line, together
    /// with defaults. A style string is a space-separated string containing 0, 1, or 2 colors
    /// (foreground and then background) and an arbitrary number of style attributes. See `delta
    /// --help` for more precise spec.
    pub fn from_str(
        style_string: &str,
        foreground_default: Option<ansi_term::Color>,
        background_default: Option<ansi_term::Color>,
        decoration_style_string: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let (ansi_term_style, is_omitted, is_raw, is_syntax_highlighted) = parse_ansi_term_style(
            &style_string,
            foreground_default,
            background_default,
            true_color,
        );
        let decoration_style = match decoration_style_string {
            Some(s) if s != "" => DecorationStyle::from_str(s, true_color),
            _ => DecorationStyle::NoDecoration,
        };
        Style {
            ansi_term_style,
            is_emph,
            is_omitted,
            is_raw,
            is_syntax_highlighted,
            decoration_style,
        }
    }

    /// Construct Style but interpreting 'ul', 'box', etc as applying to the decoration style.
    fn from_str_with_handling_of_special_decoration_attributes(
        style_string: &str,
        foreground_default: Option<ansi_term::Color>,
        background_default: Option<ansi_term::Color>,
        decoration_style_string: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let (style_string, special_attribute_from_style_string) =
            extract_special_decoration_attribute(style_string);
        let mut style = Style::from_str(
            &style_string,
            foreground_default,
            background_default,
            decoration_style_string,
            true_color,
            is_emph,
        );
        if let Some(special_attribute) = special_attribute_from_style_string {
            style.decoration_style = DecorationStyle::apply_special_decoration_attribute(
                style.decoration_style,
                &special_attribute,
                true_color,
            )
        }
        style
    }

    /// As from_str_with_handling_of_special_decoration_attributes but respecting an optional
    /// foreground color which has precedence when present.
    pub fn from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
        style_string: &str,
        foreground_default: Option<ansi_term::Color>,
        background_default: Option<ansi_term::Color>,
        decoration_style_string: Option<&str>,
        deprecated_foreground_color_arg: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let mut style = Self::from_str_with_handling_of_special_decoration_attributes(
            style_string,
            foreground_default,
            background_default,
            decoration_style_string,
            true_color,
            is_emph,
        );
        if let Some(s) = deprecated_foreground_color_arg {
            // The deprecated --{commit,file,hunk}-color args functioned to set the decoration
            // foreground color. In the case of file, it set the text foreground color also.
            let foreground_from_deprecated_arg = parse_ansi_term_style(s, None, None, true_color)
                .0
                .foreground;
            style.ansi_term_style.foreground = foreground_from_deprecated_arg;
            style.decoration_style = match style.decoration_style {
                DecorationStyle::Box(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::Box(ansi_term_style)
                }
                DecorationStyle::Underline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::Underline(ansi_term_style)
                }
                DecorationStyle::Overline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::Overline(ansi_term_style)
                }
                DecorationStyle::Underoverline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::Underoverline(ansi_term_style)
                }
                DecorationStyle::NoDecoration => style.decoration_style,
            };
        }
        style
    }

    pub fn decoration_ansi_term_style(&self) -> Option<ansi_term::Style> {
        match self.decoration_style {
            DecorationStyle::Box(style) => Some(style),
            DecorationStyle::Underline(style) => Some(style),
            DecorationStyle::Overline(style) => Some(style),
            DecorationStyle::Underoverline(style) => Some(style),
            DecorationStyle::NoDecoration => None,
        }
    }
}

impl DecorationStyle {
    pub fn from_str(style_string: &str, true_color: bool) -> Self {
        let (style_string, special_attribute) = extract_special_decoration_attribute(&style_string);
        let (style, is_omitted, is_raw, is_syntax_highlighted) =
            parse_ansi_term_style(&style_string, None, None, true_color);
        if is_raw {
            eprintln!("'raw' may not be used in a decoration style.");
            process::exit(1);
        };
        if is_syntax_highlighted {
            eprintln!("'syntax' may not be used in a decoration style.");
            process::exit(1);
        };
        match special_attribute.as_deref() {
            Some("box") => DecorationStyle::Box(style),
            Some("underline") => DecorationStyle::Underline(style),
            Some("ul") => DecorationStyle::Underline(style),
            Some("overline") => DecorationStyle::Overline(style),
            Some("underoverline") => DecorationStyle::Underoverline(style),
            Some("none") => DecorationStyle::NoDecoration,
            Some("omit") => DecorationStyle::NoDecoration,
            Some("plain") => DecorationStyle::NoDecoration,
            // TODO: Exit with error if --thing-decoration-style supplied without a decoration type
            Some("") => DecorationStyle::NoDecoration,
            _ if is_omitted => DecorationStyle::NoDecoration,
            _ => unreachable("Unreachable code path reached in parse_decoration_style."),
        }
    }

    fn apply_special_decoration_attribute(
        decoration_style: DecorationStyle,
        special_attribute: &str,
        true_color: bool,
    ) -> DecorationStyle {
        let ansi_term_style = match decoration_style {
            DecorationStyle::Box(ansi_term_style) => ansi_term_style,
            DecorationStyle::Underline(ansi_term_style) => ansi_term_style,
            DecorationStyle::Overline(ansi_term_style) => ansi_term_style,
            DecorationStyle::Underoverline(ansi_term_style) => ansi_term_style,
            DecorationStyle::NoDecoration => ansi_term::Style::new(),
        };
        match DecorationStyle::from_str(special_attribute, true_color) {
            DecorationStyle::Box(_) => DecorationStyle::Box(ansi_term_style),
            DecorationStyle::Underline(_) => DecorationStyle::Underline(ansi_term_style),
            DecorationStyle::Overline(_) => DecorationStyle::Overline(ansi_term_style),
            DecorationStyle::Underoverline(_) => DecorationStyle::Underoverline(ansi_term_style),
            DecorationStyle::NoDecoration => DecorationStyle::NoDecoration,
        }
    }
}

fn parse_ansi_term_style(
    s: &str,
    foreground_default: Option<ansi_term::Color>,
    background_default: Option<ansi_term::Color>,
    true_color: bool,
) -> (ansi_term::Style, bool, bool, bool) {
    let mut style = ansi_term::Style::new();
    let mut seen_foreground = false;
    let mut seen_background = false;
    let mut is_omitted = false;
    let mut is_raw = false;
    let mut is_syntax_highlighted = false;
    for word in s
        .to_lowercase()
        .split_whitespace()
        .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
    {
        if word == "blink" {
            style.is_blink = true;
        } else if word == "bold" {
            style.is_bold = true;
        } else if word == "dim" {
            style.is_dimmed = true;
        } else if word == "hidden" {
            style.is_hidden = true;
        } else if word == "italic" {
            style.is_italic = true;
        } else if word == "omit" {
            is_omitted = true;
        } else if word == "reverse" {
            style.is_reverse = true;
        } else if word == "raw" {
            is_raw = true;
        } else if word == "strike" {
            style.is_strikethrough = true;
        } else if word == "ul" || word == "underline" {
            style.is_underline = true;
        } else if !seen_foreground {
            if word == "syntax" {
                is_syntax_highlighted = true;
            } else {
                style.foreground = color::color_from_rgb_or_ansi_code_with_default(
                    word,
                    foreground_default,
                    true_color,
                );
            }
            seen_foreground = true;
        } else if !seen_background {
            if word == "syntax" {
                eprintln!(
                    "You have used the special color 'syntax' as a background color \
                     (second color in a style string). It may only be used as a foreground \
                     color (first color in a style string)."
                );
                process::exit(1);
            } else {
                style.background = color::color_from_rgb_or_ansi_code_with_default(
                    word,
                    background_default,
                    true_color,
                );
            }
            seen_background = true;
        } else {
            eprintln!(
                "Invalid style string: {}. See the STYLES section of delta --help.",
                s
            );
            process::exit(1);
        }
    }
    (style, is_omitted, is_raw, is_syntax_highlighted)
}

/// If the style string contains a 'special decoration attribute' then extract it and return it
/// along with the modified style string.
fn extract_special_decoration_attribute(style_string: &str) -> (String, Option<String>) {
    let style_string = style_string.to_lowercase();
    let (special_attributes, standard_attributes): (Vec<&str>, Vec<&str>) = style_string
        .split_whitespace()
        .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
        .partition(|&token| {
            // TODO: This should be tied to the enum
            token == "box"
                || token == "ul"
                || token == "underline"
                || token == "overline"
                || token == "underoverline"
                || token == "none"
                || token == "plain"
        });
    match special_attributes {
        attrs if attrs.len() == 0 => (style_string.to_string(), None),
        attrs => (
            format!("{} {}", attrs[1..].join(" "), standard_attributes.join(" ")),
            Some(attrs[0].to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ansi_term;

    use crate::color::ansi_color_name_to_number;

    #[test]
    fn test_parse_ansi_term_style() {
        assert_eq!(
            parse_ansi_term_style("", None, None, false),
            (ansi_term::Style::new(), false, false, false)
        );
        assert_eq!(
            parse_ansi_term_style("red", None, None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("red").unwrap()
                    )),
                    ..ansi_term::Style::new()
                },
                false,
                false,
                false
            )
        );
        assert_eq!(
            parse_ansi_term_style("red green", None, None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("red").unwrap()
                    )),
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("green").unwrap()
                    )),
                    ..ansi_term::Style::new()
                },
                false,
                false,
                false
            )
        );
        assert_eq!(
            parse_ansi_term_style("bold red underline green blink", None, None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("red").unwrap()
                    )),
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("green").unwrap()
                    )),
                    is_blink: true,
                    is_bold: true,
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                false,
                false,
                false
            )
        );
    }

    #[test]
    fn test_parse_ansi_term_style_with_special_syntax_color() {
        assert_eq!(
            parse_ansi_term_style("syntax", None, None, false),
            (ansi_term::Style::new(), false, false, true)
        );
        assert_eq!(
            parse_ansi_term_style("syntax italic white hidden", None, None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("white").unwrap()
                    )),
                    is_italic: true,
                    is_hidden: true,
                    ..ansi_term::Style::new()
                },
                false,
                false,
                true
            )
        );
        assert_eq!(
            parse_ansi_term_style("bold syntax italic white hidden", None, None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("white").unwrap()
                    )),
                    is_bold: true,
                    is_italic: true,
                    is_hidden: true,
                    ..ansi_term::Style::new()
                },
                false,
                false,
                true
            )
        );
    }

    #[test]
    fn test_parse_ansi_term_style_with_special_omit_attribute() {
        assert_eq!(
            parse_ansi_term_style("omit", None, None, false),
            (ansi_term::Style::new(), true, false, false)
        );
        // It doesn't make sense for omit to be combined with anything else, but it is not an error.
        assert_eq!(
            parse_ansi_term_style("omit syntax italic white hidden", None, None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("white").unwrap()
                    )),
                    is_italic: true,
                    is_hidden: true,
                    ..ansi_term::Style::new()
                },
                true,
                false,
                true
            )
        );
    }

    #[test]
    fn test_parse_ansi_term_style_with_special_raw_attribute() {
        assert_eq!(
            parse_ansi_term_style("raw", None, None, false),
            (ansi_term::Style::new(), false, true, false)
        );
        // It doesn't make sense for raw to be combined with anything else, but it is not an error.
        assert_eq!(
            parse_ansi_term_style("raw syntax italic white hidden", None, None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::Fixed(
                        ansi_color_name_to_number("white").unwrap()
                    )),
                    is_italic: true,
                    is_hidden: true,
                    ..ansi_term::Style::new()
                },
                false,
                true,
                true
            )
        );
    }
}
