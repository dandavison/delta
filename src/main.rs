mod assets;
mod paint;
mod parse_diff;

use std::io::{self, BufRead, ErrorKind, Write};
use std::process;

use assets::{HighlightingAssets, list_languages, list_themes};
use console::strip_ansi_codes;
use structopt::StructOpt;
use syntect::parsing::SyntaxReference;

#[derive(StructOpt, Debug)]
#[structopt(name = "delta",
            about = "A syntax-highlighter for git. \
                     Use 'delta | less -R' as core.pager in .gitconfig")]
struct Opt {
    /// Use diff highlighting colors appropriate for a light terminal
    /// background. This is the default.
    #[structopt(long = "light")]
    light: bool,

    /// Use diff highlighting colors appropriate for a dark terminal
    /// background.
    #[structopt(long = "dark")]
    dark: bool,

    #[structopt(long = "plus-color")]
    /// The background color (RGB hex) to use for added lines. The
    /// default is "#d0ffd0" if you are using --light, and "#013B01"
    /// if you are using --dark.
    plus_color: Option<String>,

    #[structopt(long = "minus-color")]
    /// The background color (RGB hex) to use for removed lines. The
    /// default is "#ffd0d0" if you are using --light, and "#3f0001" if
    /// you are using --dark.
    minus_color: Option<String>,

    #[structopt(long = "theme")]
    /// The syntax highlighting theme to use. Options are Light:
    /// ("InspiredGitHub", "Solarized (light)", "base16-ocean.light"),
    /// Dark: ("Solarized, (dark)", "base16-eighties.dark",
    /// "base16-mocha.dark", "base16-ocean.dark").
    theme: Option<String>,

    /// The width (in characters) of the diff highlighting. By
    /// default, the highlighting extends to the last character on
    /// each line.
    #[structopt(short = "w", long = "width")]
    width: Option<String>,

    /// List supported languages and associated file extensions.
    #[structopt(long = "list-languages")]
    list_languages: bool,

    /// List available syntax highlighting themes.
    #[structopt(long = "list-themes")]
    list_themes: bool,
}

#[derive(PartialEq)]
enum State {
    Commit,
    DiffMeta,
    DiffHunk,
    Unknown,
}

fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_themes {
        list_themes()?;
        process::exit(0);
    }

    let assets = HighlightingAssets::new();
    let paint_config = parse_args(&assets, &opt);

    match delta(
        io::stdin().lock().lines().map(|l| l.unwrap()),
        &paint_config,
        &assets,
    ) {
        Err(error) => {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        }
        _ => (),
    };
    Ok(())
}

fn delta(
    lines: impl Iterator<Item = String>,
    paint_config: &paint::Config,
    assets: &HighlightingAssets,
) -> std::io::Result<()> {

    let mut syntax: Option<&SyntaxReference> = None;
    let mut output = String::new();
    let mut stdout = io::stdout();
    let mut state = State::Unknown;
    let mut did_emit_line: bool;

    for raw_line in lines {
        let line = strip_ansi_codes(&raw_line).to_string();
        did_emit_line = false;
        if line.starts_with("diff --") {
            state = State::DiffMeta;
            syntax = match parse_diff::get_file_extension_from_diff_line(&line) {
                // TODO: cache syntaxes?
                Some(extension) => assets.syntax_set.find_syntax_by_extension(extension),
                None => None,
            };
        } else if line.starts_with("commit") {
            state = State::Commit;
        } else if line.starts_with("@@") {
            state = State::DiffHunk;
        } else if state == State::DiffHunk {
            match syntax {
                Some(syntax) => {
                    paint::paint_line(line, syntax, &paint_config, &mut output);
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

fn parse_args<'a>(assets: &'a HighlightingAssets, opt: &'a Opt) -> paint::Config<'a> {

    if opt.light && opt.dark {
        eprintln!("--light and --dark cannot be used together.");
        process::exit(1);
    }

    let width = match opt.width.as_ref().map(String::as_str) {
        Some(width) => {
            Some(width.parse::<usize>().unwrap_or_else(
                |_| panic!("Invalid width: {}", width),
            ))
        }
        None => None,
    };

    paint::get_config(
        &assets.syntax_set,
        &opt.theme,
        &assets.theme_set,
        opt.light,
        &opt.plus_color,
        &opt.minus_color,
        width,
    )
}
