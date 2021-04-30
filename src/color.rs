use std::collections::HashMap;
use std::process;
use std::str::FromStr;

use ansi_term::Color;
use lazy_static::lazy_static;
use syntect::highlighting::Color as SyntectColor;

use crate::bat_utils::terminal::to_ansi_color;
use crate::syntect_color;

pub fn parse_color(s: &str, true_color: bool) -> Option<Color> {
    if s == "normal" {
        return None;
    }
    let die = || {
        eprintln!("Invalid color or style attribute: {}", s);
        process::exit(1);
    };
    let syntect_color = if s.starts_with('#') {
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

pub fn color_to_string(color: Color) -> String {
    match color {
        Color::Fixed(n) if n < 16 => ansi_16_color_number_to_name(n).unwrap().to_string(),
        Color::Fixed(n) => format!("{}", n),
        Color::RGB(r, g, b) => format!("\"#{:02x?}{:02x?}{:02x?}\"", r, g, b),
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
lazy_static! {
    static ref ANSI_16_COLORS: HashMap<&'static str, u8> = {
        vec![
            ("black", 0),
            ("red", 1),
            ("green", 2),
            ("yellow", 3),
            ("blue", 4),
            ("magenta", 5),
            ("purple", 5),
            ("cyan", 6),
            ("white", 7),
            ("bright-black", 8),
            ("brightblack", 8),
            ("bright-red", 9),
            ("brightred", 9),
            ("bright-green", 10),
            ("brightgreen", 10),
            ("bright-yellow", 11),
            ("brightyellow", 11),
            ("bright-blue", 12),
            ("brightblue", 12),
            ("bright-magenta", 13),
            ("brightmagenta", 13),
            ("bright-purple", 13),
            ("brightpurple", 13),
            ("bright-cyan", 14),
            ("brightcyan", 14),
            ("bright-white", 15),
            ("brightwhite", 15),
        ]
        .into_iter()
        .collect()
    };
}

pub fn ansi_16_color_name_to_number(name: &str) -> Option<u8> {
    ANSI_16_COLORS.get(name).copied()
}

fn ansi_16_color_number_to_name(n: u8) -> Option<&'static str> {
    for (k, _n) in &*ANSI_16_COLORS {
        if *_n == n {
            return Some(&*k);
        }
    }
    None
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
