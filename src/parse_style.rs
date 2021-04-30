use std::process;

use bitflags::bitflags;

use crate::color;
use crate::config::delta_unreachable;
use crate::style::{DecorationStyle, Style};

impl Style {
    /// Construct Style from style and decoration-style strings supplied on command line, together
    /// with defaults. A style string is a space-separated string containing 0, 1, or 2 colors
    /// (foreground and then background) and an arbitrary number of style attributes. See `delta
    /// --help` for more precise spec.
    pub fn from_str(
        style_string: &str,
        default: Option<Self>,
        decoration_style_string: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let (ansi_term_style, is_omitted, is_raw, is_syntax_highlighted) =
            parse_ansi_term_style(&style_string, default, true_color);
        let decoration_style =
            DecorationStyle::from_str(decoration_style_string.unwrap_or(""), true_color);
        Self {
            ansi_term_style,
            is_emph,
            is_omitted,
            is_raw,
            is_syntax_highlighted,
            decoration_style,
        }
    }

    pub fn from_git_str(git_style_string: &str) -> Self {
        Self::from_str(git_style_string, None, None, true, false)
    }

    /// Construct Style but interpreting 'ul', 'box', etc as applying to the decoration style.
    pub fn from_str_with_handling_of_special_decoration_attributes(
        style_string: &str,
        default: Option<Self>,
        decoration_style_string: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let (special_attributes_from_style_string, style_string) =
            extract_special_decoration_attributes_from_non_decoration_style_string(style_string);
        let mut style = Style::from_str(
            &style_string,
            default,
            decoration_style_string.as_deref(),
            true_color,
            is_emph,
        );
        // TODO: box in this context resulted in box-with-underline for commit and file
        style.decoration_style = DecorationStyle::apply_special_decoration_attributes(
            &mut style,
            special_attributes_from_style_string,
        );
        style
    }

    /// As from_str_with_handling_of_special_decoration_attributes but respecting an optional
    /// foreground color which has precedence when present.
    pub fn from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
        style_string: &str,
        default: Option<Self>,
        decoration_style_string: Option<&str>,
        deprecated_foreground_color_arg: Option<&str>,
        true_color: bool,
        is_emph: bool,
    ) -> Self {
        let mut style = Self::from_str_with_handling_of_special_decoration_attributes(
            style_string,
            default,
            decoration_style_string,
            true_color,
            is_emph,
        );
        if let Some(s) = deprecated_foreground_color_arg {
            // The deprecated --{commit,file,hunk}-color args functioned to set the decoration
            // foreground color. In the case of file, it set the text foreground color also.
            let foreground_from_deprecated_arg =
                parse_ansi_term_style(s, None, true_color).0.foreground;
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
                DecorationStyle::UnderOverline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::UnderOverline(ansi_term_style)
                }
                DecorationStyle::BoxWithUnderline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::BoxWithUnderline(ansi_term_style)
                }
                DecorationStyle::BoxWithOverline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::BoxWithOverline(ansi_term_style)
                }
                DecorationStyle::BoxWithUnderOverline(mut ansi_term_style) => {
                    ansi_term_style.foreground = foreground_from_deprecated_arg;
                    DecorationStyle::BoxWithUnderOverline(ansi_term_style)
                }
                DecorationStyle::NoDecoration => style.decoration_style,
            };
        }
        style
    }
}

bitflags! {
    struct DecorationAttributes: u8 {
        const EMPTY = 0b00000000;
        const BOX = 0b00000001;
        const OVERLINE = 0b00000010;
        const UNDERLINE = 0b00000100;
    }
}

impl DecorationStyle {
    pub fn from_str(style_string: &str, true_color: bool) -> Self {
        let (special_attributes, style_string) =
            extract_special_decoration_attributes(&style_string);
        let (style, is_omitted, is_raw, is_syntax_highlighted) =
            parse_ansi_term_style(&style_string, None, true_color);
        if is_raw {
            eprintln!("'raw' may not be used in a decoration style.");
            process::exit(1);
        };
        if is_syntax_highlighted {
            eprintln!("'syntax' may not be used in a decoration style.");
            process::exit(1);
        };
        #[allow(non_snake_case)]
        let (BOX, UL, OL, EMPTY) = (
            DecorationAttributes::BOX,
            DecorationAttributes::UNDERLINE,
            DecorationAttributes::OVERLINE,
            DecorationAttributes::EMPTY,
        );
        match special_attributes {
            bits if bits == EMPTY => DecorationStyle::NoDecoration,
            bits if bits == BOX => DecorationStyle::Box(style),
            bits if bits == UL => DecorationStyle::Underline(style),
            bits if bits == OL => DecorationStyle::Overline(style),
            bits if bits == UL | OL => DecorationStyle::UnderOverline(style),
            bits if bits == BOX | UL => DecorationStyle::BoxWithUnderline(style),
            bits if bits == BOX | OL => DecorationStyle::BoxWithOverline(style),
            bits if bits == BOX | UL | OL => DecorationStyle::BoxWithUnderOverline(style),
            _ if is_omitted => DecorationStyle::NoDecoration,
            _ => delta_unreachable("Unreachable code path reached in parse_decoration_style."),
        }
    }

    fn apply_special_decoration_attributes(
        style: &mut Style,
        special_attributes: DecorationAttributes,
    ) -> DecorationStyle {
        let ansi_term_style = match style.decoration_style {
            DecorationStyle::Box(ansi_term_style) => ansi_term_style,
            DecorationStyle::Underline(ansi_term_style) => ansi_term_style,
            DecorationStyle::Overline(ansi_term_style) => ansi_term_style,
            DecorationStyle::UnderOverline(ansi_term_style) => ansi_term_style,
            DecorationStyle::BoxWithUnderline(ansi_term_style) => ansi_term_style,
            DecorationStyle::BoxWithOverline(ansi_term_style) => ansi_term_style,
            DecorationStyle::BoxWithUnderOverline(ansi_term_style) => ansi_term_style,
            DecorationStyle::NoDecoration => ansi_term::Style::new(),
        };
        #[allow(non_snake_case)]
        let (BOX, UL, OL, EMPTY) = (
            DecorationAttributes::BOX,
            DecorationAttributes::UNDERLINE,
            DecorationAttributes::OVERLINE,
            DecorationAttributes::EMPTY,
        );
        match special_attributes {
            bits if bits == EMPTY => style.decoration_style,
            bits if bits == BOX => DecorationStyle::Box(ansi_term_style),
            bits if bits == UL => DecorationStyle::Underline(ansi_term_style),
            bits if bits == OL => DecorationStyle::Overline(ansi_term_style),
            bits if bits == UL | OL => DecorationStyle::UnderOverline(ansi_term_style),
            bits if bits == BOX | UL => DecorationStyle::BoxWithUnderline(ansi_term_style),
            bits if bits == BOX | OL => DecorationStyle::BoxWithOverline(ansi_term_style),
            bits if bits == BOX | UL | OL => DecorationStyle::BoxWithUnderOverline(ansi_term_style),
            _ => DecorationStyle::NoDecoration,
        }
    }
}

fn parse_ansi_term_style(
    s: &str,
    default: Option<Style>,
    true_color: bool,
) -> (ansi_term::Style, bool, bool, bool) {
    let mut style = ansi_term::Style::new();
    let mut seen_foreground = false;
    let mut seen_background = false;
    let mut foreground_is_auto = false;
    let mut background_is_auto = false;
    let mut is_omitted = false;
    let mut is_raw = false;
    let mut seen_omit = false;
    let mut seen_raw = false;
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
            seen_omit = true;
            is_omitted = true;
        } else if word == "reverse" {
            style.is_reverse = true;
        } else if word == "raw" {
            seen_raw = true;
            is_raw = true;
        } else if word == "strike" {
            style.is_strikethrough = true;
        } else if word == "ul" || word == "underline" {
            style.is_underline = true;
        } else if word == "line-number" || word == "file" {
            // Allow: these are meaningful in hunk-header-style.
        } else if !seen_foreground {
            if word == "syntax" {
                is_syntax_highlighted = true;
            } else if word == "auto" {
                foreground_is_auto = true;
                style.foreground = default.and_then(|s| s.ansi_term_style.foreground);
                is_syntax_highlighted = default.map(|s| s.is_syntax_highlighted).unwrap_or(false);
            } else {
                style.foreground = color::parse_color(word, true_color);
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
            } else if word == "auto" {
                background_is_auto = true;
                style.background = default.and_then(|s| s.ansi_term_style.background);
            } else {
                style.background = color::parse_color(word, true_color);
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
    if foreground_is_auto && background_is_auto {
        if !seen_omit {
            is_omitted = default.map(|s| s.is_omitted).unwrap_or(false);
        }
        if !seen_raw {
            is_raw = default.map(|s| s.is_raw).unwrap_or(false);
        }
    }
    (style, is_omitted, is_raw, is_syntax_highlighted)
}

/// Extract set of 'special decoration attributes' and return it along with modified style string.
fn extract_special_decoration_attributes(style_string: &str) -> (DecorationAttributes, String) {
    _extract_special_decoration_attributes(style_string, true)
}

fn extract_special_decoration_attributes_from_non_decoration_style_string(
    style_string: &str,
) -> (DecorationAttributes, String) {
    _extract_special_decoration_attributes(style_string, false)
}

// If this is being called in the context of processing a decoration style string then we treat
// ul/ol as a request for an underline/overline decoration respectively. Otherwise they are
// conventional character style attributes.
fn _extract_special_decoration_attributes(
    style_string: &str,
    is_decoration_style_string: bool,
) -> (DecorationAttributes, String) {
    let mut attributes = DecorationAttributes::EMPTY;
    let mut new_style_string = Vec::new();
    let style_string = style_string.to_lowercase();
    for token in style_string
        .split_whitespace()
        .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
    {
        match token {
            "box" => attributes |= DecorationAttributes::BOX,
            token if token == "overline" || is_decoration_style_string && token == "ol" => {
                attributes |= DecorationAttributes::OVERLINE
            }
            token if token == "underline" || is_decoration_style_string && token == "ul" => {
                attributes |= DecorationAttributes::UNDERLINE
            }
            token if token == "none" || token == "plain" => {}
            _ => new_style_string.push(token),
        }
    }
    (attributes, new_style_string.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    use ansi_term;

    #[test]
    fn test_parse_ansi_term_style() {
        assert_eq!(
            parse_ansi_term_style("", None, false),
            (ansi_term::Style::new(), false, false, false)
        );
        assert_eq!(
            parse_ansi_term_style("red", None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Red),
                    ..ansi_term::Style::new()
                },
                false,
                false,
                false
            )
        );
        assert_eq!(
            parse_ansi_term_style("red green", None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Red),
                    background: Some(ansi_term::Color::Green),
                    ..ansi_term::Style::new()
                },
                false,
                false,
                false
            )
        );
        assert_eq!(
            parse_ansi_term_style("bold red underline green blink", None, false),
            (
                ansi_term::Style {
                    foreground: Some(ansi_term::Color::Red),
                    background: Some(ansi_term::Color::Green),
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
            parse_ansi_term_style("syntax", None, false),
            (ansi_term::Style::new(), false, false, true)
        );
        assert_eq!(
            parse_ansi_term_style("syntax italic white hidden", None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::White),
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
            parse_ansi_term_style("bold syntax italic white hidden", None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::White),
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
            parse_ansi_term_style("omit", None, false),
            (ansi_term::Style::new(), true, false, false)
        );
        // It doesn't make sense for omit to be combined with anything else, but it is not an error.
        assert_eq!(
            parse_ansi_term_style("omit syntax italic white hidden", None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::White),
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
            parse_ansi_term_style("raw", None, false),
            (ansi_term::Style::new(), false, true, false)
        );
        // It doesn't make sense for raw to be combined with anything else, but it is not an error.
        assert_eq!(
            parse_ansi_term_style("raw syntax italic white hidden", None, false),
            (
                ansi_term::Style {
                    background: Some(ansi_term::Color::White),
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

    #[test]
    fn test_extract_special_decoration_attribute() {
        #[allow(non_snake_case)]
        let (BOX, UL, OL, EMPTY) = (
            DecorationAttributes::BOX,
            DecorationAttributes::UNDERLINE,
            DecorationAttributes::OVERLINE,
            DecorationAttributes::EMPTY,
        );
        assert_eq!(
            extract_special_decoration_attributes(""),
            (EMPTY, "".to_string(),)
        );
        assert_eq!(
            extract_special_decoration_attributes("box"),
            (BOX, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("ul"),
            (UL, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("ol"),
            (OL, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("box ul"),
            (BOX | UL, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("box ol"),
            (BOX | OL, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("ul box ol"),
            (BOX | UL | OL, "".to_string())
        );
        assert_eq!(
            extract_special_decoration_attributes("ol ul"),
            (UL | OL, "".to_string())
        );
    }

    #[test]
    fn test_decoration_style_from_str_empty_string() {
        assert_eq!(
            DecorationStyle::from_str("", true),
            DecorationStyle::NoDecoration,
        )
    }

    #[test]
    fn test_decoration_style_from_str() {
        assert_eq!(
            DecorationStyle::from_str("ol red box bold green ul", true),
            DecorationStyle::BoxWithUnderOverline(ansi_term::Style {
                foreground: Some(ansi_term::Color::Red),
                background: Some(ansi_term::Color::Green),
                is_bold: true,
                ..ansi_term::Style::new()
            })
        )
    }

    #[test]
    fn test_style_from_str() {
        let actual_style = Style::from_str(
            "red green bold",
            None,
            Some("ol red box bold green ul"),
            true,
            false,
        );
        let red_green_bold = ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            background: Some(ansi_term::Color::Green),
            is_bold: true,
            ..ansi_term::Style::new()
        };
        assert_eq!(
            actual_style,
            Style {
                ansi_term_style: red_green_bold,
                decoration_style: DecorationStyle::BoxWithUnderOverline(red_green_bold),
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_raw_with_box() {
        let actual_style = Style::from_str("raw", None, Some("box"), true, false);
        let empty_ansi_term_style = ansi_term::Style::new();
        assert_eq!(
            actual_style,
            Style {
                ansi_term_style: empty_ansi_term_style,
                decoration_style: DecorationStyle::Box(empty_ansi_term_style),
                is_raw: true,
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_decoration_style_only() {
        let actual_style = Style::from_str("", None, Some("ol red box bold green ul"), true, false);
        let red_green_bold = ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            background: Some(ansi_term::Color::Green),
            is_bold: true,
            ..ansi_term::Style::new()
        };
        assert_eq!(
            actual_style,
            Style {
                decoration_style: DecorationStyle::BoxWithUnderOverline(red_green_bold),
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_with_handling_of_special_decoration_attributes() {
        let actual_style = Style::from_str_with_handling_of_special_decoration_attributes(
            "",
            None,
            Some("ol red box bold green ul"),
            true,
            false,
        );
        let expected_decoration_style = DecorationStyle::BoxWithUnderOverline(ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            background: Some(ansi_term::Color::Green),
            is_bold: true,
            ..ansi_term::Style::new()
        });
        assert_eq!(
            actual_style,
            Style {
                decoration_style: expected_decoration_style,
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_with_handling_of_special_decoration_attributes_raw_with_box() {
        let actual_style = Style::from_str_with_handling_of_special_decoration_attributes(
            "raw",
            None,
            Some("box"),
            true,
            false,
        );
        let empty_ansi_term_style = ansi_term::Style::new();
        assert_eq!(
            actual_style,
            Style {
                ansi_term_style: empty_ansi_term_style,
                decoration_style: DecorationStyle::Box(empty_ansi_term_style),
                is_raw: true,
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
    ) {
        let expected_decoration_style = DecorationStyle::BoxWithUnderOverline(ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            background: Some(ansi_term::Color::Green),
            is_bold: true,
            ..ansi_term::Style::new()
        });
        let actual_style = Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
                "", None, Some("ol red box bold green ul"), None, true, false
            );
        assert_eq!(
            actual_style,
            Style {
                decoration_style: expected_decoration_style,
                ..Style::new()
            }
        )
    }

    #[test]
    fn test_style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg_raw_with_box(
    ) {
        let actual_style = Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
            "raw",
            None,
            Some("box"),
            None,
            true,
            false,
        );
        let empty_ansi_term_style = ansi_term::Style::new();
        assert_eq!(
            actual_style,
            Style {
                ansi_term_style: empty_ansi_term_style,
                decoration_style: DecorationStyle::Box(empty_ansi_term_style),
                is_raw: true,
                ..Style::new()
            }
        )
    }
}
