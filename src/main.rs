use std::fmt::Write;
use std::io::{self, BufRead};
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub const DELTA_THEME_DEFAULT: &str = "InspiredGitHub"; // base16-mocha.dark

const GREEN: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0x00,
};

const RED: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0x00,
};

#[derive(PartialEq)]
enum State {
    Commit,
    DiffMeta,
    DiffHunk,
    Unknown,
}

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes[DELTA_THEME_DEFAULT];
    let mut output = String::new();
    let mut state = State::Unknown;
    let mut syntax: Option<&SyntaxReference> = None;

    for _line in io::stdin().lock().lines() {
        let line = _line.unwrap();
        if line.starts_with("diff --") {
            state = State::DiffMeta;
            syntax = match get_file_extension_from_diff_line(&line) {
                Some(extension) => ps.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::DiffHunk;
        } else if state == State::DiffHunk {
            match syntax {
                Some(syntax) => {
                    let mut highlighter = HighlightLines::new(syntax, theme);
                    let background_color = match line.chars().next() {
                        Some('+') => Some(GREEN),
                        Some('-') => Some(RED),
                        _ => None,
                    };
                    let ranges: Vec<(Style, &str)> = highlighter.highlight(&line, &ps);
                    my_as_24_bit_terminal_escaped(&ranges[..], background_color, &mut output);
                    println!("{}", output);
                    output.truncate(0);
                }
                None => println!("{}", line),
            }
        }
    }
}

/// Based on as_24_bit_terminal_escaped from syntect
fn my_as_24_bit_terminal_escaped(
    v: &[(Style, &str)],
    background_color: Option<Color>,
    buf: &mut String,
) -> () {
    for &(ref style, text) in v.iter() {
        match background_color {
            Some(background_color) => {
                write!(
                    buf,
                    "\x1b[48;2;{};{};{}m",
                    background_color.r,
                    background_color.g,
                    background_color.b
                ).unwrap()
            }
            None => (),
        }
        write!(
            buf,
            "\x1b[38;2;{};{};{}m{}",
            style.foreground.r,
            style.foreground.g,
            style.foreground.b,
            text
        ).unwrap();
    }
    buf.push_str("\x1b[0m");
}

/// Given input like
/// "diff --git a/src/main.rs b/src/main.rs"
/// Return "rs", i.e. a single file extension consistent with both files.
fn get_file_extension_from_diff_line(line: &str) -> Option<&str> {
    match get_file_extensions_from_diff_line(line) {
        (Some(ext1), Some(ext2)) => {
            if ext1 == ext2 {
                Some(ext1)
            } else {
                // TODO: Just return None and output without color.
                panic!(
                    "Old and new files have different extensions: {} vs {}",
                    ext1,
                    ext2
                );
            }
        }
        (Some(ext1), None) => Some(ext1),
        (None, Some(ext2)) => Some(ext2),
        (None, None) => None,
    }
}

/// Given input like "diff --git a/src/main.rs b/src/main.rs"
/// return ("src/main.rs", "src/main.rs").
fn get_file_extensions_from_diff_line(line: &str) -> (Option<&str>, Option<&str>) {
    let mut iter = line.split(" ");
    iter.next(); // diff
    iter.next(); // --git
    (
        iter.next().and_then(|s| get_extension(&s[2..])),
        iter.next().and_then(|s| get_extension(&s[2..])),
    )
}

/// Attempt to parse input as a file path and return extension as a &str.
fn get_extension(s: &str) -> Option<&str> {
    Path::new(s).extension().and_then(|e| e.to_str())
}
