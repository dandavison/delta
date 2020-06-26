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
mod features;
mod get_option_value;
mod git_config;
mod option_value;
mod paint;
mod parse;
mod parse_style;
mod rewrite_options;
mod set_options;
mod style;
mod syntax_theme;
mod syntect_color;
mod tests;

use std::io::{self, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::process;

use ansi_term;
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
    let opt = cli::Opt::from_args_and_git_config(&mut git_config::GitConfig::try_create());
    let config = config::Config::from(opt);

    if config.list_languages {
        list_languages()?;
        process::exit(0);
    } else if config.list_syntax_theme_names {
        list_syntax_theme_names()?;
        process::exit(0);
    } else if config.list_syntax_themes {
        list_syntax_themes()?;
        process::exit(0);
    } else if config.show_styles {
        show_styles(&config);
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

fn show_styles(config: &config::Config) {
    print!(
        "\
--commit-style {commit_style}
--file-style {file_style}
--hunk-header-style {hunk_header_style}
--minus-style {minus_style}
--minus-non-emph-style {minus_non_emph_style}
--minus-emph-style {minus_emph_style}
--minus-empty-line-marker-style {minus_empty_line_marker_style}
--zero-style {zero_style}
--plus-style {plus_style}
--plus-non-emph-style {plus_non_emph_style}
--plus-emph-style {plus_emph_style}
--plus-empty-line-marker-style {plus_empty_line_marker_style}
--whitespace-error-style {whitespace_error_style}",
        minus_style = config.minus_style.to_painted_string(),
        zero_style = config.zero_style.to_painted_string(),
        plus_style = config.plus_style.to_painted_string(),
        minus_emph_style = config.minus_emph_style.to_painted_string(),
        minus_non_emph_style = config.minus_non_emph_style.to_painted_string(),
        plus_emph_style = config.plus_emph_style.to_painted_string(),
        plus_non_emph_style = config.plus_non_emph_style.to_painted_string(),
        commit_style = config.commit_style.to_painted_string(),
        file_style = config.file_style.to_painted_string(),
        hunk_header_style = config.hunk_header_style.to_painted_string(),
        minus_empty_line_marker_style = config.minus_empty_line_marker_style.to_painted_string(),
        plus_empty_line_marker_style = config.plus_empty_line_marker_style.to_painted_string(),
        whitespace_error_style = config.whitespace_error_style.to_painted_string(),
    );
    if config.line_numbers {
        print!(
            "\
--line-numbers-minus-style {line_numbers_minus_style}
--line-numbers-zero-style {line_numbers_zero_style}
--line-numbers-plus-style {line_numbers_plus_style}
--line-numbers-left-style {line_numbers_left_style}
--line-numbers-right-style {line_numbers_right_style}",
            line_numbers_minus_style = config.line_numbers_minus_style.to_painted_string(),
            line_numbers_zero_style = config.line_numbers_zero_style.to_painted_string(),
            line_numbers_plus_style = config.line_numbers_plus_style.to_painted_string(),
            line_numbers_left_style = config.line_numbers_left_style.to_painted_string(),
            line_numbers_right_style = config.line_numbers_right_style.to_painted_string(),
        )
    }
    println!();
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

        let opt_2 = cli::Opt::from_iter(&[
            "--syntax-theme",
            syntax_theme,
            "--file-style",
            "omit",
            "--hunk-header-style",
            "omit",
        ]);
        let config = config::Config::from(opt_2);
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
