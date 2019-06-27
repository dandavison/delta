extern crate structopt;

mod paint;
mod parse_diff;

use std::io::{self, BufRead, ErrorKind};
use std::process;

use console::strip_ansi_codes;
use structopt::StructOpt;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub const DELTA_THEME_DEFAULT: &str = "base16-mocha.dark";

#[derive(StructOpt, Debug)]
#[structopt(name = "delta")]
struct Opt {
    /// Use diff highlighting colors appropriate for a light terminal
    /// background
    #[structopt(long = "light")]
    light: bool,

    /// Use diff highlighting colors appropriate for a dark terminal
    /// background
    #[structopt(long = "dark")]
    dark: bool,

    /// The width (in characters) of the diff highlighting. By
    /// default, the highlighting extends to the last character on
    /// each line
    #[structopt(short = "-w", long = "width")]
    width: Option<u16>,
}

#[derive(PartialEq)]
enum State {
    Commit,
    DiffMeta,
    DiffHunk,
    Unknown,
}

fn main() {
    match delta() {
        Err(error) => {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        }
        _ => (),
    }
}

fn delta() -> std::io::Result<()> {
    use std::io::Write;
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let theme = &theme_set.themes[DELTA_THEME_DEFAULT];
    let mut output = String::new();
    let mut state = State::Unknown;
    let mut syntax: Option<&SyntaxReference> = None;
    let mut did_emit_line: bool;
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for _line in stdin.lock().lines() {
        let raw_line = _line?;
        let line: String = strip_ansi_codes(&raw_line).to_string();
        did_emit_line = false;
        if line.starts_with("diff --") {
            state = State::DiffMeta;
            syntax = match parse_diff::get_file_extension_from_diff_line(&line) {
                Some(extension) => syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::DiffHunk;
        } else if state == State::DiffHunk {
            match syntax {
                Some(syntax) => {
                    paint::paint_line(line, syntax, &syntax_set, theme, &mut output);
                    writeln!(stdout, "{}", output)?;
                    output.truncate(0);
                    did_emit_line = true;
                }
                None => (),
            }
        }
        if !did_emit_line {
            writeln!(stdout, "{}", raw_line)?;
        }
    }
    Ok(())
}
