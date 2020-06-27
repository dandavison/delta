use std::process;
use std::str::FromStr;

use ansi_term::Color;
use syntect::highlighting::Color as SyntectColor;

use crate::bat::terminal::to_ansi_color;
use crate::syntect_color;

pub fn color_from_rgb_or_ansi_code(s: &str, true_color: bool) -> Color {
    let die = || {
        eprintln!("Invalid color or style attribute: {}", s);
        process::exit(1);
    };
    let syntect_color = if s.starts_with("#") {
        SyntectColor::from_str(s).unwrap_or_else(|_| die())
    } else {
        s.parse::<u8>()
            .ok()
            .and_then(syntect_color::syntect_color_from_ansi_number)
            .or_else(|| syntect_color::syntect_color_from_ansi_name(s))
            .unwrap_or_else(die)
    };
    to_ansi_color(syntect_color, true_color)
}

pub fn color_from_rgb_or_ansi_code_with_default(arg: &str, true_color: bool) -> Option<Color> {
    match arg {
        "normal" => None,
        s => Some(color_from_rgb_or_ansi_code(s, true_color)),
    }
}

pub fn color_to_string(color: Color) -> String {
    match color {
        Color::Fixed(n) => format!("{}", n),
        Color::RGB(r, g, b) => format!("#{:02x?}{:02x?}{:02x?}", r, g, b),
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Purple => "purple".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::White => "white".to_string(),
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
        "brightblack" => Some(8),
        "bright-red" => Some(9),
        "brightred" => Some(9),
        "bright-green" => Some(10),
        "brightgreen" => Some(10),
        "bright-yellow" => Some(11),
        "brightyellow" => Some(11),
        "bright-blue" => Some(12),
        "brightblue" => Some(12),
        "bright-magenta" => Some(13),
        "brightmagenta" => Some(13),
        "bright-purple" => Some(13),
        "brightpurple" => Some(13),
        "bright-cyan" => Some(14),
        "brightcyan" => Some(14),
        "bright-white" => Some(15),
        "brightwhite" => Some(15),
        _ => None,
    }
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
