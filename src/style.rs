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

const LIGHT_THEME_MINUS_COLOR: Color = Color {
    r: 0xff,
    g: 0xe0,
    b: 0xe0,
    a: 0xff,
};

const LIGHT_THEME_MINUS_COLOR_256: Color = Color {
    r: 224,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const LIGHT_THEME_MINUS_EMPH_COLOR: Color = Color {
    r: 0xff,
    g: 0xc0,
    b: 0xc0,
    a: 0xff,
};

const LIGHT_THEME_MINUS_EMPH_COLOR_256: Color = Color {
    r: 217,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const LIGHT_THEME_PLUS_COLOR: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0xff,
};

const LIGHT_THEME_PLUS_COLOR_256: Color = Color {
    r: 194,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const LIGHT_THEME_PLUS_EMPH_COLOR: Color = Color {
    r: 0xa0,
    g: 0xef,
    b: 0xa0,
    a: 0xff,
};

const LIGHT_THEME_PLUS_EMPH_COLOR_256: Color = Color {
    r: 157,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const DARK_THEME_MINUS_COLOR: Color = Color {
    r: 0x3f,
    g: 0x00,
    b: 0x01,
    a: 0xff,
};

const DARK_THEME_MINUS_COLOR_256: Color = Color {
    r: 52,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const DARK_THEME_MINUS_EMPH_COLOR: Color = Color {
    r: 0x90,
    g: 0x10,
    b: 0x11,
    a: 0xff,
};

const DARK_THEME_MINUS_EMPH_COLOR_256: Color = Color {
    r: 124,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const DARK_THEME_PLUS_COLOR: Color = Color {
    r: 0x00,
    g: 0x28,
    b: 0x00,
    a: 0xff,
};

const DARK_THEME_PLUS_COLOR_256: Color = Color {
    r: 22,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

const DARK_THEME_PLUS_EMPH_COLOR: Color = Color {
    r: 0x00,
    g: 0x60,
    b: 0x00,
    a: 0xff,
};

const DARK_THEME_PLUS_EMPH_COLOR_256: Color = Color {
    r: 28,
    g: 0x00,
    b: 0x00,
    a: 0x00,
};

/// A special color to specify that no color escape codes should be emitted.
pub const NO_COLOR: Color = Color::BLACK;

/// A special color value to signify that the foreground color of a style should be derived from
/// syntax highlighting.
// alpha is 0, which is how the 256-palette colors are encoded (see bat::terminal::to_ansi_color).
// So if painted, this would be black. However, the presence of a non-zero bit in the blue channel
// distinguishes this from any 256-palette color.
pub const SYNTAX_HIGHLIGHTING_COLOR: Color = Color {
    r: 0x00,
    g: 0x00,
    b: 0x01,
    a: 0x00,
};

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
