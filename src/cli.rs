use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;

use lazy_static::lazy_static;
use structopt::clap::AppSettings::{ColorAlways, ColoredHelp, DeriveDisplayOrder};
use structopt::{clap, StructOpt};
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;

use crate::bat_utils::assets::HighlightingAssets;
use crate::bat_utils::output::PagingMode;
use crate::git_config::{GitConfig, GitConfigEntry};
use crate::options;

#[derive(StructOpt, Clone, Default)]
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
    line-numbers = true
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

--minus-style 'red bold ul \"#ffeeee\"'

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

The attribute 'omit' is supported by commit-style, file-style, and hunk-header-style, meaning to
remove the element entirely from the output.

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
Specifically, you can use the alignment and width syntax.

For example, the default value of --line-numbers-left-format is '{nm:^4}⋮'. This means that the
left column should display the minus line number (nm), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (⋮).

Similarly, the default value of --line-numbers-right-format is '{np:^4}│'. This means that the
right column should display the plus line number (np), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (│).

Use '<' for left-align, '^' for center-align, and '>' for right-align.


If something isn't working correctly, or you have a feature request, please open an issue at
https://github.com/dandavison/delta/issues.
"
)]
pub struct Opt {
    /// Use default colors appropriate for a light terminal background. For more control, see the
    /// style options and --syntax-theme.
    #[structopt(long = "light")]
    pub light: bool,

    /// Use default colors appropriate for a dark terminal background. For more control, see the
    /// style options and --syntax-theme.
    #[structopt(long = "dark")]
    pub dark: bool,

    /// Display line numbers next to the diff. See LINE NUMBERS section.
    #[structopt(short = "n", long = "line-numbers")]
    pub line_numbers: bool,

    /// Display a side-by-side diff view instead of the traditional view.
    #[structopt(short = "s", long = "side-by-side")]
    pub side_by_side: bool,

    #[structopt(long = "diff-highlight")]
    /// Emulate diff-highlight (https://github.com/git/git/tree/master/contrib/diff-highlight)
    pub diff_highlight: bool,

    #[structopt(long = "diff-so-fancy")]
    /// Emulate diff-so-fancy (https://github.com/so-fancy/diff-so-fancy)
    pub diff_so_fancy: bool,

    #[structopt(long = "navigate")]
    /// Activate diff navigation: use n to jump forwards and N to jump backwards. To change the
    /// file labels used see --file-modified-label, --file-removed-label, --file-added-label,
    /// --file-renamed-label.
    pub navigate: bool,

    #[structopt(long = "relative-paths")]
    /// Output all file paths relative to the current directory so that they
    /// resolve correctly when clicked on or used in shell commands.
    pub relative_paths: bool,

    #[structopt(long = "hyperlinks")]
    /// Render commit hashes, file names, and line numbers as hyperlinks,
    /// according to the hyperlink spec for terminal emulators:
    /// https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda. By
    /// default, file names and line numbers link to the local file using a file
    /// URL, whereas commit hashes link to the commit in GitHub, if the remote
    /// repository is hosted by GitHub. See --hyperlinks-file-link-format for
    /// full control over the file URLs emitted. Hyperlinks are supported by
    /// several common terminal emulators. To make them work, you must use less
    /// version >= 581 with the -R flag (or use -r with older less versions, but
    /// this will break e.g. --navigate). If you use tmux, then you will also
    /// need a patched fork of tmux (see https://github.com/dandavison/tmux).
    pub hyperlinks: bool,

    #[structopt(long = "keep-plus-minus-markers")]
    /// Prefix added/removed lines with a +/- character, exactly as git does. By default, delta
    /// does not emit any prefix, so code can be copied directly from delta's output.
    pub keep_plus_minus_markers: bool,

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
    /// example: `git show | delta --show-syntax-themes`.
    #[structopt(long = "show-syntax-themes")]
    pub show_syntax_themes: bool,

    /// Show available delta themes, each with an example of highlighted diff
    /// output. A delta theme is a delta named feature (see --features) that
    /// sets either `light` or `dark`. See
    /// https://github.com/dandavison/delta#custom-color-themes. If diff output
    /// is supplied on standard input then this will be used for the demo. For
    /// example: `git show | delta --show-themes`. By default shows dark or
    /// light themes only, according to whether delta is in dark or light mode
    /// (as set by the user or inferred from BAT_THEME). To control the themes
    /// shown, use --dark or --light, or both, on the command line together with
    /// this option.
    #[structopt(long = "show-themes")]
    pub show_themes: bool,

    #[structopt(long = "no-gitconfig")]
    /// Do not take any settings from git config. See GIT CONFIG section.
    pub no_gitconfig: bool,

    #[structopt(long = "raw")]
    /// Do not alter the input in any way. This is mainly intended for testing delta.
    pub raw: bool,

    #[structopt(long = "color-only")]
    /// Do not alter the input structurally in any way, but color and highlight hunk lines
    /// according to your delta configuration. This is mainly intended for other tools that use
    /// delta.
    pub color_only: bool,

    ////////////////////////////////////////////////////////////////////////////////////////////
    #[structopt(long = "features", default_value = "", env = "DELTA_FEATURES")]
    /// Name of delta features to use (space-separated). A feature is a named collection of delta
    /// options in ~/.gitconfig. See FEATURES section.
    pub features: String,

    #[structopt(long = "syntax-theme", env = "BAT_THEME")]
    /// The code syntax-highlighting theme to use. Use --show-syntax-themes to demo available
    /// themes. If the syntax-highlighting theme is not set using this option, it will be taken
    /// from the BAT_THEME environment variable, if that contains a valid theme name.
    /// --syntax-theme=none disables all syntax highlighting.
    pub syntax_theme: Option<String>,

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
    /// The style 'omit' can be used to remove the commit hash line from the output.
    pub commit_style: String,

    #[structopt(long = "commit-decoration-style", default_value = "")]
    /// Style (foreground, background, attributes) for the commit hash decoration. See STYLES
    /// section. The style string should contain one of the special attributes 'box', 'ul'
    /// (underline), 'ol' (overline), or the combination 'ul ol'.
    pub commit_decoration_style: String,

    /// The regular expression used to identify the commit line when parsing git output.
    #[structopt(long = "commit-regex", default_value = r"^commit ")]
    pub commit_regex: String,

    #[structopt(long = "file-style", default_value = "blue")]
    /// Style (foreground, background, attributes) for the file section. See STYLES section. The
    /// style 'omit' can be used to remove the file section from the output.
    pub file_style: String,

    #[structopt(long = "file-decoration-style", default_value = "blue ul")]
    /// Style (foreground, background, attributes) for the file decoration. See STYLES section. The
    /// style string should contain one of the special attributes 'box', 'ul' (underline), 'ol'
    /// (overline), or the combination 'ul ol'.
    pub file_decoration_style: String,

    /// Format string for commit hyperlinks (requires --hyperlinks). The
    /// placeholder "{commit}" will be replaced by the commit hash. For example:
    /// --hyperlinks-commit-link-format='https://mygitrepo/{commit}/'
    #[structopt(long = "hyperlinks-commit-link-format")]
    pub hyperlinks_commit_link_format: Option<String>,

    /// Format string for file hyperlinks (requires --hyperlinks). The
    /// placeholders "{path}" and "{line}" will be replaced by the absolute file
    /// path and the line number, respectively. The default value of this option
    /// creates hyperlinks using standard file URLs; your operating system
    /// should open these in the application registered for that file type.
    /// However, these do not make use of the line number. In order for the link
    /// to open the file at the correct line number, you could use a custom URL
    /// format such as "file-line://{path}:{line}" and register an application
    /// to handle the custom "file-line" URL scheme by opening the file in your
    /// editor/IDE at the indicated line number. See
    /// https://github.com/dandavison/open-in-editor for an example.
    #[structopt(long = "hyperlinks-file-link-format", default_value = "file://{path}")]
    pub hyperlinks_file_link_format: String,

    #[structopt(long = "hunk-header-style", default_value = "line-number syntax")]
    /// Style (foreground, background, attributes) for the hunk-header. See STYLES section. Special
    /// attributes 'file' and 'line-number' can be used to include the file path, and number of
    /// first hunk line, in the hunk header. The style 'omit' can be used to remove the hunk header
    /// section from the output.
    pub hunk_header_style: String,

    #[structopt(long = "hunk-header-file-style", default_value = "blue")]
    /// Style (foreground, background, attributes) for the file path part of the hunk-header. See
    /// STYLES section. The file path will only be displayed if hunk-header-style contains the
    /// 'file' special attribute.
    pub hunk_header_file_style: String,

    #[structopt(long = "hunk-header-line-number-style", default_value = "blue")]
    /// Style (foreground, background, attributes) for the line number part of the hunk-header. See
    /// STYLES section. The line number will only be displayed if hunk-header-style contains the
    /// 'line-number' special attribute.
    pub hunk_header_line_number_style: String,

    #[structopt(long = "hunk-header-decoration-style", default_value = "blue box")]
    /// Style (foreground, background, attributes) for the hunk-header decoration. See STYLES
    /// section. The style string should contain one of the special attributes 'box', 'ul'
    /// (underline), 'ol' (overline), or the combination 'ul ol'.
    pub hunk_header_decoration_style: String,

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
    /// which means to display the line numbers of the minus file (old version), center-aligned,
    /// padded to a width of 4 characters, followed by a dividing character. See the LINE NUMBERS
    /// section.
    #[structopt(long = "line-numbers-left-format", default_value = "{nm:^4}⋮")]
    pub line_numbers_left_format: String,

    /// Format string for the right column of line numbers. A typical value would be "{np:^4}│ "
    /// which means to display the line numbers of the plus file (new version), center-aligned,
    /// padded to a width of 4 characters, followed by a dividing character, and a space. See the
    /// LINE NUMBERS section.
    #[structopt(long = "line-numbers-right-format", default_value = "{np:^4}│")]
    pub line_numbers_right_format: String,

    /// Style (foreground, background, attributes) for the left column of line numbers. See STYLES
    /// and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-left-style", default_value = "auto")]
    pub line_numbers_left_style: String,

    /// Style (foreground, background, attributes) for the right column of line numbers. See STYLES
    /// and LINE NUMBERS sections.
    #[structopt(long = "line-numbers-right-style", default_value = "auto")]
    pub line_numbers_right_style: String,

    #[structopt(long = "file-modified-label", default_value = "")]
    /// Text to display in front of a modified file path.
    pub file_modified_label: String,

    #[structopt(long = "file-removed-label", default_value = "removed:")]
    /// Text to display in front of a removed file path.
    pub file_removed_label: String,

    #[structopt(long = "file-added-label", default_value = "added:")]
    /// Text to display in front of a added file path.
    pub file_added_label: String,

    #[structopt(long = "file-copied-label", default_value = "copied:")]
    /// Text to display in front of a copied file path.
    pub file_copied_label: String,

    #[structopt(long = "file-renamed-label", default_value = "renamed:")]
    /// Text to display in front of a renamed file path.
    pub file_renamed_label: String,

    #[structopt(long = "max-line-length", default_value = "512")]
    /// Truncate lines longer than this. To prevent any truncation, set to zero. Note that
    /// syntax-highlighting very long lines (e.g. minified .js) will be very slow if they are not
    /// truncated.
    pub max_line_length: usize,

    /// The width of underline/overline decorations. Use --width=variable to extend decorations and
    /// background colors to the end of the text only. Otherwise background colors extend to the
    /// full terminal width.
    #[structopt(short = "w", long = "width")]
    pub width: Option<String>,

    /// Width allocated for file paths in a diff stat section. If a relativized
    /// file path exceeds this width then the diff stat will be misaligned.
    #[structopt(long = "diff-stat-align-width", default_value = "48")]
    pub diff_stat_align_width: usize,

    /// The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters
    /// through directly, but note that in that case delta will calculate line widths assuming tabs
    /// occupy one character's width on the screen: if your terminal renders tabs as more than than
    /// one character wide then delta's output will look incorrect.
    #[structopt(long = "tabs", default_value = "4")]
    pub tab_width: usize,

    /// Whether to emit 24-bit ("true color") RGB color codes. Options are auto, always, and never.
    /// "auto" means that delta will emit 24-bit color codes if the environment variable COLORTERM
    /// has the value "truecolor" or "24bit". If your terminal application (the application you use
    /// to enter commands at a shell prompt) supports 24 bit colors, then it probably already sets
    /// this environment variable, in which case you don't need to do anything.
    #[structopt(long = "true-color", default_value = "auto")]
    pub true_color: String,

    /// Deprecated: use --true-color.
    #[structopt(long = "24-bit-color")]
    pub _24_bit_color: Option<String>,

    /// Whether to examine ANSI color escape sequences in raw lines received from Git and handle
    /// lines colored in certain ways specially. This is on by default: it is how Delta supports
    /// Git's --color-moved feature. Set this to "false" to disable this behavior.
    #[structopt(long = "inspect-raw-lines", default_value = "true")]
    pub inspect_raw_lines: String,

    #[structopt(long)]
    /// Which pager to use. The default pager is `less`. You can also change pager
    /// by setting the environment variables DELTA_PAGER, BAT_PAGER, or PAGER
    /// (and that is their order of priority). This option overrides all environment
    /// variables above.
    pub pager: Option<String>,

    /// Whether to use a pager when displaying output. Options are: auto, always, and never.
    #[structopt(long = "paging", default_value = "auto")]
    pub paging_mode: String,

    /// First file to be compared when delta is being used in diff mode: `delta file_1 file_2` is
    /// equivalent to `diff -u file_1 file_2 | delta`.
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

    #[structopt(long = "line-buffer-size", default_value = "32")]
    /// Size of internal line buffer. Delta compares the added and removed versions of nearby lines
    /// in order to detect and highlight changes at the level of individual words/tokens.
    /// Therefore, nearby lines must be buffered internally before they are painted and emitted.
    /// Increasing this value might improve highlighting of some large diff hunks. However, setting
    /// this to a high value will adversely affect delta's performance when entire files are
    /// added/removed.
    pub line_buffer_size: usize,

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

    #[structopt(skip)]
    pub computed: ComputedValues,

    #[structopt(skip)]
    pub git_config_entries: HashMap<String, GitConfigEntry>,
}

#[derive(Default, Clone, Debug)]
pub struct ComputedValues {
    pub available_terminal_width: usize,
    pub background_color_extends_to_terminal_width: bool,
    pub decorations_width: Width,
    pub inspect_raw_lines: InspectRawLines,
    pub is_light_mode: bool,
    pub paging_mode: PagingMode,
    pub syntax_dummy_theme: SyntaxTheme,
    pub syntax_set: SyntaxSet,
    pub syntax_theme: Option<SyntaxTheme>,
    pub true_color: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Width {
    Fixed(usize),
    Variable,
}

impl Default for Width {
    fn default() -> Self {
        Width::Variable
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InspectRawLines {
    True,
    False,
}

impl Default for InspectRawLines {
    fn default() -> Self {
        InspectRawLines::False
    }
}

impl Default for PagingMode {
    fn default() -> Self {
        PagingMode::Never
    }
}

impl Opt {
    pub fn from_args_and_git_config(
        git_config: &mut Option<GitConfig>,
        assets: HighlightingAssets,
    ) -> Self {
        Self::from_clap_and_git_config(Self::clap().get_matches(), git_config, assets)
    }

    pub fn from_iter_and_git_config<I>(iter: I, git_config: &mut Option<GitConfig>) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        let assets = HighlightingAssets::new();
        Self::from_clap_and_git_config(Self::clap().get_matches_from(iter), git_config, assets)
    }

    fn from_clap_and_git_config(
        arg_matches: clap::ArgMatches,
        git_config: &mut Option<GitConfig>,
        assets: HighlightingAssets,
    ) -> Self {
        let mut opt = Opt::from_clap(&arg_matches);
        options::rewrite::apply_rewrite_rules(&mut opt, &arg_matches);
        options::set::set_options(&mut opt, git_config, &arg_matches, assets);
        opt
    }

    #[allow(dead_code)]
    pub fn get_option_names<'a>() -> HashMap<&'a str, &'a str> {
        itertools::chain(
            Self::clap()
                .p
                .opts
                .iter()
                .map(|opt| (opt.b.name, opt.s.long.unwrap())),
            Self::clap()
                .p
                .flags
                .iter()
                .map(|opt| (opt.b.name, opt.s.long.unwrap())),
        )
        .filter(|(name, _)| !IGNORED_OPTION_NAMES.contains(name))
        .collect()
    }
}

// Option names to exclude when listing options to process for various purposes. These are
// (1) Deprecated options
// (2) Pseudo-flag commands such as --list-languages
lazy_static! {
    static ref IGNORED_OPTION_NAMES: HashSet<&'static str> = vec![
        "deprecated-file-color",
        "deprecated-hunk-style",
        "deprecated-minus-background-color",
        "deprecated-minus-emph-background-color",
        "deprecated-hunk-color",
        "deprecated-plus-emph-background-color",
        "deprecated-plus-background-color",
        "deprecated-highlight-minus-lines",
        "deprecated-theme",
        "deprecated-commit-color",
        "list-languages",
        "list-syntax-themes",
        "show-config",
        "show-syntax-themes",
    ]
    .into_iter()
    .collect();
}
