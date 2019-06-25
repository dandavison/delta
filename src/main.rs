extern crate unidiff;

use std::fmt::Write;
use std::io::{self, Read};
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use unidiff::PatchSet;

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

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes[DELTA_THEME_DEFAULT];
    let mut input = String::new();
    let mut output = String::new();

    io::stdin().read_to_string(&mut input).expect(
        "Error reading input",
    );
    let mut patch_set = PatchSet::new();
    patch_set.parse(&mut input).ok().expect(
        "Error parsing input as a diff",
    );
    for patched_file in patch_set {
        // TODO: use both source and target to determine language
        let path = Path::new(&patched_file.target_file);
        let extension =
            path.extension().expect(
                format!("Error determining file type: {}", path.to_str().unwrap())
                    .as_str(),
            );
        let extension_str = extension.to_str().unwrap();
        let syntax = ps.find_syntax_by_extension(extension_str);

        match syntax {
            Some(syntax) => {
                let mut highlighter = HighlightLines::new(syntax, theme);
                for hunk in patched_file {
                    for line in LinesWithEndings::from(&hunk.to_string()) {
                        let background_color = match line.chars().next() {
                            Some('+') => Some(GREEN),
                            Some('-') => Some(RED),
                            _ => None,
                        };
                        let ranges: Vec<(Style, &str)> = highlighter.highlight(line, &ps);
                        my_as_24_bit_terminal_escaped(&ranges[..], background_color, &mut output);
                        print!("{}", output);
                        output.truncate(0);
                    }
                }
            }
            None => {
                for hunk in patched_file {
                    for line in LinesWithEndings::from(&hunk.to_string()) {
                        print!("{}", line);
                    }
                }
            }
        }
    }
    println!("");
}

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
