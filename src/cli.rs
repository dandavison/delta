use std::collections::HashSet;
#[cfg(test)]
use std::ffi::OsString;
use std::path::PathBuf;

use itertools;
use lazy_static::lazy_static;
use structopt::clap::AppSettings::{ColorAlways, ColoredHelp, DeriveDisplayOrder};
use structopt::{clap, StructOpt};

use crate::git_config::GitConfig;
use crate::rewrite_options;
use crate::set_options;

#[derive(StructOpt, Clone, Debug, PartialEq)]
#[structopt(
    name = "delta",
    about = "A viewer for git and diff output",
    setting(ColorAlways),
    setting(ColoredHelp),
    setting(DeriveDisplayOrder),
    after_help = "\
GIT CONFIG
----------

By default, delta takes settings from a section named \"delta\" in git config files, if one is
present. The git config file to use for delta options will usually be ~/.gitconfig, but delta
follows the rules given in https://git-scm.com/docs/git-config#FILES. Most delta options can be
given in a git config file, using the usual option names but without the initial '--'. An example
is

[delta]
    number = true
    zero-style = dim syntax

FEATURES
--------
A feature is a named collection of delta options in git config. An example is:

[delta \"my-delta-feature\"]
    syntax-theme = Dracula
    plus-style = bold syntax \"#002800\"

To activate those options, you would use:

delta --features my-delta-feature

A feature name may not contain whitespace. You can activate multiple features:

[delta]
    features = my-highlight-styles-colors-feature my-line-number-styles-feature

If more than one feature sets the same option, the last one wins.

STYLES
------

All options that have a name like --*-style work the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
https://git-scm.com/docs/git-config#Documentation/git-config.txt-color

Here is an example:

--minus-style 'red bold ul #ffeeee'

That means: For removed lines, set the foreground (text) color to 'red', make it bold and
            underlined, and set the background color to '#ffeeee'.

See the COLORS section below for how to specify a color. In addition to real colors, there are 4
special color names: 'auto', 'normal', 'raw', and 'syntax'.

Here is an example of using special color names together with a single attribute:

--minus-style 'syntax bold auto'

That means: For removed lines, syntax-highlight the text, and make it bold, and do whatever delta
            normally does for the background.

The available attributes are: 'blink', 'bold', 'dim', 'hidden', 'italic', 'reverse', 'strike',
and 'ul' (or 'underline').

A complete description of the style string syntax follows:

- If the input that delta is receiving already has colors, and you want delta to output those
  colors unchanged, then use the special style string 'raw'. Otherwise, delta will strip any colors
  from its input.

- A style string consists of 0, 1, or 2 colors, together with an arbitrary number of style
  attributes, all separated by spaces.

- The first color is the foreground (text) color. The second color is the background color.
  Attributes can go in any position.

- This means that in order to specify a background color you must also specify a foreground (text)
  color.

- If you want delta to choose one of the colors automatically, then use the special color 'auto'.
  This can be used for both foreground and background.

- If you want the foreground/background color to be your terminal's foreground/background color,
  then use the special color 'normal'.

- If you want the foreground text to be syntax-highlighted according to its language, then use the
  special foreground color 'syntax'. This can only be used for the foreground (text).

- The minimal style specification is the empty string ''. This means: do not apply any colors or
  styling to the element in question.

COLORS
------

There are three ways to specify a color (this section applies to foreground and background colors
within a style string):

1. RGB hex code

   An example of using an RGB hex code is:
   --file-style=\"#0e7c0e\"

2. ANSI color name

   There are 8 ANSI color names:
   black, red, green, yellow, blue, magenta, cyan, white.

   In addition, all of them have a bright form:
   brightblack, brightred, brightgreen, brightyellow, brightblue, brightmagenta, brightcyan, brightwhite.

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


LINE NUMBERS
------------

To display line numbers, use --line-numbers.

Line numbers are displayed in two columns. Here's what it looks like by default:

 1  ⋮ 1  │ unchanged line
 2  ⋮    │ removed line
    ⋮ 2  │ added line

In that output, the line numbers for the old (minus) version of the file appear in the left column,
and the line numbers for the new (plus) version of the file appear in the right column. In an
unchanged (zero) line, both columns contain a line number.

The following options allow the line number display to be customized:

--line-numbers-left-format:  Change the contents of the left column
--line-numbers-right-format: Change the contents of the right column
--line-numbers-left-style:   Change the style applied to the left column
--line-numbers-right-style:  Change the style applied to the right column
--line-numbers-minus-style:  Change the style applied to line numbers in minus lines
--line-numbers-zero-style:   Change the style applied to line numbers in unchanged lines
--line-numbers-plus-style:   Change the style applied to line numbers in plus lines

Options --line-numbers-left-format and --line-numbers-right-format allow you to change the contents
of the line number columns. Their values are arbitrary format strings, which are allowed to contain
the placeholders {nm} for the line number associated with the old version of the file and {np} for
the line number associated with the new version of the file. The placeholders support a subset of
the string formatting syntax documented here: https://doc.rust-lang.org/std/fmt/#formatting-parameters.
Specifically, you can use the alignment, width, and fill syntax.

For example, the default value of --line-numbers-left-format is '{nm:^4}⋮'. This means that the
left column should display the minus line number (nm), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (⋮).

Similarly, the default value of --line-numbers-right-format is '{np:^4}│ '. This means that the
right column should display the plus line number (np), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (│), and a space.

Use '<' for left-align, '^' for center-align, and '>' for right-align.


If something isn't working correctly, or you have a feature request, please open an issue at
https://github.com/dandavison/delta/issues.
"
)]
pub struct Opt {
    #[structopt(long = "features", default_value = "")]
    /// Name of delta features to use (space-separated). A feature is a named collection of delta
    /// options in ~/.gitconfig. See FEATURES section.
    pub features: String,

    #[structopt(long = "syntax-theme", env = "BAT_THEME")]
    /// The code syntax-highlighting theme to use. Use --show-syntax-themes to demo available
    /// themes. If the syntax-highlighting theme is not set using this option, it will be taken
    /// from the BAT_THEME environment variable, if that contains a valid theme name.
    /// --syntax-theme=none disables all syntax highlighting.
    pub syntax_theme: Option<String>,

    /// Use default colors appropriate for a light terminal background. For more control, see the
    /// style options.
    #[structopt(long = "light")]
    pub light: bool,

    /// Use default colors appropriate for a dark terminal background. For more control, see the
    /// style options.
    #[structopt(long = "dark")]
    pub dark: bool,

    #[structopt(long = "diff-highlight")]
    /// Emulate diff-highlight (https://github.com/git/git/tree/master/contrib/diff-highlight)
    pub diff_highlight: bool,

    #[structopt(long = "diff-so-fancy")]
    /// Emulate diff-so-fancy (https://github.com/so-fancy/diff-so-fancy)
    pub diff_so_fancy: bool,

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

    #[structopt(long = "minus-non-emph-style", default_value = "auto auto")]
    /// Style (foreground, background, attributes) for non-emphasized sections of removed lines
    /// that have an emphasized section. Defaults to --minus-style. See STYLES section.
    pub minus_non_emph_style: String,

    #[structopt(long = "plus-emph-style", default_value = "syntax auto")]
    /// Style (foreground, background, attributes) for emphasized sections of added lines. See
    /// STYLES section.
    pub plus_emph_style: String,

    #[structopt(long = "plus-non-emph-style", default_value = "auto auto")]
    /// Style (foreground, background, attributes) for non-emphasized sections of added lines that
    /// have an emphasized section. Defaults to --plus-style. See STYLES section.
    pub plus_non_emph_style: String,

    #[structopt(long = "commit-style", default_value = "raw")]
    /// Style (foreground, background, attributes) for the commit hash line. See STYLES section.
    pub commit_style: String,

    #[structopt(long = "commit-decoration-style", default_value = "")]
    /// Style (foreground, background, attributes) for the commit hash decoration. See STYLES
    /// section. One of the special attributes 'box', 'ul', 'overline', or 'underoverline' must be
    /// given.
    pub commit_decoration_style: String,

    #[structopt(long = "file-style", default_value = "blue")]
    /// Style (foreground, background, attributes) for the file section. See STYLES section.
    pub file_style: String,

    #[structopt(long = "file-decoration-style", default_value = "blue ul")]
    /// Style (foreground, background, attributes) for the file decoration. See STYLES section. One
    /// of the special attributes 'box', 'ul', 'overline', or 'underoverline' must be given.
    pub file_decoration_style: String,

    #[structopt(long = "navigate")]
    /// Activate diff navigation: use n to jump forwards and N to jump backwards. To change the
    /// file labels used see --file-modified-label, --file-removed-label, --file-added-label,
    /// --file-renamed-label.
    pub navigate: bool,

    #[structopt(long = "file-modified-label", default_value = "")]
    /// Text to display in front of a modified file path.
    pub file_modified_label: String,

    #[structopt(long = "file-removed-label", default_value = "removed:")]
    /// Text to display in front of a removed file path.
    pub file_removed_label: String,

    #[structopt(long = "file-added-label", default_value = "added:")]
    /// Text to display in front of a added file path.
    pub file_added_label: String,

    #[structopt(long = "file-renamed-label", default_value = "renamed:")]
    /// Text to display in front of a renamed file path.
    pub file_renamed_label: String,

    #[structopt(long = "hunk-header-style", default_value = "syntax")]
    /// Style (foreground, background, attributes) for the hunk-header. See STYLES section.
    pub hunk_header_style: String,

    #[structopt(long = "hunk-header-decoration-style", default_value = "blue box")]
    /// Style (foreground, background, attributes) for the hunk-header decoration. See STYLES
    /// section. One of the special attributes 'box', 'ul', 'overline', or 'underoverline' must be
    /// given.
    pub hunk_header_decoration_style: String,

    /// Display line numbers next to the diff. See LINE NUMBERS section.
    #[structopt(short = "n", long = "line-numbers")]
    pub line_numbers: bool,

    /// Style (foreground, background, attributes) for line numbers in the old (minus) version of
    /// the file. See STYLES and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-minus-style", default_value = "auto")]
    pub line_numbers_minus_style: String,

    /// Style (foreground, background, attributes) for line numbers in unchanged (zero) lines. See
    /// STYLES and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-zero-style", default_value = "auto")]
    pub line_numbers_zero_style: String,

    /// Style (foreground, background, attributes) for line numbers in the new (plus) version of
    /// the file. See STYLES and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-plus-style", default_value = "auto")]
    pub line_numbers_plus_style: String,

    /// Format string for the left column of line numbers. A typical value would be "{nm:^4}⋮"
    /// which means to display the line numbers of the minus file (old version), followed by a
    /// dividing character. See the LINE NUMBERS section.
    #[structopt(long = "line-numbers-left-format", default_value = "{nm:^4}⋮")]
    pub line_numbers_left_format: String,

    /// Format string for the right column of line numbers. A typical value would be "{np:^4}│ "
    /// which means to display the line numbers of the plus file (new version), followed by a
    /// dividing character, and a space. See the LINE NUMBERS section.
    #[structopt(long = "line-numbers-right-format", default_value = "{np:^4}│ ")]
    pub line_numbers_right_format: String,

    /// Style (foreground, background, attributes) for the left column of line numbers. See STYLES
    /// and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-left-style", default_value = "auto")]
    pub line_numbers_left_style: String,

    /// Style (foreground, background, attributes) for the right column of line numbers. See STYLES
    /// and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-right-style", default_value = "auto")]
    pub line_numbers_right_style: String,

    #[structopt(long = "raw")]
    /// Do not alter the input in any way. The only exceptions are the coloring of hunk lines:
    /// minus lines use color.diff.old (with fallback to "red") and plus lines use color.diff.new
    /// (with fallback to "green").
    pub raw: bool,

    #[structopt(long = "color-only")]
    /// Do not alter the input in any way except for coloring hunk lines.
    pub color_only: bool,

    #[structopt(long = "no-gitconfig")]
    /// Do not take settings from git config files. See GIT CONFIG section.
    pub no_gitconfig: bool,

    #[structopt(long = "keep-plus-minus-markers")]
    /// Prefix added/removed lines with a +/- character, respectively, exactly as git does. The
    /// default behavior is to output a space character in place of these markers.
    pub keep_plus_minus_markers: bool,

    /// The width of underline/overline decorations. Use --width=variable to extend decorations and
    /// background colors to the end of the text only. Otherwise background colors extend to the
    /// full terminal width.
    #[structopt(short = "w", long = "width")]
    pub width: Option<String>,

    /// The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters
    /// through directly, but note that in that case delta will calculate line widths assuming tabs
    /// occupy one character's width on the screen: if your terminal renders tabs as more than than
    /// one character wide then delta's output will look incorrect.
    #[structopt(long = "tabs", default_value = "4")]
    pub tab_width: usize,

    /// Display the active values for all Delta options. Style options are displayed with
    /// foreground and background colors. This can be used to experiment with colors by combining
    /// this option with other options such as --minus-style, --zero-style, --plus-style, --light,
    /// --dark, etc.
    #[structopt(long = "show-config")]
    pub show_config: bool,

    /// List supported languages and associated file extensions.
    #[structopt(long = "list-languages")]
    pub list_languages: bool,

    /// List available syntax-highlighting color themes.
    #[structopt(long = "list-syntax-themes")]
    pub list_syntax_themes: bool,

    /// Show all available syntax-highlighting themes, each with an example of highlighted diff output.
    /// If diff output is supplied on standard input then this will be used for the demo. For
    /// example: `git show --color=always | delta --show-syntax-themes`.
    #[structopt(long = "show-syntax-themes")]
    pub show_syntax_themes: bool,

    /// The regular expression used to decide what a word is for the within-line highlight
    /// algorithm. For less fine-grained matching than the default try --word-diff-regex="\S+"
    /// --max-line-distance=1.0 (this is more similar to `git --word-diff`).
    #[structopt(long = "word-diff-regex", default_value = r"\w+")]
    pub tokenization_regex: String,

    /// The maximum distance between two lines for them to be inferred to be homologous. Homologous
    /// line pairs are highlighted according to the deletion and insertion operations transforming
    /// one into the other.
    #[structopt(long = "max-line-distance", default_value = "0.6")]
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

    /// First file to be compared when delta is being used in diff mode.
    #[structopt(parse(from_os_str))]
    pub minus_file: Option<PathBuf>,

    /// Second file to be compared when delta is being used in diff mode.
    #[structopt(parse(from_os_str))]
    pub plus_file: Option<PathBuf>,

    /// Style for removed empty line marker (used only if --minus-style has no background color)
    #[structopt(
        long = "--minus-empty-line-marker-style",
        default_value = "normal auto"
    )]
    pub minus_empty_line_marker_style: String,

    /// Style for added empty line marker (used only if --plus-style has no background color)
    #[structopt(long = "--plus-empty-line-marker-style", default_value = "normal auto")]
    pub plus_empty_line_marker_style: String,

    /// Style for whitespace errors. Defaults to color.diff.whitespace if that is set in git
    /// config, or else 'magenta reverse'.
    #[structopt(long = "whitespace-error-style", default_value = "auto auto")]
    pub whitespace_error_style: String,

    #[structopt(long = "minus-color")]
    /// Deprecated: use --minus-style='normal my_background_color'.
    pub deprecated_minus_background_color: Option<String>,

    #[structopt(long = "minus-emph-color")]
    /// Deprecated: use --minus-emph-style='normal my_background_color'.
    pub deprecated_minus_emph_background_color: Option<String>,

    #[structopt(long = "plus-color")]
    /// Deprecated: Use --plus-style='syntax my_background_color' to change the background color
    /// while retaining syntax-highlighting.
    pub deprecated_plus_background_color: Option<String>,

    #[structopt(long = "plus-emph-color")]
    /// Deprecated: Use --plus-emph-style='syntax my_background_color' to change the background
    /// color while retaining syntax-highlighting.
    pub deprecated_plus_emph_background_color: Option<String>,

    #[structopt(long = "highlight-removed")]
    /// Deprecated: use --minus-style='syntax'.
    pub deprecated_highlight_minus_lines: bool,

    #[structopt(long = "commit-color")]
    /// Deprecated: use --commit-style='my_foreground_color'
    /// --commit-decoration-style='my_foreground_color'.
    pub deprecated_commit_color: Option<String>,

    #[structopt(long = "file-color")]
    /// Deprecated: use --file-style='my_foreground_color'
    /// --file-decoration-style='my_foreground_color'.
    pub deprecated_file_color: Option<String>,

    #[structopt(long = "hunk-style")]
    /// Deprecated: synonym of --hunk-header-decoration-style.
    pub deprecated_hunk_style: Option<String>,

    #[structopt(long = "hunk-color")]
    /// Deprecated: use --hunk-header-style='my_foreground_color'
    /// --hunk-header-decoration-style='my_foreground_color'.
    pub deprecated_hunk_color: Option<String>,

    #[structopt(long = "theme")]
    /// Deprecated: use --syntax-theme.
    pub deprecated_theme: Option<String>,
}

impl Opt {
    pub fn from_args_and_git_config(git_config: &mut Option<GitConfig>) -> Self {
        Self::from_clap_and_git_config(Self::clap().get_matches(), git_config)
    }

    #[cfg(test)]
    pub fn from_iter_and_git_config<I>(iter: I, git_config: &mut Option<GitConfig>) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        Self::from_clap_and_git_config(Self::clap().get_matches_from(iter), git_config)
    }

    fn from_clap_and_git_config(
        arg_matches: clap::ArgMatches,
        git_config: &mut Option<GitConfig>,
    ) -> Self {
        let mut opt = Opt::from_clap(&arg_matches);
        set_options::set_options(&mut opt, git_config, &arg_matches);
        rewrite_options::apply_rewrite_rules(&mut opt, &arg_matches);
        opt
    }

    pub fn get_option_or_flag_names<'a>() -> HashSet<&'a str> {
        let names: HashSet<&str> = itertools::chain(
            Self::clap().p.opts.iter().filter_map(|opt| opt.s.long),
            Self::clap().p.flags.iter().filter_map(|opt| opt.s.long),
        )
        .collect();
        &names - &*IGNORED_OPTION_OR_FLAG_NAMES
    }
}

// Option names to exclude when listing options to process for various purposes. These are
// (1) Deprecated options
// (2) Pseudo-flag commands such as --list-languages
lazy_static! {
    static ref IGNORED_OPTION_OR_FLAG_NAMES: HashSet<&'static str> = vec![
        "commit-color",
        "file-color",
        "highlight-removed",
        "hunk-color",
        "hunk-style",
        "list-languages",
        "list-syntax-themes",
        "minus-color",
        "minus-emph-color",
        "plus-color",
        "plus-emph-color",
        "show-config",
        "show-syntax-themes",
        "theme",
    ]
    .into_iter()
    .collect();
}
