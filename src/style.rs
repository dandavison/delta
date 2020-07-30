use std::borrow::Cow;
use std::fmt;

use ansi_term;
use lazy_static::lazy_static;

use crate::ansi;
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
    UnderOverline(ansi_term::Style),
    BoxWithUnderline(ansi_term::Style),
    BoxWithOverline(ansi_term::Style),
    BoxWithUnderOverline(ansi_term::Style),
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

    pub fn from_colors(
        foreground: Option<ansi_term::Color>,
        background: Option<ansi_term::Color>,
    ) -> Self {
        Self {
            ansi_term_style: ansi_term::Style {
                foreground,
                background,
                ..ansi_term::Style::new()
            },
            ..Self::new()
        }
    }

    pub fn paint<'a, I, S: 'a + ToOwned + ?Sized>(
        self,
        input: I,
    ) -> ansi_term::ANSIGenericString<'a, S>
    where
        I: Into<Cow<'a, S>>,
        <S as ToOwned>::Owned: fmt::Debug,
    {
        self.ansi_term_style.paint(input)
    }

    pub fn get_background_color(&self) -> Option<ansi_term::Color> {
        if self.ansi_term_style.is_reverse {
            self.ansi_term_style.foreground
        } else {
            self.ansi_term_style.background
        }
    }

    pub fn decoration_ansi_term_style(&self) -> Option<ansi_term::Style> {
        match self.decoration_style {
            DecorationStyle::Box(style) => Some(style),
            DecorationStyle::Underline(style) => Some(style),
            DecorationStyle::Overline(style) => Some(style),
            DecorationStyle::UnderOverline(style) => Some(style),
            DecorationStyle::BoxWithUnderline(style) => Some(style),
            DecorationStyle::BoxWithOverline(style) => Some(style),
            DecorationStyle::BoxWithUnderOverline(style) => Some(style),
            DecorationStyle::NoDecoration => None,
        }
    }

    pub fn is_applied_to(&self, s: &str) -> bool {
        s.starts_with(&self.ansi_term_style.prefix().to_string())
    }

    pub fn to_painted_string(&self) -> ansi_term::ANSIGenericString<str> {
        self.paint(self.to_string())
    }

    fn to_string(&self) -> String {
        if self.is_raw {
            return "raw".to_string();
        }
        let mut words = Vec::<String>::new();
        if self.is_omitted {
            words.push("omit".to_string());
        }
        if self.ansi_term_style.is_blink {
            words.push("blink".to_string());
        }
        if self.ansi_term_style.is_bold {
            words.push("bold".to_string());
        }
        if self.ansi_term_style.is_dimmed {
            words.push("dim".to_string());
        }
        if self.ansi_term_style.is_italic {
            words.push("italic".to_string());
        }
        if self.ansi_term_style.is_reverse {
            words.push("reverse".to_string());
        }
        if self.ansi_term_style.is_strikethrough {
            words.push("strike".to_string());
        }
        if self.ansi_term_style.is_underline {
            words.push("ul".to_string());
        }

        match (self.is_syntax_highlighted, self.ansi_term_style.foreground) {
            (true, _) => words.push("syntax".to_string()),
            (false, Some(color)) => {
                words.push(color::color_to_string(color));
            }
            (false, None) => words.push("normal".to_string()),
        }
        match self.ansi_term_style.background {
            Some(color) => words.push(color::color_to_string(color)),
            None => {}
        }
        words.join(" ")
    }
}

lazy_static! {
    pub static ref GIT_DEFAULT_MINUS_STYLE: Style = Style {
        ansi_term_style: ansi_term::Color::Red.normal(),
        ..Style::new()
    };
    pub static ref GIT_DEFAULT_PLUS_STYLE: Style = Style {
        ansi_term_style: ansi_term::Color::Green.normal(),
        ..Style::new()
    };
}

pub fn line_has_style_other_than<'a>(line: &str, styles: impl Iterator<Item = &'a Style>) -> bool {
    if !ansi::string_starts_with_ansi_escape_sequence(line) {
        return false;
    }
    for style in styles {
        if style.is_applied_to(line) {
            return false;
        }
    }
    return true;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_is_applied_to() {
        assert!(Style::from_git_str(r##"black "#ddeeff""##)
                .is_applied_to(
                    "\x1b[30;48;2;221;238;255m+\x1b[m\x1b[30;48;2;221;238;255m        .map(|(_, is_ansi)| is_ansi)\x1b[m\n"))
    }

    #[test]
    fn test_git_default_styles() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let plus_line_from_unconfigured_git = "\x1b[32m+\x1b[m\x1b[32m____\x1b[m\n";
        assert!(GIT_DEFAULT_MINUS_STYLE.is_applied_to(minus_line_from_unconfigured_git));
        assert!(!GIT_DEFAULT_MINUS_STYLE.is_applied_to(plus_line_from_unconfigured_git));

        assert!(GIT_DEFAULT_PLUS_STYLE.is_applied_to(plus_line_from_unconfigured_git));
        assert!(!GIT_DEFAULT_PLUS_STYLE.is_applied_to(minus_line_from_unconfigured_git));
    }

    #[test]
    fn test_line_has_style_other_than() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let plus_line_from_unconfigured_git = "\x1b[32m+\x1b[m\x1b[32m____\x1b[m\n";

        // Unstyled lines should test negative, regardless of supplied styles.
        assert!(!line_has_style_other_than("", [].iter()));
        assert!(!line_has_style_other_than(
            "",
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));

        // Lines from git should test negative when corresponding default is supplied
        assert!(!line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));
        assert!(!line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [*GIT_DEFAULT_PLUS_STYLE].iter()
        ));

        // Styled lines should test positive when unless their style is supplied.
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [*GIT_DEFAULT_PLUS_STYLE].iter()
        ));
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [].iter()
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [].iter()
        ));
    }
}
