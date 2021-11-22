use std::str::FromStr;

use syntect::highlighting::{Color, FontStyle, Style};

use crate::color;
use crate::style as delta_style;

pub fn syntect_color_from_ansi_name(name: &str) -> Option<Color> {
    color::ansi_16_color_name_to_number(name).and_then(syntect_color_from_ansi_number)
}

pub fn syntect_color_from_name(name: &str) -> Option<Color> {
    palette::named::from_str(name).map(|color| Color {
        r: color.red,
        g: color.green,
        b: color.blue,
        a: 0xFF,
    })
}

/// Convert 8-bit ANSI code to #RGBA string with ANSI code in red channel and 0 in alpha channel.
// See https://github.com/sharkdp/bat/pull/543
pub fn syntect_color_from_ansi_number(n: u8) -> Option<Color> {
    Color::from_str(&format!("#{:02x}000000", n)).ok()
}

pub trait FromAnsiTermStyle {
    fn from_ansi_term_style(ansi_term_style: ansi_term::Style) -> Self;
}

impl FromAnsiTermStyle for Style {
    fn from_ansi_term_style(ansi_term_style: ansi_term::Style) -> Self {
        let default = Self::default();
        Self {
            foreground: if let Some(color) = ansi_term_style.foreground {
                Color::from_ansi_term_color(color)
            } else {
                default.foreground
            },
            background: if let Some(color) = ansi_term_style.background {
                Color::from_ansi_term_color(color)
            } else {
                default.background
            },
            font_style: FontStyle::from_ansi_term_style(ansi_term_style),
        }
    }
}

impl FromAnsiTermStyle for FontStyle {
    fn from_ansi_term_style(ansi_term_style: ansi_term::Style) -> Self {
        let mut font_style = FontStyle::empty();
        if ansi_term_style.is_bold {
            font_style |= FontStyle::BOLD
        }
        if ansi_term_style.is_italic {
            font_style |= FontStyle::ITALIC
        }
        if ansi_term_style.is_underline {
            font_style |= FontStyle::UNDERLINE
        }
        font_style
    }
}

pub trait FromAnsiTermColor {
    fn from_ansi_term_color(ansi_term_color: ansi_term::Color) -> Self;
}

impl FromAnsiTermColor for Color {
    fn from_ansi_term_color(ansi_term_color: ansi_term::Color) -> Self {
        match ansi_term_color {
            ansi_term::Color::Black => syntect_color_from_ansi_number(0).unwrap(),
            ansi_term::Color::Red => syntect_color_from_ansi_number(1).unwrap(),
            ansi_term::Color::Green => syntect_color_from_ansi_number(2).unwrap(),
            ansi_term::Color::Yellow => syntect_color_from_ansi_number(3).unwrap(),
            ansi_term::Color::Blue => syntect_color_from_ansi_number(4).unwrap(),
            ansi_term::Color::Purple => syntect_color_from_ansi_number(5).unwrap(),
            ansi_term::Color::Cyan => syntect_color_from_ansi_number(6).unwrap(),
            ansi_term::Color::White => syntect_color_from_ansi_number(7).unwrap(),
            ansi_term::Color::Fixed(n) => syntect_color_from_ansi_number(n).unwrap(),
            ansi_term::Color::RGB(r, g, b) => Self { r, g, b, a: 0xFF },
        }
    }
}

pub trait FromDeltaStyle {
    fn from_delta_style(delta_style: delta_style::Style) -> Self;
}

impl FromDeltaStyle for Style {
    fn from_delta_style(delta_style: delta_style::Style) -> Self {
        Self::from_ansi_term_style(delta_style.ansi_term_style)
    }
}
