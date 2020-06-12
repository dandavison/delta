extern crate bitflags;

#[macro_use]
extern crate error_chain;

mod align;
mod bat;
mod cli;
mod color;
mod config;
mod delta;
mod draw;
mod edits;
mod env;
#[macro_use]
mod gitconfig;
mod paint;
mod parse;
mod preset;
mod rewrite;
mod style;
mod syntax_theme;
mod syntect_color;
mod tests;

use std::io::{self, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::process;

use ansi_term::{self, Color};
use atty;
use bytelines::ByteLinesReader;
use structopt::StructOpt;

use crate::bat::assets::{list_languages, HighlightingAssets};
use crate::bat::output::{OutputType, PagingMode};
use crate::delta::delta;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut git_config = match std::env::current_dir() {
        Ok(dir) => match git2::Repository::discover(dir) {
            Ok(repo) => match repo.config() {
                Ok(config) => Some(config),
                Err(_) => None,
            },
            Err(_) => None,
        },
        Err(_) => None,
    };

    let config = config::Config::from_arg_matches(cli::Opt::clap().get_matches(), &mut git_config);

    if config.list_languages {
        list_languages()?;
        process::exit(0);
    } else if config.list_syntax_theme_names {
        list_syntax_theme_names()?;
        process::exit(0);
    } else if config.list_syntax_themes {
        list_syntax_themes()?;
        process::exit(0);
    } else if config.show_background_colors {
        show_background_colors(&config);
        process::exit(0);
    } else if atty::is(atty::Stream::Stdin) {
        return diff(
            config.minus_file.as_ref(),
            config.plus_file.as_ref(),
            &config,
        );
    }

    let mut output_type = OutputType::from_mode(config.paging_mode, None, &config).unwrap();
    let mut writer = output_type.handle().unwrap();

    if let Err(error) = delta(io::stdin().lock().byte_lines(), &mut writer, &config) {
        match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => eprintln!("{}", error),
        }
    };
    Ok(())
}

/// Run `diff -u` on the files provided on the command line and display the output.
fn diff(
    minus_file: Option<&PathBuf>,
    plus_file: Option<&PathBuf>,
    config: &config::Config,
) -> std::io::Result<()> {
    use std::io::BufReader;
    let die = || {
        eprintln!("Usage: delta minus_file plus_file");
        process::exit(1);
    };
    let diff_process = process::Command::new(PathBuf::from("diff"))
        .arg("-u")
        .args(&[
            minus_file.unwrap_or_else(die),
            plus_file.unwrap_or_else(die),
        ])
        .stdout(process::Stdio::piped())
        .spawn();

    let mut output_type = OutputType::from_mode(config.paging_mode, None, &config).unwrap();
    let mut writer = output_type.handle().unwrap();
    if let Err(error) = delta(
        BufReader::new(diff_process.unwrap().stdout.unwrap()).byte_lines(),
        &mut writer,
        &config,
    ) {
        match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => eprintln!("{}", error),
        }
    };
    Ok(())
}

fn show_background_colors(config: &config::Config) {
    println!(
        "delta \
         --minus-color=\"{minus_color}\" \
         --minus-emph-color=\"{minus_emph_color}\" \
         --plus-color=\"{plus_color}\" \
         --plus-emph-color=\"{plus_emph_color}\"",
        minus_color =
            get_painted_rgb_string(config.minus_style.ansi_term_style.background.unwrap()),
        minus_emph_color =
            get_painted_rgb_string(config.minus_emph_style.ansi_term_style.background.unwrap()),
        plus_color = get_painted_rgb_string(config.plus_style.ansi_term_style.background.unwrap()),
        plus_emph_color =
            get_painted_rgb_string(config.plus_emph_style.ansi_term_style.background.unwrap()),
    )
}

fn get_painted_rgb_string(color: Color) -> String {
    color.paint(format!("{:?}", color)).to_string()
}

fn list_syntax_themes() -> std::io::Result<()> {
    use bytelines::ByteLines;
    use std::io::BufReader;
    let opt = cli::Opt::from_args();
    let input = if !atty::is(atty::Stream::Stdin) {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        buf
    } else {
        b"\
diff --git a/example.rs b/example.rs
index f38589a..0f1bb83 100644
--- a/example.rs
+++ b/example.rs
@@ -1,5 +1,5 @@
-// Output the square of a number.
-fn print_square(num: f64) {
-    let result = f64::powf(num, 2.0);
-    println!(\"The square of {:.2} is {:.2}.\", num, result);
+// Output the cube of a number.
+fn print_cube(num: f64) {
+    let result = f64::powf(num, 3.0);
+    println!(\"The cube of {:.2} is {:.2}.\", num, result);
 }"
        .to_vec()
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let style = ansi_term::Style::new().bold();

    let assets = HighlightingAssets::new();

    for (syntax_theme, _) in assets.theme_set.themes.iter() {
        if opt.light && !syntax_theme::is_light_theme(syntax_theme)
            || opt.dark && syntax_theme::is_light_theme(syntax_theme)
        {
            continue;
        }

        writeln!(stdout, "\n\nTheme: {}\n", style.paint(syntax_theme))?;
        let config = config::Config::from_args(
            &[
                "--syntax-theme",
                syntax_theme,
                "--file-style",
                "omit",
                "--hunk-header-style",
                "omit",
            ],
            &mut None,
        );
        let mut output_type =
            OutputType::from_mode(PagingMode::QuitIfOneScreen, None, &config).unwrap();
        let mut writer = output_type.handle().unwrap();

        if let Err(error) = delta(
            ByteLines::new(BufReader::new(&input[0..])),
            &mut writer,
            &config,
        ) {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        };
    }
    Ok(())
}

pub fn list_syntax_theme_names() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    writeln!(stdout, "Light themes:")?;
    for (theme, _) in themes.iter() {
        if syntax_theme::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    writeln!(stdout, "Dark themes:")?;
    for (theme, _) in themes.iter() {
        if !syntax_theme::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    Ok(())
}
