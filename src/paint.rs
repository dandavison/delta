use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub const LIGHT_THEMES: [&str; 3] = ["InspiredGitHub", "Solarized (light)", "base16-ocean.light"];

pub const DARK_THEMES: [&str; 4] = [
    "Solarized (dark)",
    "base16-eighties.dark",
    "base16-mocha.dark",
    "base16-ocean.dark",
];

const LIGHT_THEME_GREEN: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0x00,
};

const LIGHT_THEME_RED: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0x00,
};

const DARK_THEME_GREEN: Color = Color {
    r: 0x01,
    g: 0x18,
    b: 0x00,
    a: 0x00,
};

const DARK_THEME_RED: Color = Color {
    r: 0x24,
    g: 0x00,
    b: 0x01,
    a: 0x00,
};

/// Write line to buffer with color escape codes applied.
pub fn paint_line(
    mut line: String, // TODO: pass reference
    syntax: &SyntaxReference,
    syntax_set: &SyntaxSet,
    theme: &Theme,
    theme_name: &String,
    minus_color: &Option<String>,
    plus_color: &Option<String>,
    buf: &mut String,
) {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let first_char = line.chars().next();
    let is_dark = DARK_THEMES.contains(&theme_name.as_str());
    let background_color = match (first_char, is_dark) {
        (Some('+'), true) => Some(DARK_THEME_GREEN),
        (Some('-'), true) => Some(DARK_THEME_RED),
        (Some('+'), false) => Some(LIGHT_THEME_GREEN),
        (Some('-'), false) => Some(LIGHT_THEME_RED),
        _ => None,
    };
    if first_char == Some('+') || first_char == Some('-') {
        line = line[1..].to_string();
        buf.push_str(" ");
    }
    if line.len() < 100 {
        line = format!("{}{}", line, " ".repeat(100 - line.len()));
    }
    let ranges: Vec<(Style, &str)> = highlighter.highlight(&line, &syntax_set);
    paint_ranges(&ranges[..], background_color, buf);
}

/// Based on as_24_bit_terminal_escaped from syntect
fn paint_ranges(
    foreground_style_ranges: &[(Style, &str)],
    background_color: Option<Color>,
    buf: &mut String,
) -> () {
    for &(ref style, text) in foreground_style_ranges.iter() {
        paint(text, Some(style.foreground), background_color, false, buf);
    }
    buf.push_str("\x1b[0m");
}

/// Write text to buffer with color escape codes applied.
fn paint(
    text: &str,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    reset_color: bool,
    buf: &mut String,
) -> () {
    use std::fmt::Write;
    match background_color {
        Some(background_color) => {
            write!(
                buf,
                "\x1b[48;2;{};{};{}m",
                background_color.r,
                background_color.g,
                background_color.b
            ).unwrap();
            if reset_color {
                buf.push_str("\x1b[0m");
            }
        }
        None => (),
    }
    match foreground_color {
        Some(foreground_color) => {
            write!(
                buf,
                "\x1b[38;2;{};{};{}m{}",
                foreground_color.r,
                foreground_color.g,
                foreground_color.b,
                text
            ).unwrap();
            if reset_color {
                buf.push_str("\x1b[0m");
            }
        }
        None => {
            write!(buf, "{}", text).unwrap();
        }
    }
}
