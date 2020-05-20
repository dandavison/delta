#[macro_use]
extern crate error_chain;

mod align;
mod bat;
mod cli;
mod config;
mod delta;
mod draw;
mod edits;
mod env;
mod paint;
mod parse;
mod style;
mod syntect_color;
mod tests;

use std::io::{self, ErrorKind, Read, Write};
use std::process;

use ansi_term::{Color, Style};
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
    let opt = cli::Opt::from_args();

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_theme_names {
        list_theme_names()?;
        process::exit(0);
    } else if opt.list_themes {
        list_themes()?;
        process::exit(0);
    }

    let show_background_colors_option = opt.show_background_colors;

    let config = cli::process_command_line_arguments(opt);

    if show_background_colors_option {
        show_background_colors(&config);
        process::exit(0);
    }

    let mut output_type = OutputType::from_mode(config.paging_mode, None).unwrap();
    let mut writer = output_type.handle().unwrap();

    if let Err(error) = delta(io::stdin().lock().byte_lines(), &mut writer, &config) {
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
        minus_color = get_painted_rgb_string(config.minus_style.background.unwrap()),
        minus_emph_color = get_painted_rgb_string(config.minus_emph_style.background.unwrap()),
        plus_color = get_painted_rgb_string(config.plus_style.background.unwrap()),
        plus_emph_color = get_painted_rgb_string(config.plus_emph_style.background.unwrap()),
    )
}

fn get_painted_rgb_string(color: Color) -> String {
    color.paint(format!("{:?}", color)).to_string()
}

fn list_themes() -> std::io::Result<()> {
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
    let style = Style::new().bold();

    let assets = HighlightingAssets::new();

    for (theme, _) in assets.theme_set.themes.iter() {
        if opt.light && !style::is_light_theme(theme) || opt.dark && style::is_light_theme(theme) {
            continue;
        }

        writeln!(stdout, "\n\nTheme: {}\n", style.paint(theme))?;
        let mut config = cli::process_command_line_arguments(cli::Opt {
            theme: Some(theme.to_string()),
            ..opt.clone()
        });
        config.file_style = cli::SectionStyle::Omit;
        config.hunk_style = cli::SectionStyle::Omit;
        let mut output_type = OutputType::from_mode(PagingMode::QuitIfOneScreen, None).unwrap();
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

pub fn list_theme_names() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    writeln!(stdout, "Light themes:")?;
    for (theme, _) in themes.iter() {
        if style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    writeln!(stdout, "Dark themes:")?;
    for (theme, _) in themes.iter() {
        if !style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    Ok(())
}
