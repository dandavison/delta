extern crate bitflags;

#[macro_use]
extern crate error_chain;

mod align;
mod ansi;
#[cfg(not(tarpaulin_include))]
mod bat_utils;
mod cli;
mod color;
mod config;
mod delta;
mod draw;
mod edits;
mod env;
mod features;
mod format;
mod git_config;
mod git_config_entry;
mod options;
mod paint;
mod parse;
mod parse_style;
mod style;
mod syntect_color;
mod tests;

use std::io::{self, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::process;

use bytelines::ByteLinesReader;
use itertools::Itertools;
use structopt::StructOpt;

use crate::bat_utils::assets::{list_languages, HighlightingAssets};
use crate::bat_utils::output::{OutputType, PagingMode};
use crate::config::delta_unreachable;
use crate::delta::delta;
use crate::options::theme::is_light_syntax_theme;

pub mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let opt = cli::Opt::from_args_and_git_config(&mut git_config::GitConfig::try_create(), assets);

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_syntax_themes {
        list_syntax_themes()?;
        process::exit(0);
    } else if opt.show_syntax_themes {
        show_syntax_themes()?;
        process::exit(0);
    }

    let _show_config = opt.show_config;
    let config = config::Config::from(opt);

    if _show_config {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        show_config(&config, &mut stdout)?;
        process::exit(0);
    }

    let mut output_type = OutputType::from_mode(config.paging_mode, None, &config).unwrap();
    let mut writer = output_type.handle().unwrap();

    if atty::is(atty::Stream::Stdin) {
        process::exit(diff(
            config.minus_file.as_ref(),
            config.plus_file.as_ref(),
            &config,
            &mut writer,
        ));
    }

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
    writer: &mut dyn Write,
) -> i32 {
    use std::io::BufReader;
    let die = || {
        eprintln!("Usage: delta minus_file plus_file");
        process::exit(config.error_exit_code);
    };
    let diff_command = "diff";
    let mut diff_process = process::Command::new(PathBuf::from(diff_command))
        .arg("-u")
        .args(&[
            minus_file.unwrap_or_else(die),
            plus_file.unwrap_or_else(die),
        ])
        .stdout(process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| {
            eprintln!("Failed to execute the command '{}': {}", diff_command, err);
            process::exit(config.error_exit_code);
        });
    let exit_code = diff_process
        .wait()
        .unwrap_or_else(|_| {
            delta_unreachable(&format!("'{}' process not running.", diff_command));
        })
        .code()
        .unwrap_or_else(|| {
            eprintln!("'{}' process terminated without exit status.", diff_command);
            process::exit(config.error_exit_code);
        });

    if let Err(error) = delta(
        BufReader::new(diff_process.stdout.unwrap()).byte_lines(),
        writer,
        &config,
    ) {
        match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => {
                eprintln!("{}", error);
                process::exit(config.error_exit_code);
            }
        }
    };
    exit_code
}

fn show_config(config: &config::Config, writer: &mut dyn Write) -> std::io::Result<()> {
    // styles first
    writeln!(
        writer,
        "    commit-style                  = {commit_style}
    file-style                    = {file_style}
    hunk-header-style             = {hunk_header_style}
    minus-style                   = {minus_style}
    minus-non-emph-style          = {minus_non_emph_style}
    minus-emph-style              = {minus_emph_style}
    minus-empty-line-marker-style = {minus_empty_line_marker_style}
    zero-style                    = {zero_style}
    plus-style                    = {plus_style}
    plus-non-emph-style           = {plus_non_emph_style}
    plus-emph-style               = {plus_emph_style}
    plus-empty-line-marker-style  = {plus_empty_line_marker_style}
    whitespace-error-style        = {whitespace_error_style}",
        commit_style = config.commit_style.to_painted_string(),
        file_style = config.file_style.to_painted_string(),
        hunk_header_style = config.hunk_header_style.to_painted_string(),
        minus_emph_style = config.minus_emph_style.to_painted_string(),
        minus_empty_line_marker_style = config.minus_empty_line_marker_style.to_painted_string(),
        minus_non_emph_style = config.minus_non_emph_style.to_painted_string(),
        minus_style = config.minus_style.to_painted_string(),
        plus_emph_style = config.plus_emph_style.to_painted_string(),
        plus_empty_line_marker_style = config.plus_empty_line_marker_style.to_painted_string(),
        plus_non_emph_style = config.plus_non_emph_style.to_painted_string(),
        plus_style = config.plus_style.to_painted_string(),
        whitespace_error_style = config.whitespace_error_style.to_painted_string(),
        zero_style = config.zero_style.to_painted_string(),
    )?;
    // Everything else
    writeln!(
        writer,
        "    24-bit-color                  = {true_color}
    file-added-label              = {file_added_label}
    file-modified-label           = {file_modified_label}
    file-removed-label            = {file_removed_label}
    file-renamed-label            = {file_renamed_label}",
        true_color = config.true_color,
        file_added_label = format_option_value(&config.file_added_label),
        file_modified_label = format_option_value(&config.file_modified_label),
        file_removed_label = format_option_value(&config.file_removed_label),
        file_renamed_label = format_option_value(&config.file_renamed_label),
    )?;
    writeln!(
        writer,
        "    hyperlinks                    = {hyperlinks}",
        hyperlinks = config.hyperlinks
    )?;
    if config.hyperlinks {
        writeln!(
            writer,
            "    hyperlinks-file-link-format   = {hyperlinks_file_link_format}",
            hyperlinks_file_link_format = format_option_value(&config.hyperlinks_file_link_format),
        )?
    }
    writeln!(
        writer,
        "    inspect-raw-lines             = {inspect_raw_lines}
    keep-plus-minus-markers       = {keep_plus_minus_markers}",
        inspect_raw_lines = match config.inspect_raw_lines {
            cli::InspectRawLines::True => "true",
            cli::InspectRawLines::False => "false",
        },
        keep_plus_minus_markers = config.keep_plus_minus_markers,
    )?;
    writeln!(
        writer,
        "    line-numbers                  = {line_numbers}",
        line_numbers = config.line_numbers
    )?;
    if config.line_numbers {
        writeln!(
            writer,
            "    line-numbers-minus-style      = {line_numbers_minus_style}
    line-numbers-zero-style       = {line_numbers_zero_style}
    line-numbers-plus-style       = {line_numbers_plus_style}
    line-numbers-left-style       = {line_numbers_left_style}
    line-numbers-right-style      = {line_numbers_right_style}
    line-numbers-left-format      = {line_numbers_left_format}
    line-numbers-right-format     = {line_numbers_right_format}",
            line_numbers_minus_style = config.line_numbers_minus_style.to_painted_string(),
            line_numbers_zero_style = config.line_numbers_zero_style.to_painted_string(),
            line_numbers_plus_style = config.line_numbers_plus_style.to_painted_string(),
            line_numbers_left_style = config.line_numbers_left_style.to_painted_string(),
            line_numbers_right_style = config.line_numbers_right_style.to_painted_string(),
            line_numbers_left_format = format_option_value(&config.line_numbers_left_format),
            line_numbers_right_format = format_option_value(&config.line_numbers_right_format),
        )?
    }
    writeln!(
        writer,
        "    max-line-distance             = {max_line_distance}
    max-line-length               = {max_line_length}
    navigate                      = {navigate}
    paging                        = {paging_mode}
    side-by-side                  = {side_by_side}
    syntax-theme                  = {syntax_theme}
    width                         = {width}
    tabs                          = {tab_width}
    word-diff-regex               = {tokenization_regex}",
        max_line_distance = config.max_line_distance,
        max_line_length = config.max_line_length,
        navigate = config.navigate,
        paging_mode = match config.paging_mode {
            PagingMode::Always => "always",
            PagingMode::Never => "never",
            PagingMode::QuitIfOneScreen => "auto",
        },
        side_by_side = config.side_by_side,
        syntax_theme = config
            .syntax_theme
            .clone()
            .map(|t| t.name.unwrap_or_else(|| "none".to_string()))
            .unwrap_or_else(|| "none".to_string()),
        width = match config.decorations_width {
            cli::Width::Fixed(width) => width.to_string(),
            cli::Width::Variable => "variable".to_string(),
        },
        tab_width = config.tab_width,
        tokenization_regex = format_option_value(&config.tokenization_regex.to_string()),
    )?;
    Ok(())
}

// Heuristics determining whether to quote string option values when printing values intended for
// git config.
fn format_option_value<S>(s: S) -> String
where
    S: AsRef<str>,
{
    let s = s.as_ref();
    if s.ends_with(' ')
        || s.starts_with(' ')
        || s.contains(&['\\', '{', '}', ':'][..])
        || s.is_empty()
    {
        format!("'{}'", s)
    } else {
        s.to_string()
    }
}

#[cfg(not(tarpaulin_include))]
fn show_syntax_themes() -> std::io::Result<()> {
    let mut opt = cli::Opt::from_args();
    let assets = HighlightingAssets::new();
    let mut output_type = OutputType::from_mode(
        PagingMode::QuitIfOneScreen,
        None,
        &config::Config::from(cli::Opt::default()),
    )
    .unwrap();
    let mut writer = output_type.handle().unwrap();
    opt.computed.syntax_set = assets.syntax_set;

    if !(opt.dark || opt.light) {
        _show_syntax_themes(opt.clone(), false, &mut writer)?;
        _show_syntax_themes(opt, true, &mut writer)?;
    } else if opt.light {
        _show_syntax_themes(opt, true, &mut writer)?;
    } else {
        _show_syntax_themes(opt, false, &mut writer)?
    };
    Ok(())
}

fn _show_syntax_themes(
    mut opt: cli::Opt,
    is_light_mode: bool,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    use bytelines::ByteLines;
    use std::io::BufReader;
    let mut input = b"\
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
"
    .to_vec();
    if !atty::is(atty::Stream::Stdin) {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        if !buf.is_empty() {
            input = buf;
        }
    };

    opt.computed.is_light_mode = is_light_mode;
    let mut config = config::Config::from(opt);
    let title_style = ansi_term::Style::new().bold();
    let assets = HighlightingAssets::new();

    for syntax_theme in assets
        .theme_set
        .themes
        .iter()
        .filter(|(t, _)| is_light_syntax_theme(t) == is_light_mode)
        .map(|(t, _)| t)
    {
        writeln!(writer, "\n\nTheme: {}\n", title_style.paint(syntax_theme))?;
        config.syntax_theme = Some(assets.theme_set.themes[syntax_theme.as_str()].clone());
        if let Err(error) = delta(ByteLines::new(BufReader::new(&input[0..])), writer, &config) {
            match error.kind() {
                ErrorKind::BrokenPipe => process::exit(0),
                _ => eprintln!("{}", error),
            }
        };
    }
    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn list_syntax_themes() -> std::io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if atty::is(atty::Stream::Stdout) {
        _list_syntax_themes_for_humans(&mut stdout)
    } else {
        _list_syntax_themes_for_machines(&mut stdout)
    }
}

pub fn _list_syntax_themes_for_humans(writer: &mut dyn Write) -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;

    writeln!(writer, "Light themes:")?;
    for (theme, _) in themes.iter().filter(|(t, _)| is_light_syntax_theme(*t)) {
        writeln!(writer, "    {}", theme)?;
    }
    writeln!(writer, "\nDark themes:")?;
    for (theme, _) in themes.iter().filter(|(t, _)| !is_light_syntax_theme(*t)) {
        writeln!(writer, "    {}", theme)?;
    }
    writeln!(
        writer,
        "\nUse delta --show-syntax-themes to demo the themes."
    )?;
    Ok(())
}

pub fn _list_syntax_themes_for_machines(writer: &mut dyn Write) -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;
    for (theme, _) in themes
        .iter()
        .sorted_by_key(|(t, _)| is_light_syntax_theme(*t))
    {
        writeln!(
            writer,
            "{}\t{}",
            if is_light_syntax_theme(theme) {
                "light"
            } else {
                "dark"
            },
            theme
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use std::io::{Cursor, Seek, SeekFrom};

    use crate::ansi;
    use crate::tests::integration_test_utils::integration_test_utils;

    #[test]
    fn test_show_config() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![0; 1024]);
        show_config(&config, &mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("    commit-style                  = raw\n"));
        assert!(s.contains(r"    word-diff-regex               = '\w+'"));
    }

    #[test]
    #[ignore] // Not working (timing out) when run by tarpaulin, presumably due to stdin detection.
    fn test_show_syntax_themes() {
        let opt = integration_test_utils::make_options_from_args(&[]);

        let mut writer = Cursor::new(vec![0; 1024]);
        _show_syntax_themes(opt, true, &mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("\nTheme: gruvbox-white\n"));
        println!("{}", s);
        assert!(s.contains("\nfn print_cube(num: f64) {\n"));
    }

    #[test]
    fn test_list_syntax_themes_for_humans() {
        let mut writer = Cursor::new(vec![0; 512]);
        _list_syntax_themes_for_humans(&mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("Light themes:\n"));
        assert!(s.contains("    GitHub\n"));
        assert!(s.contains("Dark themes:\n"));
        assert!(s.contains("    Dracula\n"));
    }

    #[test]
    fn test_list_syntax_themes_for_machines() {
        let mut writer = Cursor::new(vec![0; 512]);
        _list_syntax_themes_for_machines(&mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("light	GitHub\n"));
        assert!(s.contains("dark	Dracula\n"));
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_same_empty_file() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![]);
        let exit_code = diff(
            Some(&PathBuf::from("/dev/null")),
            Some(&PathBuf::from("/dev/null")),
            &config,
            &mut writer,
        );
        assert_eq!(exit_code, 0);
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.is_empty());
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_same_non_empty_file() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![]);
        let exit_code = diff(
            Some(&PathBuf::from("/etc/passwd")),
            Some(&PathBuf::from("/etc/passwd")),
            &config,
            &mut writer,
        );
        assert_eq!(exit_code, 0);
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.is_empty());
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn test_diff_differing_files() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let mut writer = Cursor::new(vec![]);
        let exit_code = diff(
            Some(&PathBuf::from("/dev/null")),
            Some(&PathBuf::from("/etc/passwd")),
            &config,
            &mut writer,
        );
        assert_eq!(exit_code, 1);
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("comparing: /dev/null ⟶   /etc/passwd\n"));
    }
}
