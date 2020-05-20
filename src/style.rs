use ansi_term::Color;

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

pub fn get_minus_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_MINUS_COLOR,
        (true, false) => LIGHT_THEME_MINUS_COLOR_256,
        (false, true) => DARK_THEME_MINUS_COLOR,
        (false, false) => DARK_THEME_MINUS_COLOR_256,
    }
}

pub fn get_minus_emph_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_MINUS_EMPH_COLOR,
        (true, false) => LIGHT_THEME_MINUS_EMPH_COLOR_256,
        (false, true) => DARK_THEME_MINUS_EMPH_COLOR,
        (false, false) => DARK_THEME_MINUS_EMPH_COLOR_256,
    }
}

pub fn get_plus_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
    match (is_light_mode, is_true_color) {
        (true, true) => LIGHT_THEME_PLUS_COLOR,
        (true, false) => LIGHT_THEME_PLUS_COLOR_256,
        (false, true) => DARK_THEME_PLUS_COLOR,
        (false, false) => DARK_THEME_PLUS_COLOR_256,
    }
}

pub fn get_plus_emph_color_default(is_light_mode: bool, is_true_color: bool) -> Color {
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

/// A special color value to signify that the foreground color of a style should be derived from
/// syntax highlighting.
pub const SYNTAX_HIGHLIGHTING_COLOR: Color = Color::White; // TODO
