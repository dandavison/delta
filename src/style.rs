use syntect::highlighting::{Color, FontStyle, Style, StyleModifier};

pub const LIGHT_THEMES: [&str; 4] = [
    "GitHub",
    "Monokai Extended Light",
    "OneHalfLight",
    "ansi-light",
];

pub const DEFAULT_LIGHT_THEME: &str = "GitHub";
pub const DEFAULT_DARK_THEME: &str = "Monokai Extended";

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

pub const LIGHT_THEME_MINUS_COLOR: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0xff,
};

pub const LIGHT_THEME_MINUS_EMPH_COLOR: Color = Color {
    r: 0xef,
    g: 0xa0,
    b: 0xa0,
    a: 0xff,
};

pub const LIGHT_THEME_PLUS_COLOR: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0xff,
};

pub const LIGHT_THEME_PLUS_EMPH_COLOR: Color = Color {
    r: 0xa0,
    g: 0xef,
    b: 0xa0,
    a: 0xff,
};

pub const DARK_THEME_MINUS_COLOR: Color = Color {
    r: 0x3F,
    g: 0x00,
    b: 0x01,
    a: 0xff,
};

pub const DARK_THEME_MINUS_EMPH_COLOR: Color = Color {
    r: 0x90,
    g: 0x10,
    b: 0x11,
    a: 0xff,
};

pub const DARK_THEME_PLUS_COLOR: Color = Color {
    r: 0x01,
    g: 0x3B,
    b: 0x01,
    a: 0xff,
};

pub const DARK_THEME_PLUS_EMPH_COLOR: Color = Color {
    r: 0x11,
    g: 0x80,
    b: 0x11,
    a: 0xff,
};

/// A special color to specify that no color escape codes should be emitted.
pub const NO_COLOR: Color = Color::BLACK;

pub fn get_no_style() -> Style {
    Style {
        foreground: NO_COLOR,
        background: NO_COLOR,
        font_style: FontStyle::empty(),
    }
}

pub const NO_BACKGROUND_COLOR_STYLE_MODIFIER: StyleModifier = StyleModifier {
    foreground: None,
    background: Some(NO_COLOR),
    font_style: None,
};
