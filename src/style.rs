use syntect::highlighting::{Color, FontStyle, Style, StyleModifier};

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

pub const LIGHT_THEME_MINUS_COLOR: Color = Color {
    r: 0xff,
    g: 0xe0,
    b: 0xe0,
    a: 0xff,
};

pub const LIGHT_THEME_MINUS_EMPH_COLOR: Color = Color {
    r: 0xff,
    g: 0xc0,
    b: 0xc0,
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
    r: 0x00,
    g: 0x28,
    b: 0x00,
    a: 0xff,
};

pub const DARK_THEME_PLUS_EMPH_COLOR: Color = Color {
    r: 0x00,
    g: 0x60,
    b: 0x00,
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
