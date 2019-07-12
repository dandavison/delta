use std::process;

use console::Term;
use structopt::StructOpt;

use crate::bat::assets::HighlightingAssets;
use crate::paint;

#[derive(StructOpt, Debug)]
#[structopt(name = "delta", about = "A syntax-highlighter for git.")]
pub struct Opt {
    /// Use colors appropriate for a light terminal background. For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "light")]
    pub light: bool,

    /// Use colors appropriate for a dark terminal background.  For
    /// more control, see --theme, --plus-color, and --minus-color.
    #[structopt(long = "dark")]
    pub dark: bool,

    #[structopt(long = "minus-color")]
    /// The background color (RGB hex) to use for removed lines.
    pub minus_color: Option<String>,

    #[structopt(long = "minus-emph-color")]
    /// The background color (RGB hex) to use for emphasized sections of removed lines.
    pub minus_emph_color: Option<String>,

    #[structopt(long = "plus-color")]
    /// The background color (RGB hex) to use for added lines.
    pub plus_color: Option<String>,

    #[structopt(long = "plus-emph-color")]
    /// The background color (RGB hex) to use for emphasized sections of added lines.
    pub plus_emph_color: Option<String>,

    #[structopt(long = "theme")]
    /// The syntax highlighting theme to use.
    pub theme: Option<String>,

    #[structopt(long = "highlight-removed")]
    /// Apply syntax highlighting to removed lines. The default is to
    /// apply syntax highlighting to unchanged and new lines only.
    pub highlight_removed: bool,

    #[structopt(long = "no-structural-changes")]
    /// Do not modify input text; only add colors. This disables
    /// prettification of metadata sections in the git diff output.
    pub no_structural_changes: bool,

    /// The width (in characters) of the background color
    /// highlighting. By default, the width is the current terminal
    /// width. Use --width=variable to apply background colors to the
    /// end of each line, without right padding to equal width.
    #[structopt(short = "w", long = "width")]
    pub width: Option<String>,

    /// List supported languages and associated file extensions.
    #[structopt(long = "list-languages")]
    pub list_languages: bool,

    /// List available syntax highlighting themes.
    #[structopt(long = "list-themes")]
    pub list_themes: bool,

    /// Compare available syntax highlighting themes. To use this
    /// option, supply git diff output to delta on standard input.
    /// For example: `git show --color=always | delta --compare-themes`.
    #[structopt(long = "compare-themes")]
    pub compare_themes: bool,
}

pub fn process_command_line_arguments<'a>(
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
