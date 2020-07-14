use std::borrow::Cow;
use std::fmt;

use ansi_term;

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
