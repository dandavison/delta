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

use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::process;

use ansi_term;
use atty;
use structopt::StructOpt;
use syntect::highlighting::{Color, FontStyle, Style};

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

    let config = cli::process_command_line_arguments(&assets, &opt);

    if opt.show_background_colors {
        show_background_colors(&config);
        process::exit(0);
    }

    let mut output_type = OutputType::from_mode(PagingMode::QuitIfOneScreen, None).unwrap();
    let mut writer = output_type.handle().unwrap();

    if let Err(error) = delta(
        io::stdin().lock().lines().map(|l| l.unwrap()),
        &config,
        &assets,
        &mut writer,
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
        minus_color = color_to_hex(config.minus_style_modifier.background.unwrap()),
        minus_emph_color = color_to_hex(config.minus_emph_style_modifier.background.unwrap()),
        plus_color = color_to_hex(config.plus_style_modifier.background.unwrap()),
        plus_emph_color = color_to_hex(config.plus_emph_style_modifier.background.unwrap()),
    )
}

fn color_to_hex(color: Color) -> String {
    let mut string = String::new();
    let style = Style {
        foreground: style::NO_COLOR,
        background: color,
        font_style: FontStyle::empty(),
    };
    paint::paint_text(
        &format!("#{:x?}{:x?}{:x?}", color.r, color.g, color.b),
        style,
        &mut string,
    )
    .unwrap();
    string.push_str("\x1b[0m"); // reset
    string
}

fn compare_themes(assets: &HighlightingAssets) -> std::io::Result<()> {
    let opt = cli::Opt::from_args();
    let mut input = String::new();
    if atty::is(atty::Stream::Stdin) {
        input = "\
diff --git a/tests/data/hello.c b/tests/data/hello.c
index 541e930..e23bef1 100644
--- a/tests/data/hello.c
+++ b/tests/data/hello.c
@@ -1,5 +1,5 @@
 #include <stdio.h>

 int main(int argc, char **argv) {
-    printf(\"Hello!\\n\");
+    printf(\"Hello world!\\n\");
 }"
        .to_string()
    } else {
        io::stdin().read_to_string(&mut input)?;
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let style = ansi_term::Style::new().bold();

    for (theme, _) in assets.theme_set.themes.iter() {
        if opt.light && !style::is_light_theme(theme) || opt.dark && style::is_light_theme(theme) {
            continue;
        }

        writeln!(stdout, "\nTheme: {}\n", style.paint(theme))?;
        let new_opt = cli::Opt {
            theme: Some(theme.to_string()),
            ..opt.clone()
        };
        let config = cli::process_command_line_arguments(&assets, &new_opt);
        let mut output_type = OutputType::from_mode(PagingMode::QuitIfOneScreen, None).unwrap();
        let mut writer = output_type.handle().unwrap();

        if let Err(error) = delta(
            input.split('\n').map(String::from),
            &config,
            &assets,
            &mut writer,
        ) {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        };
    }
    Ok(())
}

pub fn list_themes() -> std::io::Result<()> {
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
