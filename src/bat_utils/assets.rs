// Based on code from https://github.com/sharkdp/bat a1b9334a44a2c652f52dddaa83dbacba57372468
// See src/bat_utils/LICENSE

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;

use ansi_term::Colour::Green;
use ansi_term::Style;
use syntect::dumps::{from_binary, from_reader};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::bat_utils::dirs::PROJECT_DIRS;
use crate::errors::*;

pub struct HighlightingAssets {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

impl HighlightingAssets {
    pub fn new() -> Self {
        Self::from_cache().unwrap_or_else(|_| Self::from_binary())
    }

    fn get_integrated_syntaxset() -> SyntaxSet {
        from_binary(include_bytes!("../../etc/assets/syntaxes.bin"))
    }

    fn get_integrated_themeset() -> ThemeSet {
        from_binary(include_bytes!("../../etc/assets/themes.bin"))
    }

    fn from_cache() -> Result<Self> {
        let theme_set_path = theme_set_path();
        let syntax_set_file = File::open(&syntax_set_path()).chain_err(|| {
            format!(
                "Could not load cached syntax set '{}'",
                syntax_set_path().to_string_lossy()
            )
        })?;
        let syntax_set: SyntaxSet = from_reader(BufReader::new(syntax_set_file))
            .chain_err(|| "Could not parse cached syntax set")?;

        let theme_set_file = File::open(&theme_set_path).chain_err(|| {
            format!(
                "Could not load cached theme set '{}'",
                theme_set_path.to_string_lossy()
            )
        })?;
        let theme_set: ThemeSet = from_reader(BufReader::new(theme_set_file))
            .chain_err(|| "Could not parse cached theme set")?;

        Ok(HighlightingAssets {
            syntax_set,
            theme_set,
        })
    }

    fn from_binary() -> Self {
        let syntax_set = Self::get_integrated_syntaxset();
        let theme_set = Self::get_integrated_themeset();

        HighlightingAssets {
            syntax_set,
            theme_set,
        }
    }
}

fn theme_set_path() -> PathBuf {
    PROJECT_DIRS.cache_dir().join("themes.bin")
}

fn syntax_set_path() -> PathBuf {
    PROJECT_DIRS.cache_dir().join("syntaxes.bin")
}

pub fn list_languages() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let mut languages = assets
        .syntax_set
        .syntaxes()
        .iter()
        .filter(|syntax| !syntax.hidden && !syntax.file_extensions.is_empty())
        .collect::<Vec<_>>();
    languages.sort_by_key(|lang| lang.name.to_uppercase());

    let loop_through = false;
    let colored_output = true;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if loop_through {
        for lang in languages {
            writeln!(stdout, "{}:{}", lang.name, lang.file_extensions.join(","))?;
        }
    } else {
        let longest = languages
            .iter()
            .map(|syntax| syntax.name.len())
            .max()
            .unwrap_or(32); // Fallback width if they have no language definitions.

        let comma_separator = ", ";
        let separator = " ";
        // Line-wrapping for the possible file extension overflow.
        let desired_width = 100;

        let style = if colored_output {
            Green.normal()
        } else {
            Style::default()
        };

        for lang in languages {
            write!(stdout, "{:width$}{}", lang.name, separator, width = longest)?;

            // Number of characters on this line so far, wrap before `desired_width`
            let mut num_chars = 0;

            let mut extension = lang.file_extensions.iter().peekable();
            while let Some(word) = extension.next() {
                // If we can't fit this word in, then create a line break and align it in.
                let new_chars = word.len() + comma_separator.len();
                if num_chars + new_chars >= desired_width {
                    num_chars = 0;
                    write!(stdout, "\n{:width$}{}", "", separator, width = longest)?;
                }

                num_chars += new_chars;
                write!(stdout, "{}", style.paint(&word[..]))?;
                if extension.peek().is_some() {
                    write!(stdout, "{}", comma_separator)?;
                }
            }
            writeln!(stdout)?;
        }
    }

    Ok(())
}
