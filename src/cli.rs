use std::process;
use std::str::FromStr;
use std::string::ToString;

use console::Term;
use structopt::StructOpt;

use crate::bat::assets::HighlightingAssets;
use crate::config;
use crate::style;

#[derive(StructOpt, Clone, Debug)]
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

    #[structopt(long = "theme", env = "BAT_THEME")]
    /// The syntax highlighting theme to use. Use --theme=none to disable syntax highlighting.
    /// If the theme is not set using this option, it will be taken from the BAT_THEME environment variable,
    /// if that contains a valid theme name. Use --list-themes and --compare-themes to view available themes.
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

    /// Show the command-line arguments (RGB hex codes) for the background colors that are in
    /// effect. The hex codes are displayed with their associated background color. This option can
    /// be combined with --light and --dark to view the background colors for those modes. It can
    /// also be used to experiment with different RGB hex codes by combining this option with
    /// --minus-color, --minus-emph-color, --plus-color, --plus-emph-color.
    #[structopt(long = "show-background-colors")]
    pub show_background_colors: bool,

    /// List supported languages and associated file extensions.
    #[structopt(long = "list-languages")]
    pub list_languages: bool,

    /// List available syntax-highlighting color themes.
    #[structopt(long = "list-theme-names")]
    pub list_theme_names: bool,

    /// List available syntax highlighting themes, each with an example of highlighted diff output.
    /// If diff output is supplied on standard input then this will be used for the demo. For
    /// example: `git show --color=always | delta --list-themes`.
    #[structopt(long = "list-themes")]
    pub list_themes: bool,

    /// The maximum distance between two lines for them to be inferred to be homologous. Homologous
    /// line pairs are highlighted according to the deletion and insertion operations transforming
    /// one into the other.
    #[structopt(long = "max-line-distance", default_value = "0.3")]
    pub max_line_distance: f64,
}

#[derive(Clone, Debug, PartialEq)]
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
        Some(theme) if !style::is_no_syntax_highlighting_theme_name(&theme) => {
            if !assets.theme_set.themes.contains_key(theme.as_str()) {
                eprintln!("Invalid theme: '{}'", theme);
                process::exit(1);
            }
            let is_light_theme = style::is_light_theme(&theme);
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

    // We do not use the full width, in case `less --status-column` is in effect. See #41 and #10.

    // TODO: There seems to be some confusion in the accounting: we are actually leaving 2
    // characters unused for less at the right edge of the terminal, despite the subtraction of 1
    // here.
    let available_terminal_width = (Term::stdout().size().1 - 1) as usize;
    let background_color_width = match opt.width.as_ref().map(String::as_str) {
        Some("variable") => None,
        Some(width) => Some(
            width
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("Invalid width: {}", width)),
        ),
        None => Some(available_terminal_width),
    };

    config::get_config(
        opt,
        &assets.syntax_set,
        &assets.theme_set,
        available_terminal_width,
        background_color_width,
    )
}
