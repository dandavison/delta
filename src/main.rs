mod assets;
mod paint;
mod parse_diff;

use std::io::{self, BufRead, ErrorKind, Read, Write};
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
    /// Use colors appropriate for a light terminal background. For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "light")]
    light: bool,

    /// Use colors appropriate for a dark terminal background.  For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "dark")]
    dark: bool,

    #[structopt(long = "plus-color")]
    /// The background color (RGB hex) to use for added lines.
    plus_color: Option<String>,

    #[structopt(long = "minus-color")]
    /// The background color (RGB hex) to use for removed lines.
    minus_color: Option<String>,

    #[structopt(long = "theme")]
    /// The syntax highlighting theme to use.
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

    /// Compare available syntax highlighting themes. To use this
    /// option, supply git diff output to delta on standard input.
    /// For example: `git show --color=always | delta --compare-themes`.
    #[structopt(long = "compare-themes")]
    compare_themes: bool,
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

    let assets = HighlightingAssets::new();

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_themes {
        list_themes()?;
        process::exit(0);
    } else if opt.compare_themes {
        compare_themes(&assets)?;
        process::exit(0);
    }

    let paint_config = process_command_line_arguments(&assets, &opt);

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

fn process_command_line_arguments<'a>(
    assets: &'a HighlightingAssets,
    opt: &'a Opt,
) -> paint::Config<'a> {

    if opt.light && opt.dark {
        eprintln!("--light and --dark cannot be used together.");
        process::exit(1);
    }
    match &opt.theme {
        Some(theme) => {
            if !assets.theme_set.themes.contains_key(theme.as_str()) {
                eprintln!("Invalid theme: '{}'", theme);
                process::exit(1);
            }
            let is_light_theme = paint::LIGHT_THEMES.contains(&theme.as_str());
            if is_light_theme && opt.dark {
                eprintln!(
                    "{} is a light theme, but you supplied --dark. \
                     If you use --theme, you do not need to supply --light or --dark.",
                    theme
                );
                process::exit(1);
            } else if !is_light_theme && opt.light {
                eprintln!(
                    "{} is a dark theme, but you supplied --light. \
                     If you use --theme, you do not need to supply --light or --dark.",
                    theme
                );
                process::exit(1);
            }
        }
        None => (),
    };

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

fn compare_themes(assets: &HighlightingAssets) -> std::io::Result<()> {
    let mut opt = Opt::from_args();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut paint_config: paint::Config;
    let hline = "-".repeat(100);

    for (theme, _) in assets.theme_set.themes.iter() {
        writeln!(stdout, "{}\n{}\n{}\n", hline, theme, hline)?;
        opt.theme = Some(theme.to_string());
        paint_config = process_command_line_arguments(&assets, &opt);
        delta(input.split("\n").map(String::from), &paint_config, &assets)?;
    }

    Ok(())
}
