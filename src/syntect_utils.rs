use std::str::FromStr;

use syntect::highlighting::Color;

use crate::color;

pub fn syntect_color_from_ansi_name(name: &str) -> Option<Color> {
    color::ansi_16_color_name_to_number(name).and_then(syntect_color_from_ansi_number)
}

/// Convert 8-bit ANSI code to #RGBA string with ANSI code in red channel and 0 in alpha channel.
// See https://github.com/sharkdp/bat/pull/543
pub fn syntect_color_from_ansi_number(n: u8) -> Option<Color> {
    Color::from_str(&format!("#{:02x}000000", n)).ok()
}
