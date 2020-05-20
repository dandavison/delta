use std::str::FromStr;

use syntect::highlighting::Color;

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

pub fn syntect_color_from_ansi_name(name: &str) -> Option<Color> {
    ansi_color_name_to_number(name).and_then(syntect_color_from_ansi_number)
}

/// Convert 8-bit ANSI code to #RGBA string with ANSI code in red channel and 0 in alpha channel.
// See https://github.com/sharkdp/bat/pull/543
pub fn syntect_color_from_ansi_number(n: u8) -> Option<Color> {
    Color::from_str(&format!("#{:02x}000000", n)).ok()
}
