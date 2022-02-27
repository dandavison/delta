// Based on code from https://github.com/sharkdp/bat a1b9334a44a2c652f52dddaa83dbacba57372468
// See src/utils/bat/LICENSE

use std::io::{self, Write};

use ansi_term::Colour::Green;
use ansi_term::Style;
use bat;

use crate::utils;

pub fn load_highlighting_assets() -> bat::assets::HighlightingAssets {
    bat::assets::HighlightingAssets::from_cache(utils::bat::dirs::PROJECT_DIRS.cache_dir())
        .unwrap_or_else(|_| bat::assets::HighlightingAssets::from_binary())
}

pub fn list_languages() -> std::io::Result<()> {
    let assets = utils::bat::assets::load_highlighting_assets();
    let mut languages = assets
        .get_syntaxes()
        .unwrap()
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
