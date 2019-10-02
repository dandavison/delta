use std::process;
use std::str::FromStr;
use std::string::ToString;

use console::Term;
use structopt::StructOpt;

use crate::bat::assets::HighlightingAssets;
use crate::config;
use crate::style;

#[derive(StructOpt, Debug)]
#[structopt(name = "delta", about = "A syntax-highlighting pager for git")]
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
    /// The syntax highlighting theme to use. Use --theme=none to disable syntax highlighting.
    pub theme: Option<String>,

    #[structopt(long = "highlight-removed")]
    /// Apply syntax highlighting to removed lines. The default is to
    /// apply syntax highlighting to unchanged and new lines only.
    pub highlight_removed: bool,

    #[structopt(long = "commit-style", default_value = "plain")]
    /// Formatting style for commit section of git output. Options
    /// are: plain, box.
    pub commit_style: SectionStyle,

    #[structopt(long = "file-style", default_value = "underline")]
    /// Formatting style for file section of git output. Options
    /// are: plain, box, underline.
    pub file_style: SectionStyle,

    #[structopt(long = "hunk-style", default_value = "box")]
    /// Formatting style for hunk section of git output. Options
    /// are: plain, box.
    pub hunk_style: SectionStyle,

    /// The width (in characters) of the background color
    /// highlighting. By default, the width is the current terminal
    /// width. Use --width=variable to apply background colors to the
    /// end of each line, without right padding to equal width.
    #[structopt(short = "w", long = "width")]
    pub width: Option<String>,

    /// The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters
    /// through directly, but note that in that case delta will calculate line widths assuming tabs
    /// occupy one character's width on the screen: if your terminal renders tabs as more than than
    /// one character wide then delta's output will look incorrect.
    #[structopt(long = "tabs", default_value = "4")]
    pub tab_width: usize,

    /// Show the command-line arguments for the current colors.
    #[structopt(long = "show-colors")]
    pub show_colors: bool,

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

    /// The maximum distance between two lines for them to be inferred to be homologous. Homologous
    /// line pairs are highlighted according to the deletion and insertion operations transforming
    /// one into the other.
    #[structopt(long = "max-line-distance", default_value = "0.3")]
    pub max_line_distance: f64,
}

#[derive(Debug, PartialEq)]
pub enum SectionStyle {
    Box,
    Plain,
    Underline,
}

// TODO: clean up enum parsing and error handling

#[derive(Debug)]
pub enum Error {
    SectionStyleParseError,
}

impl FromStr for SectionStyle {
    type Err = Error;
    fn from_str(s: &str) -> Result<SectionStyle, Error> {
        match s.to_lowercase().as_str() {
            "box" => Ok(SectionStyle::Box),
            "plain" => Ok(SectionStyle::Plain),
            "underline" => Ok(SectionStyle::Underline),
            _ => Err(Error::SectionStyleParseError),
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        "".to_string()
    }
}

pub fn process_command_line_arguments<'a>(
    assets: &'a HighlightingAssets,
    opt: &'a Opt,
) -> config::Config<'a> {
    if opt.light && opt.dark {
        eprintln!("--light and --dark cannot be used together.");
        process::exit(1);
    }
    match &opt.theme {
        Some(theme) if theme.to_lowercase() != "none" => {
            if !assets.theme_set.themes.contains_key(theme.as_str()) {
                eprintln!("Invalid theme: '{}'", theme);
                process::exit(1);
            }
            let is_light_theme = style::LIGHT_THEMES.contains(&theme.as_str());
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
        _ => (),
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

    config::get_config(
        opt,
        &assets.syntax_set,
        &assets.theme_set,
        terminal_width,
        width,
    )
}
