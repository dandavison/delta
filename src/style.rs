use ansi_term::{self, Color};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub ansi_term_style: ansi_term::Style,
    pub is_syntax_highlighted: bool,
    pub decoration_style: Option<DecorationStyle>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecorationStyle {
    Box(ansi_term::Style),
    Underline(ansi_term::Style),
    Omit,
}

impl Style {
    pub fn new() -> Self {
        Self {
            ansi_term_style: ansi_term::Style::new(),
            is_syntax_highlighted: false,
            decoration_style: None,
        }
    }

    pub fn decoration_ansi_term_style(&self) -> Option<ansi_term::Style> {
        match self.decoration_style {
            Some(DecorationStyle::Box(style)) => Some(style),
            Some(DecorationStyle::Underline(style)) => Some(style),
            _ => None,
        }
    }
}

// See
// https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
pub fn ansi_color_name_to_number(name: &str) -> Option<u8> {
    match name.to_lowercase().as_ref() {
        "black" => Some(0),
        "red" => Some(1),
        "green" => Some(2),
        "yellow" => Some(3),
        "blue" => Some(4),
        "magenta" => Some(5),
        "purple" => Some(5),
        "cyan" => Some(6),
        "white" => Some(7),
        "bright-black" => Some(8),
        "bright-red" => Some(9),
        "bright-green" => Some(10),
        "bright-yellow" => Some(11),
        "bright-blue" => Some(12),
        "bright-magenta" => Some(13),
        "bright-purple" => Some(13),
        "bright-cyan" => Some(14),
        "bright-white" => Some(15),
        _ => None,
    }
}

pub const LIGHT_THEMES: [&str; 5] = [
    "GitHub",
    "Monokai Extended Light",
    "OneHalfLight",
    "ansi-light",
    "Solarized (light)",
];

pub const DEFAULT_LIGHT_THEME: &str = "GitHub";
pub const DEFAULT_DARK_THEME: &str = "Monokai Extended";

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

pub fn is_no_syntax_highlighting_theme_name(theme_name: &str) -> bool {
    theme_name.to_lowercase() == "none"
}

pub fn get_minus_background_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_MINUS_COLOR,
        (true, false) => LIGHT_THEME_MINUS_COLOR_256,
        (false, true) => DARK_THEME_MINUS_COLOR,
        (false, false) => DARK_THEME_MINUS_COLOR_256,
    }
}

pub fn get_minus_emph_background_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_MINUS_EMPH_COLOR,
        (true, false) => LIGHT_THEME_MINUS_EMPH_COLOR_256,
        (false, true) => DARK_THEME_MINUS_EMPH_COLOR,
        (false, false) => DARK_THEME_MINUS_EMPH_COLOR_256,
    }
}

pub fn get_plus_background_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_PLUS_COLOR,
        (true, false) => LIGHT_THEME_PLUS_COLOR_256,
        (false, true) => DARK_THEME_PLUS_COLOR,
        (false, false) => DARK_THEME_PLUS_COLOR_256,
    }
}

pub fn get_plus_emph_background_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_PLUS_EMPH_COLOR,
        (true, false) => LIGHT_THEME_PLUS_EMPH_COLOR_256,
        (false, true) => DARK_THEME_PLUS_EMPH_COLOR,
        (false, false) => DARK_THEME_PLUS_EMPH_COLOR_256,
    }
}

const LIGHT_THEME_MINUS_COLOR: Color = Color::RGB(0xff, 0xe0, 0xe0);

const LIGHT_THEME_MINUS_COLOR_256: Color = Color::Fixed(224);

const LIGHT_THEME_MINUS_EMPH_COLOR: Color = Color::RGB(0xff, 0xc0, 0xc0);

const LIGHT_THEME_MINUS_EMPH_COLOR_256: Color = Color::Fixed(217);

const LIGHT_THEME_PLUS_COLOR: Color = Color::RGB(0xd0, 0xff, 0xd0);

const LIGHT_THEME_PLUS_COLOR_256: Color = Color::Fixed(194);

const LIGHT_THEME_PLUS_EMPH_COLOR: Color = Color::RGB(0xa0, 0xef, 0xa0);

const LIGHT_THEME_PLUS_EMPH_COLOR_256: Color = Color::Fixed(157);

const DARK_THEME_MINUS_COLOR: Color = Color::RGB(0x3f, 0x00, 0x01);

const DARK_THEME_MINUS_COLOR_256: Color = Color::Fixed(52);

const DARK_THEME_MINUS_EMPH_COLOR: Color = Color::RGB(0x90, 0x10, 0x11);

const DARK_THEME_MINUS_EMPH_COLOR_256: Color = Color::Fixed(124);

const DARK_THEME_PLUS_COLOR: Color = Color::RGB(0x00, 0x28, 0x00);

const DARK_THEME_PLUS_COLOR_256: Color = Color::Fixed(22);

const DARK_THEME_PLUS_EMPH_COLOR: Color = Color::RGB(0x00, 0x60, 0x00);

const DARK_THEME_PLUS_EMPH_COLOR_256: Color = Color::Fixed(28);
