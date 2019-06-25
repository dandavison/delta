use std::fmt::Write;
use std::io::{self, BufRead};
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

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
    let mut highlighter: Option<HighlightLines> = None;

    for _line in io::stdin().lock().lines() {
        let line = _line.unwrap();
        if line.starts_with("diff --") {
            state = State::DiffMeta;
            let extension = get_file_extension_from_diff_line(&line);
            highlighter = match ps.find_syntax_by_extension(extension) {
                Some(syntax) => Some(HighlightLines::new(syntax, theme)),
                None => None,
            }
        } else if line.starts_with("commit") {
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::DiffHunk;
        } else if state == State::DiffHunk {
            match highlighter {
                Some(mut highlighter) => {
                    let background_color = match line.chars().next() {
                        Some('+') => Some(GREEN),
                        Some('-') => Some(RED),
                        _ => None,
                    };
                    let ranges: Vec<(Style, &str)> = highlighter.highlight(&line, &ps);
                    my_as_24_bit_terminal_escaped(&ranges[..], background_color, &mut output);
                    print!("{}", output);
                    output.truncate(0);
                }
                None => {
                    print!("{}", line);
                }
            }
        };
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



fn get_file_extension_from_diff_line(line: &String) -> &str {

    //let (old, new) = get_file_names_from_diff_line(&line);
    let path = Path::new(line);
    let extension =
        path.extension().expect(
            format!("Error determining file type: {}", path.to_str().unwrap()).as_str(),
        );
    extension.to_str().unwrap()
}
