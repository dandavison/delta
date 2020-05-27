use std::process;

use console::Term;
use structopt::clap::AppSettings::{ColorAlways, ColoredHelp, DeriveDisplayOrder};
use structopt::StructOpt;

use crate::bat::assets::HighlightingAssets;
use crate::bat::output::PagingMode;
use crate::config;
use crate::env;
use crate::rewrite;
use crate::style;

#[derive(StructOpt, Clone, Debug, PartialEq)]
#[structopt(
    name = "delta",
    about = "A syntax-highlighter for git and diff output",
    setting(ColorAlways),
    setting(ColoredHelp),
    setting(DeriveDisplayOrder),
    after_help = "\
STYLES
------

All options that have a name like --*-style work the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
https://git-scm.com/docs/git-config#Documentation/git-config.txt-color

Here is an example:

--minus-style 'red bold underline #ffeeee'

That means: For removed lines, set the foreground (text) color to 'red', make it bold and
            underlined, and set the background color to '#ffeeee'.

See the COLORS section below for how to specify a color. In addition to real colors, there are 3
special color names: 'auto', 'normal', 'syntax'.

Here is an example of using special color names together with a single attribute:

--minus-style 'syntax bold auto'

That means: For removed lines, syntax-highlight the text, and make it bold, and do whatever delta
            normally does for the background.

The available attributes are: 'blink', 'bold', 'dimmed', 'hidden', 'italic', 'reverse',
'strikethrough', 'underline'.

A complete description of the style string syntax follows:

- A style string consists of 0, 1, or 2 colors, together with an arbitrary number of style
  attributes, all separated by spaces.

- The first color is the foreground (text) color. The second color is the background color.
  Attributes can go in any position.

- This means that in order to specify a background color you must also specify a foreground (text)
  color.

- If you just want delta to choose one of the colors automatically, then use the special color
  'auto'. This can be used for both foreground and background.

- If you want the foreground text to be syntax-highlighted according to its language, then use the
  special foreground color 'syntax'. This can only be used for the foreground (text).

- If you want delta to not apply any color, then use the special color 'normal'. This can be used
  for both foreground and background.

- The minimal style specification is the empty string ''. This means: do not apply any colors or
  styling to the element in question.

COLORS
------

There are three ways to specify a color (this section applies to both foreground and background
colors within a style string):

1. RGB hex code

   An example of using an RGB hex code is:
   --file-style=\"#0e7c0e\"

2. ANSI color name

   There are 8 ANSI color names:
   black, red, green, yellow, blue, magenta, cyan, white.

   In addition, all of them have a bright form:
   bright-black, bright-red, bright-green, bright-yellow, bright-blue, bright-magenta, bright-cyan, bright-white

   An example of using an ANSI color name is:
   --file-style=\"green\"

   Unlike RGB hex codes, ANSI color names are just names: you can choose the exact color that each
   name corresponds to in the settings of your terminal application (the application you use to
   enter commands at a shell prompt). This means that if you use ANSI color names, and you change
   the color theme used by your terminal, then delta's colors will respond automatically, without
   needing to change the delta command line.

   \"purple\" is accepted as a synonym for \"magenta\". Color names and codes are case-insensitive.

3. ANSI color number

   An example of using an ANSI color number is:
   --file-style=28

   There are 256 ANSI color numbers: 0-255. The first 16 are the same as the colors described in
   the \"ANSI color name\" section above. See https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit.
   Specifying colors like this is useful if your terminal only supports 256 colors (i.e. doesn\'t
   support 24-bit color).
"
)]
pub struct Opt {
    #[structopt(long = "theme", env = "BAT_THEME")]
    /// The code syntax highlighting theme to use. Use --list-themes to demo available themes. If
    /// the theme is not set using this option, it will be taken from the BAT_THEME environment
    /// variable, if that contains a valid theme name. --theme=none disables all syntax
    /// highlighting.
    pub theme: Option<String>,

    /// Use default colors appropriate for a light terminal background. For more control, see the
    /// style options.
    #[structopt(long = "light")]
    pub light: bool,

    /// Use default colors appropriate for a dark terminal background. For more control, see the
    /// style options.
    #[structopt(long = "dark")]
    pub dark: bool,

    #[structopt(long = "minus-style", default_value = "normal auto")]
    /// Style (foreground, background, attributes) for removed lines. See STYLES section.
    pub minus_style: String,

    #[structopt(long = "zero-style", default_value = "syntax normal")]
    /// Style (foreground, background, attributes) for unchanged lines. See STYLES section.
    pub zero_style: String,

    #[structopt(long = "plus-style", default_value = "syntax auto")]
    /// Style (foreground, background, attributes) for added lines. See STYLES section.
    pub plus_style: String,

    #[structopt(long = "minus-emph-style", default_value = "normal auto")]
    /// Style (foreground, background, attributes) for emphasized sections of removed lines. See
    /// STYLES section.
    pub minus_emph_style: String,

    #[structopt(long = "minus-non-emph-style")]
    /// Style (foreground, background, attributes) for non-emphasized sections of removed lines
    /// that have an emphasized section. Defaults to --minus-style. See STYLES section.
    pub minus_non_emph_style: Option<String>,

    #[structopt(long = "plus-emph-style", default_value = "syntax auto")]
    /// Style (foreground, background, attributes) for emphasized sections of added lines. See
    /// STYLES section.
    pub plus_emph_style: String,

    #[structopt(long = "plus-non-emph-style")]
    /// Style (foreground, background, attributes) for non-emphasized sections of added lines that
    /// have an emphasized section. Defaults to --plus-style. See STYLES section.
    pub plus_non_emph_style: Option<String>,

    #[structopt(long = "commit-style", default_value = "yellow")]
    /// Style (foreground, background, attributes) for the commit hash line. See STYLES section.
    pub commit_style: String,

    #[structopt(long = "commit-decoration-style", default_value = "")]
    /// Style for the commit hash decoration. See STYLES section. Special attributes are 'box', and
    /// 'underline' are available in addition to the usual style attributes.
    pub commit_decoration_style: String,

    #[structopt(long = "file-style", default_value = "blue")]
    /// Style (foreground, background, attributes) for the file section. See STYLES section.
    pub file_style: String,

    #[structopt(long = "file-decoration-style", default_value = "blue underline")]
    /// Style for the file decoration. See STYLES section. Special attributes are 'box', and
    /// 'underline' are available in addition to the usual style attributes.
    pub file_decoration_style: String,

    #[structopt(long = "hunk-header-style", default_value = "syntax")]
    /// Style (foreground, background, attributes) for the hunk-header. See STYLES section.
    pub hunk_header_style: String,

    #[structopt(long = "hunk-header-decoration-style", default_value = "blue box")]
    /// Style (foreground, background, attributes) for the hunk-header decoration. See STYLES
    /// section. Special attributes are 'box', and 'underline' are available in addition to the
    /// usual style attributes.
    pub hunk_header_decoration_style: String,

    #[structopt(long = "color-only")]
    /// Do not alter the input in any way other than applying colors. Equivalent to
    /// `--keep-plus-minus-markers --width variable --tabs 0 --commit-decoration ''
    /// --file-decoration '' --hunk-decoration ''`.
    pub color_only: bool,

    #[structopt(long = "keep-plus-minus-markers")]
    /// Prefix added/removed lines with a +/- character, respectively, exactly as git does. The
    /// default behavior is to output a space character in place of these markers.
    pub keep_plus_minus_markers: bool,

    /// Use --width=variable to extend background colors to the end of each line only. Otherwise
    /// background colors extend to the full terminal width.
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
    /// also be used to experiment with different RGB hex codes by combining this option with style
    /// options such as --minus-style, --zero-style, --plus-style, etc.
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

    /// Whether to emit 24-bit ("true color") RGB color codes. Options are auto, always, and never.
    /// "auto" means that delta will emit 24-bit color codes iff the environment variable COLORTERM
    /// has the value "truecolor" or "24bit". If your terminal application (the application you use
    /// to enter commands at a shell prompt) supports 24 bit colors, then it probably already sets
    /// this environment variable, in which case you don't need to do anything.
    #[structopt(long = "24-bit-color", default_value = "auto")]
    pub true_color: String,

    /// Whether to use a pager when displaying output. Options are: auto, always, and never. The
    /// default pager is `less`: this can be altered by setting the environment variables BAT_PAGER
    /// or PAGER (BAT_PAGER has priority).
    #[structopt(long = "paging", default_value = "auto")]
    pub paging_mode: String,

    #[structopt(long = "minus-color")]
    /// Deprecated: use --minus-style='normal my_background_color'.
    pub deprecated_minus_background_color: Option<String>,

    #[structopt(long = "minus-emph-color")]
    /// Deprecated: use --minus-emph-style='normal my_background_color'.
    pub deprecated_minus_emph_background_color: Option<String>,

    #[structopt(long = "plus-color")]
    /// Deprecated: Use --plus-style='normal my_background_color'.
    pub deprecated_plus_background_color: Option<String>,

    #[structopt(long = "plus-emph-color")]
    /// Deprecated: Use --plus-emph-style='normal my_background_color'.
    pub deprecated_plus_emph_background_color: Option<String>,

    #[structopt(long = "highlight-removed")]
    /// Deprecated: use --minus-style='syntax'.
    pub deprecated_highlight_minus_lines: bool,

    #[structopt(long = "commit-color")]
    /// Deprecated: use --commit-style='my_foreground_color' --commit-decoration-style='my_foreground_color'.
    pub deprecated_commit_color: Option<String>,

    #[structopt(long = "file-color")]
    /// Deprecated: use --file-style='my_foreground_color' --file-decoration-style='my_foreground_color'.
    pub deprecated_file_color: Option<String>,

    #[structopt(long = "hunk-style")]
    /// Deprecated: synonym of --hunk-header-decoration-style.
    pub deprecated_hunk_style: Option<String>,

    #[structopt(long = "hunk-color")]
    /// Deprecated: use --hunk-header-style='my_foreground_color' --hunk-header-decoration-style='my_foreground_color'.
    pub deprecated_hunk_color: Option<String>,
}

pub fn process_command_line_arguments<'a>(mut opt: Opt) -> config::Config<'a> {
    let assets = HighlightingAssets::new();

    _check_validity(&opt, &assets);

    rewrite::apply_rewrite_rules(&mut opt);

    // We do not use the full width, in case `less --status-column` is in effect. See #41 and #10.
    // TODO: There seems to be some confusion in the accounting: we are actually leaving 2
    // characters unused for less at the right edge of the terminal, despite the subtraction of 1
    // here.
    let available_terminal_width = (Term::stdout().size().1 - 1) as usize;

    let paging_mode = match opt.paging_mode.as_ref() {
        "always" => PagingMode::Always,
        "never" => PagingMode::Never,
        "auto" => PagingMode::QuitIfOneScreen,
        _ => {
            eprintln!(
                "Invalid value for --paging option: {} (valid values are \"always\", \"never\", and \"auto\")",
                opt.paging_mode
            );
            process::exit(1);
        }
    };

    let true_color = match opt.true_color.as_ref() {
        "always" => true,
        "never" => false,
        "auto" => is_truecolor_terminal(),
        _ => {
            eprintln!(
                "Invalid value for --24-bit-color option: {} (valid values are \"always\", \"never\", and \"auto\")",
                opt.true_color
            );
            process::exit(1);
        }
    };

    config::get_config(
        opt,
        assets.syntax_set,
        assets.theme_set,
        true_color,
        available_terminal_width,
        paging_mode,
    )
}

fn _check_validity(opt: &Opt, assets: &HighlightingAssets) {
    if opt.light && opt.dark {
        eprintln!("--light and --dark cannot be used together.");
        process::exit(1);
    }
    if let Some(ref theme) = opt.theme {
        if !style::is_no_syntax_highlighting_theme_name(&theme) {
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
    }
}

pub fn unreachable(message: &str) -> ! {
    eprintln!(
        "{} This should not be possible. \
         Please report the bug at https://github.com/dandavison/delta/issues.",
        message
    );
    process::exit(1);
}

fn is_truecolor_terminal() -> bool {
    env::get_env_var("COLORTERM")
        .map(|colorterm| colorterm == "truecolor" || colorterm == "24bit")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::cli;
    use crate::style;
    use crate::tests::integration_test_utils::integration_test_utils;

    #[test]
    #[ignore]
    fn test_theme_selection() {
        #[derive(PartialEq)]
        enum Mode {
            Light,
            Dark,
        };
        for (
            theme_option,
            bat_theme_env_var,
            mode_option, // (--light, --dark)
            expected_theme,
            expected_mode,
        ) in vec![
            (None, "", None, style::DEFAULT_DARK_THEME, Mode::Dark),
            (Some("GitHub".to_string()), "", None, "GitHub", Mode::Light),
            (
                Some("GitHub".to_string()),
                "1337",
                None,
                "GitHub",
                Mode::Light,
            ),
            (None, "1337", None, "1337", Mode::Dark),
            (
                None,
                "<not set>",
                None,
                style::DEFAULT_DARK_THEME,
                Mode::Dark,
            ),
            (
                None,
                "",
                Some(Mode::Light),
                style::DEFAULT_LIGHT_THEME,
                Mode::Light,
            ),
            (
                None,
                "",
                Some(Mode::Dark),
                style::DEFAULT_DARK_THEME,
                Mode::Dark,
            ),
            (
                None,
                "<@@@@@>",
                Some(Mode::Light),
                style::DEFAULT_LIGHT_THEME,
                Mode::Light,
            ),
            (None, "1337", Some(Mode::Light), "1337", Mode::Light),
            (Some("none".to_string()), "", None, "none", Mode::Dark),
            (
                Some("None".to_string()),
                "",
                Some(Mode::Light),
                "None",
                Mode::Light,
            ),
        ] {
            if bat_theme_env_var == "<not set>" {
                env::remove_var("BAT_THEME")
            } else {
                env::set_var("BAT_THEME", bat_theme_env_var);
            }
            let is_true_color = true;
            let mut options = integration_test_utils::get_command_line_options();
            options.theme = theme_option;
            match mode_option {
                Some(Mode::Light) => {
                    options.light = true;
                    options.dark = false;
                }
                Some(Mode::Dark) => {
                    options.light = false;
                    options.dark = true;
                }
                None => {
                    options.light = false;
                    options.dark = false;
                }
            }
            let config = cli::process_command_line_arguments(options);
            assert_eq!(config.theme_name, expected_theme);
            if style::is_no_syntax_highlighting_theme_name(expected_theme) {
                assert!(config.theme.is_none())
            } else {
                assert_eq!(config.theme.unwrap().name.as_ref().unwrap(), expected_theme);
            }
            assert_eq!(
                config.minus_style.ansi_term_style.background.unwrap(),
                style::get_minus_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.minus_emph_style.ansi_term_style.background.unwrap(),
                style::get_minus_emph_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.plus_style.ansi_term_style.background.unwrap(),
                style::get_plus_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.plus_emph_style.ansi_term_style.background.unwrap(),
                style::get_plus_emph_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
        }
    }
}
