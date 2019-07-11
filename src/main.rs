#[macro_use]
extern crate error_chain;

mod bat;
mod paint;
mod parse;

use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::process;

use console::Term;
use structopt::StructOpt;

use crate::bat::assets::{list_languages, HighlightingAssets};
use crate::bat::output::{OutputType, PagingMode};
use crate::parse::delta;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "delta", about = "A syntax-highlighter for git.")]
struct Opt {
    /// Use colors appropriate for a light terminal background. For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "light")]
    light: bool,

    /// Use colors appropriate for a dark terminal background.  For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "dark")]
    dark: bool,

    #[structopt(long = "minus-color")]
    /// The background color (RGB hex) to use for removed lines.
    minus_color: Option<String>,

    #[structopt(long = "minus-emph-color")]
    /// The background color (RGB hex) to use for emphasized sections of removed lines.
    minus_emph_color: Option<String>,

    #[structopt(long = "plus-color")]
    /// The background color (RGB hex) to use for added lines.
    plus_color: Option<String>,

    #[structopt(long = "plus-emph-color")]
    /// The background color (RGB hex) to use for emphasized sections of added lines.
    plus_emph_color: Option<String>,

    #[structopt(long = "theme")]
    /// The syntax highlighting theme to use.
    theme: Option<String>,

    #[structopt(long = "highlight-removed")]
    /// Apply syntax highlighting to removed lines. The default is to
    /// apply syntax highlighting to unchanged and new lines only.
    highlight_removed: bool,

    #[structopt(long = "no-structural-changes")]
    /// Do not modify input text; only add colors. This disables
    /// prettification of metadata sections in the git diff output.
    no_structural_changes: bool,

    /// The width (in characters) of the background color
    /// highlighting. By default, the width is the current terminal
    /// width. Use --width=variable to apply background colors to the
    /// end of each line, without right padding to equal width.
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

    let mut output_type =
        OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(paint_config.pager)).unwrap();
    let mut writer = output_type.handle().unwrap();

    match delta(
        io::stdin().lock().lines().map(|l| l.unwrap()),
        &paint_config,
        &assets,
        &mut writer,
    ) {
        Err(error) => match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => eprintln!("{}", error),
        },
        _ => (),
    };
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

    let terminal_width = Term::stdout().size().1 as usize;
    let width = match opt.width.as_ref().map(String::as_str) {
        Some("variable") => None,
        Some(width) => Some(
            width
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("Invalid width: {}", width)),
        ),
        None => Some(terminal_width - 1),
    };

    paint::get_config(
        &assets.syntax_set,
        &opt.theme,
        &assets.theme_set,
        opt.light,
        &opt.minus_color,
        &opt.minus_emph_color,
        &opt.plus_color,
        &opt.plus_emph_color,
        opt.highlight_removed,
        opt.no_structural_changes,
        terminal_width,
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
        if opt.light && !paint::is_light_theme(theme) || opt.dark && paint::is_light_theme(theme) {
            continue;
        }

        writeln!(stdout, "{}\n{}\n{}\n", hline, theme, hline)?;
        opt.theme = Some(theme.to_string());
        paint_config = process_command_line_arguments(&assets, &opt);
        let mut output_type =
            OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(paint_config.pager)).unwrap();
        let mut writer = output_type.handle().unwrap();

        delta(
            input.split("\n").map(String::from),
            &paint_config,
            &assets,
            &mut writer,
        )?;
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
        if paint::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    writeln!(stdout, "Dark themes:")?;
    for (theme, _) in themes.iter() {
        if !paint::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use console::strip_ansi_codes;

    #[test]
    #[ignore]
    fn test_added_file() {
        let input = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

diff --git a/a.py b/a.py
new file mode 100644
index 0000000..8c55b7d
--- /dev/null
+++ b/a.py
@@ -0,0 +1,3 @@
+# hello
+class X:
+    pass";

        let expected_output = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
modified: a.py
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
────────────────────────────────────────────────────────────────────────────────

────────────────────────────────────────────────────────────────────────────────
 # hello
 class X:
     pass
";

        let mut opt = Opt::from_args();
        opt.width = Some("variable".to_string());
        let assets = HighlightingAssets::new();
        let paint_config = process_command_line_arguments(&assets, &opt);
        let mut writer: Vec<u8> = Vec::new();
        delta(
            input.split("\n").map(String::from),
            &paint_config,
            &assets,
            &mut writer,
        )
        .unwrap();
        let output = strip_ansi_codes(&String::from_utf8(writer).unwrap()).to_string();
        assert!(output.contains("\nadded: a.py\n"));
        if false {
            // TODO: hline width
            assert_eq!(output, expected_output);
        }
    }

}
