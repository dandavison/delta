use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use bat::assets::HighlightingAssets;
use clap::error::Error;
use clap::{ArgMatches, ColorChoice, CommandFactory, FromArgMatches, Parser, ValueEnum, ValueHint};
use clap_complete::Shell;
use console::Term;
use lazy_static::lazy_static;
use syntect::highlighting::Theme as SyntaxTheme;
use syntect::parsing::SyntaxSet;

use crate::ansi::{ANSI_SGR_BOLD, ANSI_SGR_RESET, ANSI_SGR_UNDERLINE};
use crate::color::ColorMode;
use crate::config::delta_unreachable;
use crate::env::DeltaEnv;
use crate::git_config::GitConfig;
use crate::options;
use crate::subcommands;
use crate::utils;
use crate::utils::bat::output::PagingMode;

const TERM_FALLBACK_WIDTH: usize = 79;

#[derive(Parser)]
#[command(
    name = "delta",
    about = "A viewer for git and diff output",
    version,
    color = ColorChoice::Always,
    // output is wrapped by delta later:
    term_width = usize::MAX,
    max_term_width = usize::MAX,
)]
pub struct Opt {
    #[arg(long = "blame-code-style", value_name = "STYLE")]
    /// Style string for the code section of a git blame line.
    ///
    /// By default the code will be syntax-highlighted with the same background color as the blame
    /// format section of the line (the background color is determined by blame-palette). E.g.
    /// setting this option to 'syntax' will syntax-highlight the code with no background color.
    pub blame_code_style: Option<String>,

    #[arg(
        long = "blame-format",
        default_value = "{timestamp:<15} {author:<15.14} {commit:<8}",
        value_name = "FMT"
    )]
    /// Format string for git blame commit metadata.
    ///
    /// Available placeholders are "{timestamp}", "{author}", and "{commit}".
    pub blame_format: String,

    #[arg(long = "blame-palette", value_name = "COLORS")]
    /// Background colors used for git blame lines (space-separated string).
    ///
    /// Lines added by the same commit are painted with the same color; colors are recycled as
    /// needed.
    pub blame_palette: Option<String>,

    #[arg(
        long = "blame-separator-format",
        default_value = "│{n:^4}│",
        value_name = "FMT"
    )]
    /// Separator between the blame format and the code section of a git blame line.
    ///
    /// Contains the line number by default. Possible values are "none" to disable line numbers or a format
    /// string. This may contain one "{n:}" placeholder and will display the line number on every line.
    /// A type may be added after all other format specifiers and can be separated by '_':
    /// If type is set to 'block' (e.g. "{n:^4_block}") the line number will only be shown when a new blame
    /// block starts; or if it is set to 'every-N' the line will be show with every block and every
    /// N-th (modulo) line.
    pub blame_separator_format: String,

    #[arg(long = "blame-separator-style", value_name = "STYLE")]
    /// Style string for the blame-separator-format.
    pub blame_separator_style: Option<String>,

    #[arg(
        long = "blame-timestamp-format",
        default_value = "%Y-%m-%d %H:%M:%S %z",
        value_name = "FMT"
    )]
    /// Format of `git blame` timestamp in raw git output received by delta.
    pub blame_timestamp_format: String,

    #[arg(long = "blame-timestamp-output-format", value_name = "FMT")]
    /// Format string for git blame timestamp output.
    ///
    /// This string is used for formatting the timestamps in git blame output. It must follow
    /// the `strftime` format syntax specification. If it is not present, the timestamps will
    /// be formatted in a human-friendly but possibly less accurate form.
    ///
    /// See: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    pub blame_timestamp_output_format: Option<String>,

    #[arg(long = "color-only")]
    /// Do not alter the input structurally in any way.
    ///
    /// But color and highlight hunk lines according to your delta configuration. This is mainly
    /// intended for other tools that use delta.
    pub color_only: bool,

    #[arg(long = "config", default_value = "", value_name = "PATH", value_hint = ValueHint::FilePath)]
    /// Load the config file at PATH instead of ~/.gitconfig.
    pub config: String,

    #[arg(
        long = "commit-decoration-style",
        default_value = "",
        value_name = "STYLE"
    )]
    /// Style string for the commit hash decoration.
    ///
    /// See STYLES section. The style string should contain one of the special attributes 'box',
    /// 'ul' (underline), 'ol' (overline), or the combination 'ul ol'.
    pub commit_decoration_style: String,

    #[arg(
        long = "commit-regex",
        default_value = r"^commit ",
        value_name = "REGEX"
    )]
    /// Regular expression used to identify the commit line when parsing git output.
    pub commit_regex: String,

    #[arg(long = "commit-style", default_value = "raw", value_name = "STYLE")]
    /// Style string for the commit hash line.
    ///
    /// See STYLES section. The style 'omit' can be used to remove the commit hash line from the
    /// output.
    pub commit_style: String,

    #[arg(long = "dark")]
    /// Use default colors appropriate for a dark terminal background.
    ///
    /// For more control, see the style options and --syntax-theme.
    pub dark: bool,

    #[arg(long = "default-language", value_name = "LANG", default_value = "txt")]
    /// Default language used for syntax highlighting.
    ///
    /// Used as a fallback when the language cannot be inferred from a filename. It will
    /// typically make sense to set this in the per-repository config file '.git/config'.
    pub default_language: String,

    /// Detect whether or not the terminal is dark or light by querying for its colors.
    ///
    /// Ignored if either `--dark` or `--light` is specified.
    ///
    /// Querying the terminal for its colors requires "exclusive" access
    /// since delta reads/writes from the terminal and enables/disables raw mode.
    /// This causes race conditions with pagers such as less when they are attached to the
    /// same terminal as delta.
    ///
    /// This is usually only an issue when the output is manually piped to a pager.
    /// For example: `git diff | delta | less`.
    /// Otherwise, if delta starts the pager itself, then there's no race condition
    /// since the pager is started *after* the color is detected.
    ///
    /// `auto` tries to account for these situations by testing if the output is redirected.
    ///
    /// The `--color-only` option is treated as an indicator that delta is used
    /// as `interactive.diffFilter`. In this case the color is queried from the terminal even
    /// though the output is redirected.
    ///
    #[arg(long = "detect-dark-light", value_enum, default_value_t = DetectDarkLight::default())]
    pub detect_dark_light: DetectDarkLight,

    #[arg(
        long = "diff-args",
        short = '@',
        default_value = "",
        value_name = "STRING"
    )]
    /// Extra arguments to pass to `git diff` when using delta to diff two files.
    ///
    /// E.g. `delta --diff-args=-U999 file_1 file_2` is equivalent to
    /// `git diff --no-index --color -U999 file_1 file_2 | delta`.
    ///
    /// If you use process substitution (`delta <(command_1) <(command_2)`) and your git version
    /// doesn't support it, then delta will fall back to `diff` instead of `git diff`.
    pub diff_args: String,

    #[arg(long = "diff-highlight")]
    /// Emulate diff-highlight.
    ///
    /// <https://github.com/git/git/tree/master/contrib/diff-highlight>
    pub diff_highlight: bool,

    #[arg(long = "diff-so-fancy")]
    /// Emulate diff-so-fancy.
    ///
    /// <https://github.com/so-fancy/diff-so-fancy>
    pub diff_so_fancy: bool,

    #[arg(long = "diff-stat-align-width", default_value = "48", value_name = "N")]
    /// Width allocated for file paths in a diff stat section.
    ///
    /// If a relativized file path exceeds this width then the diff stat will be misaligned.
    pub diff_stat_align_width: usize,

    #[arg(long = "features", value_name = "FEATURES")]
    /// Names of delta features to activate (space-separated).
    ///
    /// A feature is a named collection of delta options in ~/.gitconfig. See FEATURES section. The
    /// environment variable DELTA_FEATURES can be set to a space-separated list of feature names.
    /// If this is preceded with a + character, the features from the environment variable will be added
    /// to those specified in git config. E.g. DELTA_FEATURES=+side-by-side can be used to activate
    /// side-by-side temporarily (use DELTA_FEATURES=+ to go back to just the features from git config).
    pub features: Option<String>,

    #[arg(
        long = "file-added-label",
        default_value = "added:",
        value_name = "STRING"
    )]
    /// Text to display before an added file path.
    ///
    /// Used in the default value of navigate-regex.
    pub file_added_label: String,

    #[arg(
        long = "file-copied-label",
        default_value = "copied:",
        value_name = "STRING"
    )]
    /// Text to display before a copied file path.
    pub file_copied_label: String,

    #[arg(
        long = "file-decoration-style",
        default_value = "blue ul",
        value_name = "STYLE"
    )]
    /// Style string for the file decoration.
    ///
    /// See STYLES section. The style string should contain one of the special attributes 'box',
    /// 'ul' (underline), 'ol' (overline), or the combination 'ul ol'.
    pub file_decoration_style: String,

    #[arg(
        long = "file-modified-label",
        default_value = "",
        value_name = "STRING"
    )]
    /// Text to display before a modified file path.
    ///
    /// Used in the default value of navigate-regex.
    pub file_modified_label: String,

    #[arg(
        long = "file-removed-label",
        default_value = "removed:",
        value_name = "STRING"
    )]
    /// Text to display before a removed file path.
    ///
    /// Used in the default value of navigate-regex.
    pub file_removed_label: String,

    #[arg(
        long = "file-renamed-label",
        default_value = "renamed:",
        value_name = "STRING"
    )]
    /// Text to display before a renamed file path.
    ///
    /// Used in the default value of navigate-regex.
    pub file_renamed_label: String,

    #[arg(long = "file-style", default_value = "blue", value_name = "STYLE")]
    /// Style string for the file section.
    ///
    /// See STYLES section. The style 'omit' can be used to remove the file section from the output.
    pub file_style: String,

    #[arg(long = "file-transformation", value_name = "SED_CMD")]
    /// Sed-style command transforming file paths for display.
    pub file_regex_replacement: Option<String>,

    #[arg(long = "generate-completion")]
    /// Print completion file for the given shell.
    pub generate_completion: Option<Shell>,

    #[arg(long = "grep-context-line-style", value_name = "STYLE")]
    /// Style string for non-matching lines of grep output.
    ///
    /// See STYLES section. Defaults to zero-style.
    pub grep_context_line_style: Option<String>,

    #[arg(
        long = "grep-file-style",
        default_value = "magenta",
        value_name = "STYLE"
    )]
    /// Style string for file paths in grep output.
    ///
    /// See STYLES section.
    pub grep_file_style: String,

    #[arg(long = "grep-header-decoration-style", value_name = "STYLE")]
    /// Style string for the header decoration in grep output.
    ///
    /// Default is "none" when grep-output-type-is "ripgrep", otherwise defaults
    /// to value of header-decoration-style. See hunk-header-decoration-style.
    pub grep_header_decoration_style: Option<String>,

    #[arg(long = "grep-header-file-style", value_name = "STYLE")]
    /// Style string for the file path part of the header in grep output.
    ///
    /// See hunk_header_file_style.
    pub grep_header_file_style: Option<String>,

    #[arg(
        long = "grep-line-number-style",
        default_value = "green",
        value_name = "STYLE"
    )]
    /// Style string for line numbers in grep output.
    ///
    /// See STYLES section.
    pub grep_line_number_style: String,

    #[arg(long = "grep-output-type", value_name = "OUTPUT_TYPE", value_parser = ["ripgrep", "classic"])]
    /// Grep output format. Possible values:
    /// "ripgrep" - file name printed once, followed by matching lines within that file, each preceded by a line number.
    /// "classic" - file name:line number, followed by matching line.
    /// Default is "ripgrep" if `rg --json` format is detected, otherwise "classic".
    pub grep_output_type: Option<String>,

    #[arg(long = "grep-match-line-style", value_name = "STYLE")]
    /// Style string for matching lines of grep output.
    ///
    /// See STYLES section. Defaults to plus-style.
    pub grep_match_line_style: Option<String>,

    #[arg(long = "grep-match-word-style", value_name = "STYLE")]
    /// Style string for the matching substrings within a matching line of grep output.
    ///
    /// See STYLES section. Defaults to plus-style.
    pub grep_match_word_style: Option<String>,

    #[arg(
        long = "grep-separator-symbol",
        default_value = ":",
        value_name = "STRING"
    )]
    /// Separator symbol printed after the file path and line number in grep output.
    ///
    /// Defaults to ":" for both match and context lines, since many terminal emulators recognize
    /// constructs like "/path/to/file:7:". However, standard grep output uses "-" for context
    /// lines: set this option to "keep" to keep the original separator symbols.
    pub grep_separator_symbol: String,

    #[arg(
        long = "hunk-header-decoration-style",
        default_value = "blue box",
        value_name = "STYLE"
    )]
    /// Style string for the hunk-header decoration.
    ///
    /// See STYLES section. The style string should contain one of the special attributes 'box',
    /// 'ul' (underline), 'ol' (overline), or the combination 'ul ol'.
    pub hunk_header_decoration_style: String,

    #[arg(
        long = "hunk-header-file-style",
        default_value = "blue",
        value_name = "STYLE"
    )]
    /// Style string for the file path part of the hunk-header.
    ///
    /// See STYLES section. The file path will only be displayed if hunk-header-style contains the
    /// 'file' special attribute.
    pub hunk_header_file_style: String,

    #[arg(
        long = "hunk-header-line-number-style",
        default_value = "blue",
        value_name = "STYLE"
    )]
    /// Style string for the line number part of the hunk-header.
    ///
    /// See STYLES section. The line number will only be displayed if hunk-header-style contains the
    /// 'line-number' special attribute.
    pub hunk_header_line_number_style: String,

    #[arg(
        long = "hunk-header-style",
        default_value = "line-number syntax",
        value_name = "STYLE"
    )]
    /// Style string for the hunk-header.
    ///
    /// See STYLES section. Special attributes 'file' and 'line-number' can be used to include the
    /// file path, and number of first hunk line, in the hunk header. The style 'omit' can be used
    /// to remove the hunk header section from the output.
    pub hunk_header_style: String,

    #[arg(long = "hunk-label", default_value = "", value_name = "STRING")]
    /// Text to display before a hunk header.
    ///
    /// Used in the default value of navigate-regex.
    pub hunk_label: String,

    #[arg(long = "hyperlinks")]
    /// Render commit hashes, file names, and line numbers as hyperlinks.
    ///
    /// Following the hyperlink spec for terminal emulators:
    /// <https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda>. By default, file names
    /// and line numbers link to the local file using a file URL, whereas commit hashes link to the
    /// commit in GitHub, if the remote repository is hosted by GitHub. See
    /// --hyperlinks-file-link-format for full control over the file URLs emitted. Hyperlinks are
    /// supported by several common terminal emulators. To make them work, you must use less
    /// version >= 581 with the -R flag (or use -r with older less versions, but this will break
    /// e.g. --navigate). If you use tmux, then you will also need a patched fork of tmux (see
    /// <https://github.com/dandavison/tmux>).
    pub hyperlinks: bool,

    #[arg(long = "hyperlinks-commit-link-format", value_name = "FMT")]
    /// Format string for commit hyperlinks (requires --hyperlinks).
    ///
    /// The placeholder "{commit}" will be replaced by the commit hash. For example:
    /// --hyperlinks-commit-link-format='https://mygitrepo/{commit}/'
    pub hyperlinks_commit_link_format: Option<String>,

    #[arg(
        long = "hyperlinks-file-link-format",
        default_value = "file://{path}",
        value_name = "FMT"
    )]
    /// Format string for file hyperlinks (requires --hyperlinks).
    ///
    /// Placeholders "{path}" and "{line}" will be replaced by the absolute file path and the line
    /// number; "{host}" with the hostname delta is currently running on. The default is to create
    /// a hyperlink containing a standard file URI with only the filename, which your terminal or
    /// OS should handle. You can specify any scheme, such as "file-line://{path}:{line}" and
    /// register an application to handle it. See
    /// <https://dandavison.github.io/delta/hyperlinks.html> for details.
    pub hyperlinks_file_link_format: String,

    #[arg(
        long = "inline-hint-style",
        default_value = "blue",
        value_name = "STYLE"
    )]
    /// Style string for short inline hint text.
    ///
    /// This styles certain content added by delta to the original diff such as special characters
    /// to highlight tabs, and the symbols used to indicate wrapped lines. See STYLES section.
    pub inline_hint_style: String,

    #[arg(
        long = "inspect-raw-lines",
        default_value = "true",
        value_name = "true|false",
        value_parser = ["true", "false"],
    )]
    /// Kill-switch for --color-moved support.
    ///
    /// Whether to examine ANSI color escape sequences in raw lines received from Git and handle
    /// lines colored in certain ways specially. This is on by default: it is how Delta supports
    /// Git's --color-moved feature. Set this to "false" to disable this behavior.
    pub inspect_raw_lines: String,

    #[arg(long = "keep-plus-minus-markers")]
    /// Prefix added/removed lines with a +/- character, as git does.
    ///
    /// By default, delta does not emit any prefix, so code can be copied directly from delta's
    /// output.
    pub keep_plus_minus_markers: bool,

    #[arg(long = "light")]
    /// Use default colors appropriate for a light terminal background.
    ///
    /// For more control, see the style options and --syntax-theme.
    pub light: bool,

    #[arg(long = "line-buffer-size", default_value = "32", value_name = "N")]
    /// Size of internal line buffer.
    ///
    /// Delta compares the added and removed versions of nearby lines in order to detect and
    /// highlight changes at the level of individual words/tokens. Therefore, nearby lines must be
    /// buffered internally before they are painted and emitted. Increasing this value might improve
    /// highlighting of some large diff hunks. However, setting this to a high value will adversely
    /// affect delta's performance when entire files are added/removed.
    pub line_buffer_size: usize,

    #[arg(long = "line-fill-method", value_name = "STRING", value_parser = ["ansi", "spaces"])]
    /// Line-fill method in side-by-side mode.
    ///
    /// How to extend the background color to the end of the line in side-by-side mode. Can be ansi
    /// (default) or spaces (default if output is not to a terminal). Has no effect if
    /// --width=variable is given.
    pub line_fill_method: Option<String>,

    #[arg(short = 'n', long = "line-numbers")]
    /// Display line numbers next to the diff.
    ///
    /// See LINE NUMBERS section.
    pub line_numbers: bool,

    #[arg(
        long = "line-numbers-left-format",
        default_value = "{nm:^4}⋮",
        value_name = "FMT"
    )]
    /// Format string for the left column of line numbers.
    ///
    /// A typical value would be "{nm:^4}⋮" which means to display the line numbers of the minus
    /// file (old version), center-aligned, padded to a width of 4 characters, followed by a
    /// dividing character. See the LINE NUMBERS section.
    pub line_numbers_left_format: String,

    #[arg(
        long = "line-numbers-left-style",
        default_value = "auto",
        value_name = "STYLE"
    )]
    /// Style string for the left column of line numbers.
    ///
    /// See STYLES and LINE NUMBERS sections.
    pub line_numbers_left_style: String,

    #[arg(
        long = "line-numbers-minus-style",
        default_value = "auto",
        value_name = "STYLE"
    )]
    /// Style string for line numbers in the old (minus) version of the file.
    ///
    /// See STYLES and LINE NUMBERS sections.
    pub line_numbers_minus_style: String,

    #[arg(
        long = "line-numbers-plus-style",
        default_value = "auto",
        value_name = "STYLE"
    )]
    /// Style string for line numbers in the new (plus) version of the file.
    ///
    /// See STYLES and LINE NUMBERS sections.
    pub line_numbers_plus_style: String,

    #[arg(
        long = "line-numbers-right-format",
        default_value = "{np:^4}│",
        value_name = "FMT"
    )]
    /// Format string for the right column of line numbers.
    ///
    /// A typical value would be "{np:^4}│ " which means to display the line numbers of the plus
    /// file (new version), center-aligned, padded to a width of 4 characters, followed by a
    /// dividing character, and a space. See the LINE NUMBERS section.
    pub line_numbers_right_format: String,

    #[arg(
        long = "line-numbers-right-style",
        default_value = "auto",
        value_name = "STYLE"
    )]
    /// Style string for the right column of line numbers.
    ///
    /// See STYLES and LINE NUMBERS sections.
    pub line_numbers_right_style: String,

    #[arg(
        long = "line-numbers-zero-style",
        default_value = "auto",
        value_name = "STYLE"
    )]
    /// Style string for line numbers in unchanged (zero) lines.
    ///
    /// See STYLES and LINE NUMBERS sections.
    pub line_numbers_zero_style: String,

    #[arg(long = "list-languages")]
    /// List supported languages and associated file extensions.
    pub list_languages: bool,

    #[arg(long = "list-syntax-themes")]
    /// List available syntax-highlighting color themes.
    pub list_syntax_themes: bool,

    #[arg(long = "map-styles", value_name = "STYLES_MAP")]
    /// Map styles encountered in raw input to desired output styles.
    ///
    /// An example is --map-styles='bold purple => red "#eeeeee", bold cyan => syntax "#eeeeee"'
    pub map_styles: Option<String>,

    #[arg(long = "max-line-distance", default_value = "0.6", value_name = "DIST")]
    /// Maximum line pair distance parameter in within-line diff algorithm.
    ///
    /// This parameter is the maximum distance (0.0 - 1.0) between two lines for them to be inferred
    /// to be homologous. Homologous line pairs are highlighted according to the deletion and
    /// insertion operations transforming one into the other.
    pub max_line_distance: f64,

    #[arg(
        long = "max-syntax-highlighting-length",
        default_value = "400",
        value_name = "N"
    )]
    /// Stop syntax highlighting lines after this many characters.
    ///
    /// To always highlight entire lines, set to zero - but note that delta will be slow on very
    /// long lines (e.g. minified .js).
    pub max_syntax_length: usize,

    #[arg(long = "max-line-length", default_value = "3000", value_name = "N")]
    /// Truncate lines longer than this.
    ///
    /// To prevent any truncation, set to zero. When wrapping lines this does nothing as it is
    /// overwritten to fit at least all visible characters, see `--wrap-max-lines`.
    pub max_line_length: usize,

    #[arg(
        long = "merge-conflict-begin-symbol",
        default_value = "▼",
        value_name = "STRING"
    )]
    /// String marking the beginning of a merge conflict region.
    ///
    /// The string will be repeated until it reaches the required length.
    pub merge_conflict_begin_symbol: String,

    #[arg(
        long = "merge-conflict-end-symbol",
        default_value = "▲",
        value_name = "STRING"
    )]
    /// String marking the end of a merge conflict region.
    ///
    /// The string will be repeated until it reaches the required length.
    pub merge_conflict_end_symbol: String,

    #[arg(
        long = "merge-conflict-ours-diff-header-decoration-style",
        default_value = "box",
        value_name = "STYLE"
    )]
    /// Style string for the decoration of the header above the 'ours' merge conflict diff.
    ///
    /// This styles the decoration of the header above the diff between the ancestral commit and the
    /// 'ours' branch. See STYLES section. The style string should contain one of the special
    /// attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol'.
    pub merge_conflict_ours_diff_header_decoration_style: String,

    #[arg(
        long = "merge-conflict-ours-diff-header-style",
        default_value = "normal",
        value_name = "STYLE"
    )]
    /// Style string for the header above the 'ours' branch merge conflict diff.
    ///
    /// See STYLES section.
    pub merge_conflict_ours_diff_header_style: String,

    #[arg(
        long = "merge-conflict-theirs-diff-header-decoration-style",
        default_value = "box",
        value_name = "STYLE"
    )]
    /// Style string for the decoration of the header above the 'theirs' merge conflict diff.
    ///
    /// This styles the decoration of the header above the diff between the ancestral commit and
    /// 'their' branch.  See STYLES section. The style string should contain one of the special
    /// attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol'.
    pub merge_conflict_theirs_diff_header_decoration_style: String,

    #[arg(
        long = "merge-conflict-theirs-diff-header-style",
        default_value = "normal",
        value_name = "STYLE"
    )]
    /// Style string for the header above the 'theirs' branch merge conflict diff.
    ///
    /// This styles the header above the diff between the ancestral commit and 'their' branch. See
    /// STYLES section.
    pub merge_conflict_theirs_diff_header_style: String,

    #[arg(
        long = "minus-empty-line-marker-style",
        default_value = "normal auto",
        value_name = "STYLE"
    )]
    /// Style string for removed empty line marker.
    ///
    /// Used only if --minus-style has no background color.
    pub minus_empty_line_marker_style: String,

    #[arg(
        long = "minus-emph-style",
        default_value = "normal auto",
        value_name = "STYLE"
    )]
    /// Style string for emphasized sections of removed lines.
    ///
    /// See STYLES section.
    pub minus_emph_style: String,

    #[arg(
        long = "minus-non-emph-style",
        default_value = "minus-style",
        value_name = "STYLE"
    )]
    /// Style string for non-emphasized sections of removed lines that have an emphasized section.
    ///
    /// See STYLES section.
    pub minus_non_emph_style: String,

    #[arg(
        long = "minus-style",
        default_value = "normal auto",
        value_name = "STYLE"
    )]
    /// Style string for removed lines.
    ///
    /// See STYLES section.
    pub minus_style: String,

    #[arg(long = "navigate")]
    /// Activate diff navigation.
    ///
    /// Use n to jump forwards and N to jump backwards. To change the file labels used see
    /// --file-added-label, --file-copied-label, --file-modified-label, --file-removed-label, --file-renamed-label.
    pub navigate: bool,

    #[arg(long = "navigate-regex", value_name = "REGEX")]
    /// Regular expression defining navigation stop points.
    pub navigate_regex: Option<String>,

    #[arg(long = "no-gitconfig")]
    /// Do not read any settings from git config.
    ///
    /// See GIT CONFIG section.
    pub no_gitconfig: bool,

    #[arg(long = "pager", value_name = "CMD")]
    /// Which pager to use.
    ///
    /// The default pager is `less`. You can also change pager by setting the
    /// environment variable DELTA_PAGER, or PAGER. This option overrides these
    /// environment variables.
    pub pager: Option<String>,

    #[arg(
        long = "paging",
        default_value = "auto",
        value_name = "auto|always|never",
        value_parser = ["auto", "always", "never"],
    )]
    /// Whether to use a pager when displaying output.
    ///
    /// Options are: auto, always, and never.
    pub paging_mode: String,

    #[arg(long = "parse-ansi")]
    /// Display ANSI color escape sequences in human-readable form.
    ///
    /// Example usage: git show --color=always | delta --parse-ansi
    /// This can be used to help identify input style strings to use with map-styles.
    pub parse_ansi: bool,

    #[arg(
        long = "plus-emph-style",
        default_value = "syntax auto",
        value_name = "STYLE"
    )]
    /// Style string for emphasized sections of added lines.
    ///
    /// See STYLES section.
    pub plus_emph_style: String,

    #[arg(
        long = "plus-empty-line-marker-style",
        default_value = "normal auto",
        value_name = "STYLE"
    )]
    /// Style string for added empty line marker.
    ///
    /// Used only if --plus-style has no background color.
    pub plus_empty_line_marker_style: String,

    #[arg(
        long = "plus-non-emph-style",
        default_value = "plus-style",
        value_name = "STYLE"
    )]
    /// Style string for non-emphasized sections of added lines that have an emphasized section.
    ///
    /// See STYLES section.
    pub plus_non_emph_style: String,

    #[arg(
        long = "plus-style",
        default_value = "syntax auto",
        value_name = "STYLE"
    )]
    /// Style string for added lines.
    ///
    /// See STYLES section.
    pub plus_style: String,

    #[arg(long = "raw")]
    /// Do not alter the input in any way.
    ///
    /// This is mainly intended for testing delta.
    pub raw: bool,

    #[arg(long = "relative-paths")]
    /// Output all file paths relative to the current directory.
    ///
    /// This means that they will resolve correctly when clicked on or used in shell commands.
    pub relative_paths: bool,

    #[arg(long = "right-arrow", default_value = "⟶  ", value_name = "STRING")]
    /// Text to display with a changed file path.
    ///
    /// For example, a unified diff heading, a rename, or a chmod.
    pub right_arrow: String,

    #[arg(long = "show-colors")]
    /// Show available named colors.
    ///
    /// In addition to named colors, arbitrary colors can be specified using RGB hex codes. See
    /// COLORS section.
    pub show_colors: bool,

    #[arg(long = "show-config")]
    /// Display the active values for all Delta options.
    ///
    /// Style string options are displayed with foreground and background colors. This can be used to
    /// experiment with colors by combining this option with other options such as --minus-style,
    /// --zero-style, --plus-style, --light, --dark, etc.
    pub show_config: bool,

    #[arg(long = "show-syntax-themes")]
    /// Show example diff for available syntax-highlighting themes.
    ///
    /// If diff output is supplied on standard input then this will be used for the demo. For
    /// example: `git show | delta --show-syntax-themes`.
    pub show_syntax_themes: bool,

    #[arg(long = "show-themes")]
    /// Show example diff for available delta themes.
    ///
    /// A delta theme is a delta named feature (see --features) that sets either `light` or `dark`.
    /// See <https://github.com/dandavison/delta#custom-color-themes>. If diff output is supplied on
    /// standard input then this will be used for the demo. For example: `git show | delta
    /// --show-themes`. By default shows dark or light themes only, according to whether delta is in
    /// dark or light mode (as set by the user or inferred from BAT_THEME). To control the themes
    /// shown, use --dark or --light, or both, on the command line together with this option.
    pub show_themes: bool,

    #[arg(short = 's', long = "side-by-side")]
    /// Display diffs in side-by-side layout.
    pub side_by_side: bool,

    #[arg(long = "syntax-theme", value_name = "SYNTAX_THEME")]
    /// The syntax-highlighting theme to use.
    ///
    /// Use --show-syntax-themes to demo available themes. Defaults to the value of the BAT_THEME
    /// environment variable, if that contains a valid theme name. --syntax-theme=none disables all
    /// syntax highlighting.
    pub syntax_theme: Option<String>,

    #[arg(long = "tabs", default_value = "8", value_name = "N")]
    /// The number of spaces to replace tab characters with.
    ///
    /// Use --tabs=0 to pass tab characters through directly, but note that in that case delta will
    /// calculate line widths assuming tabs occupy one character's width on the screen: if your
    /// terminal renders tabs as more than one character wide then delta's output will look
    /// incorrect.
    pub tab_width: usize,

    #[arg(
        long = "true-color",
        default_value = "auto",
        value_name = "auto|always|never",
        value_parser = ["auto", "always", "never"],
    )]
    /// Whether to emit 24-bit ("true color") RGB color codes.
    ///
    /// Options are auto, always, and never. "auto" means that delta will emit 24-bit color codes if
    /// the environment variable COLORTERM has the value "truecolor" or "24bit". If your terminal
    /// application (the application you use to enter commands at a shell prompt) supports 24 bit
    /// colors, then it probably already sets this environment variable, in which case you don't
    /// need to do anything.
    pub true_color: String,

    #[arg(
        long = "whitespace-error-style",
        default_value = "auto auto",
        value_name = "STYLE"
    )]
    /// Style string for whitespace errors.
    ///
    /// Defaults to color.diff.whitespace if that is set in git config, or else 'magenta reverse'.
    pub whitespace_error_style: String,

    #[arg(short = 'w', long = "width", value_name = "N")]
    /// The width of underline/overline decorations.
    ///
    /// Examples: "72" (exactly 72 characters), "-2" (auto-detected terminal width minus 2). An
    /// expression such as "74-2" is also valid (equivalent to 72 but may be useful if the caller
    /// has a variable holding the value "74"). Use --width=variable to extend decorations and
    /// background colors to the end of the text only. Otherwise background colors extend to the
    /// full terminal width.
    pub width: Option<String>,

    #[arg(long = "word-diff-regex", default_value = r"\w+", value_name = "REGEX")]
    /// Regular expression defining a 'word' in within-line diff algorithm.
    ///
    /// The regular expression used to decide what a word is for the within-line highlight
    /// algorithm. For less fine-grained matching than the default try --word-diff-regex="\S+"
    /// --max-line-distance=1.0 (this is more similar to `git --word-diff`).
    pub tokenization_regex: String,

    #[arg(long = "wrap-left-symbol", default_value = "↵", value_name = "STRING")]
    /// End-of-line wrapped content symbol (left-aligned).
    ///
    /// Symbol added to the end of a line indicating that the content has been wrapped onto the next
    /// line and continues left-aligned.
    pub wrap_left_symbol: String,

    #[arg(long = "wrap-max-lines", default_value = "2", value_name = "N")]
    /// How often a line should be wrapped if it does not fit.
    ///
    /// Zero means to never wrap. Any content which does not fit after wrapping will be truncated. A
    /// value of "unlimited" means a line will be wrapped as many times as required.
    pub wrap_max_lines: String,

    #[arg(
        long = "wrap-right-percent",
        default_value = "37.0",
        value_name = "PERCENT"
    )]
    /// Threshold for right-aligning wrapped content.
    ///
    /// If the length of the remaining wrapped content, as a percentage of width, is less than this
    /// quantity it will be right-aligned. Otherwise it will be left-aligned.
    pub wrap_right_percent: String,

    #[arg(
        long = "wrap-right-prefix-symbol",
        default_value = "…",
        value_name = "STRING"
    )]
    /// Pre-wrapped content symbol (right-aligned).
    ///
    /// Symbol displayed before right-aligned wrapped content.
    pub wrap_right_prefix_symbol: String,

    #[arg(long = "wrap-right-symbol", default_value = "↴", value_name = "STRING")]
    /// End-of-line wrapped content symbol (right-aligned).
    ///
    /// Symbol added to the end of a line indicating that the content has been wrapped onto the next
    /// line and continues right-aligned.
    pub wrap_right_symbol: String,

    #[arg(
        long = "zero-style",
        default_value = "syntax normal",
        value_name = "STYLE"
    )]
    /// Style string for unchanged lines.
    ///
    /// See STYLES section.
    pub zero_style: String,

    #[arg(long = "24-bit-color", value_name = "auto|always|never", value_parser = ["auto", "always", "never"])]
    /// Deprecated: use --true-color.
    pub _24_bit_color: Option<String>,

    /// First file to be compared when delta is being used to diff two files.
    ///
    /// `delta file1 file2` is equivalent to `diff -u file1 file2 | delta`.
    pub minus_file: Option<PathBuf>,

    /// Second file to be compared when delta is being used to diff two files.
    pub plus_file: Option<PathBuf>,

    #[arg(skip)]
    pub computed: ComputedValues,

    #[arg(skip)]
    pub git_config: Option<GitConfig>,

    #[arg(skip)]
    pub env: DeltaEnv,
}

fn get_after_long_help(is_term: bool, no_indent: &str, no_wrap: &str) -> String {
    let i0 = no_indent;
    let l = no_wrap;

    #[allow(non_snake_case)]
    // <H>header</H> and  <u>underline</u>
    let (H_, _H, u_, _u) = if is_term {
        (
            format!("{ANSI_SGR_BOLD}{ANSI_SGR_UNDERLINE}"),
            ANSI_SGR_RESET,
            ANSI_SGR_UNDERLINE,
            ANSI_SGR_RESET,
        )
    } else {
        ("".to_string(), "", "", "")
    };

    format!(
        r##"

{i0}{H_}Git config{_H}

By default, delta takes settings from a section named "delta" in git config files, if one is present. The git config file to use for delta options will usually be ~/.gitconfig, but delta follows the rules given in <https://git-scm.com/docs/git-config#FILES>. Most delta options can be given in a git config file, using the usual option names but without the initial '--'. An example is

[delta]
    line-numbers = true
    zero-style = dim syntax


{i0}{H_}Features{_H}

A feature is a named collection of delta options in git config. An example is:

[delta "my-delta-feature"]
    syntax-theme = Dracula
    plus-style = bold syntax "#002800"

To activate those options, you would use:

delta --features my-delta-feature

A feature name may not contain whitespace. You can activate multiple features:

{l}[delta]
{l}    features = my-highlight-styles-colors-feature my-line-number-styles-feature

If more than one feature sets the same option, the last one wins.

If an option is present in the [delta] section, then features are not considered at all.

If you want an option to be fully overridable by a feature and also have a non default value when no features are used, then you need to define a "default" feature and include it in the main delta configuration.

For instance:

[delta]
    feature = default-feature

[delta "default-feature"]
    width = 123

At this point, you can override features set in the command line or in the environment variables and the "last one wins" rules will apply as expected.


{i0}{H_}Styles{_H}

All options that have a name like --*-style work the same way. It is very similar to how colors/styles are specified in a gitconfig file: <https://git-scm.com/docs/git-config#Documentation/git-config.txt-color>

Here is an example:

--minus-style 'red bold ul "#ffeeee"'

That means: For removed lines, set the foreground (text) color to 'red', make it bold and underlined, and set the background color to '#ffeeee'.

See the {u_}Colors{_u} section below for how to specify a color. In addition to real colors, there are 4 special color names: 'auto', 'normal', 'raw', and 'syntax'.

Here is an example of using special color names together with a single attribute:

--minus-style 'syntax bold auto'

That means: For removed lines, syntax-highlight the text, and make it bold, and do whatever delta normally does for the background.

The available attributes are: 'blink', 'bold', 'dim', 'hidden', 'italic', 'reverse', 'strike', and 'ul' (or 'underline').

The attribute 'omit' is supported by commit-style, file-style, and hunk-header-style, meaning to remove the element entirely from the output.

A complete description of the style string syntax follows:

- If the input that delta is receiving already has colors, and you want delta to output those colors unchanged, then use the special style string 'raw'. Otherwise, delta will strip any colors from its input.

- A style string consists of 0, 1, or 2 colors, together with an arbitrary number of style attributes, all separated by spaces.

- The first color is the foreground (text) color. The second color is the background color. Attributes can go in any position.

- This means that in order to specify a background color you must also specify a foreground (text) color.

- If you want delta to choose one of the colors automatically, then use the special color 'auto'. This can be used for both foreground and background.

- If you want the foreground/background color to be your terminal's foreground/background color, then use the special color 'normal'.

- If you want the foreground text to be syntax-highlighted according to its language, then use the special foreground color 'syntax'. This can only be used for the foreground (text).

- The minimal style specification is the empty string ''. This means: do not apply any colors or styling to the element in question.


{i0}{H_}Colors{_H}

There are four ways to specify a color (this section applies to foreground and background colors within a style string):

1. CSS color name

   Any of the 140 color names used in CSS: <https://www.w3schools.com/colors/colors_groups.asp>

2. RGB hex code

   An example of using an RGB hex code is:
   --file-style="#0e7c0e"

3. ANSI color name

   There are 8 ANSI color names:
   black, red, green, yellow, blue, magenta, cyan, white.

   In addition, all of them have a bright form:
   brightblack, brightred, brightgreen, brightyellow, brightblue, brightmagenta, brightcyan, brightwhite.

   An example of using an ANSI color name is:
   --file-style="green"

   Unlike RGB hex codes, ANSI color names are just names: you can choose the exact color that each name corresponds to in the settings of your terminal application (the application you use to enter commands at a shell prompt). This means that if you use ANSI color names, and you change the color theme used by your terminal, then delta's colors will respond automatically, without needing to change the delta command line.

   "purple" is accepted as a synonym for "magenta". Color names and codes are case-insensitive.

4. ANSI color number

   An example of using an ANSI color number is:
   --file-style=28

   There are 256 ANSI color numbers: 0-255. The first 16 are the same as the colors described in the "ANSI color name" section above. See <https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit>. Specifying colors like this is useful if your terminal only supports 256 colors (i.e. doesn't support 24-bit color).


{i0}{H_}Line Numbers{_H}

To display line numbers, use --line-numbers.

Line numbers are displayed in two columns. Here's what it looks like by default:

  1 ⋮  1 │ unchanged line
  2 ⋮    │ removed line
    ⋮  2 │ added line

In that output, the line numbers for the old (minus) version of the file appear in the left column, and the line numbers for the new (plus) version of the file appear in the right column. In an unchanged (zero) line, both columns contain a line number.

The following options allow the line number display to be customized:

--line-numbers-left-format:  Change the contents of the left column
--line-numbers-right-format: Change the contents of the right column
--line-numbers-left-style:   Change the style applied to the left column
--line-numbers-right-style:  Change the style applied to the right column
--line-numbers-minus-style:  Change the style applied to line numbers in minus lines
--line-numbers-zero-style:   Change the style applied to line numbers in unchanged lines
--line-numbers-plus-style:   Change the style applied to line numbers in plus lines

Options --line-numbers-left-format and --line-numbers-right-format allow you to change the contents of the line number columns. Their values are arbitrary format strings, which are allowed to contain the placeholders {{nm}} for the line number associated with the old version of the file and {{np}} for the line number associated with the new version of the file. The placeholders support a subset of the string formatting syntax documented here: <https://doc.rust-lang.org/std/fmt/#formatting-parameters>. Specifically, you can use the alignment and width syntax.

For example, the default value of --line-numbers-left-format is '{{nm:^4}}⋮'. This means that the left column should display the minus line number (nm), center-aligned, padded with spaces to a width of 4 characters, followed by a unicode dividing-line character (⋮).

Similarly, the default value of --line-numbers-right-format is '{{np:^4}}│'. This means that the right column should display the plus line number (np), center-aligned, padded with spaces to a width of 4 characters, followed by a unicode dividing-line character (│).

Use '<' for left-align, '^' for center-align, and '>' for right-align.


{i0}{H_}Support{_H}

If something isn't working correctly, or you have a feature request, please open an issue at <https://github.com/dandavison/delta/issues>.

For a short help summary, please use delta -h.
"##
    )
}

#[derive(Default, Clone, Debug)]
pub struct ComputedValues {
    pub available_terminal_width: usize,
    pub stdout_is_term: bool,
    pub background_color_extends_to_terminal_width: bool,
    pub decorations_width: Width,
    pub inspect_raw_lines: InspectRawLines,
    pub color_mode: ColorMode,
    pub paging_mode: PagingMode,
    pub syntax_set: SyntaxSet,
    pub syntax_theme: Option<SyntaxTheme>,
    pub true_color: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Width {
    Fixed(usize),
    #[default]
    Variable,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InspectRawLines {
    True,
    #[default]
    False,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum DetectDarkLight {
    /// Only query the terminal for its colors if the output is not redirected.
    #[default]
    Auto,
    /// Always query the terminal for its colors.
    Always,
    /// Never query the terminal for its colors.
    Never,
}

// Which call path to take
#[derive(Debug)]
pub enum Call<T> {
    Delta(T),
    DeltaDiff(T, PathBuf, PathBuf),
    SubCommand(T, subcommands::SubCommand),
    Help(String),
    Version(String),
}

impl Opt {
    fn handle_help_and_version(args: &[OsString]) -> Call<ArgMatches> {
        match Self::command().try_get_matches_from(args) {
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayVersion => {
                let version = Self::command().render_version();
                Call::Version(version)
            }
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
                let term = Term::stdout();
                // No wrapping if short -h instead of --help was used:
                if args.iter().any(|arg| arg == "-h") {
                    let help_clap = Self::command().render_help();
                    return Call::Help(if term.is_term() {
                        help_clap.ansi().to_string()
                    } else {
                        help_clap.to_string()
                    });
                }

                let help_clap = Self::command().render_long_help();
                let (help_clap, wrap_width) = if term.is_term() {
                    (
                        help_clap.ansi().to_string(),
                        utils::workarounds::windows_msys2_width_fix(term.size(), &term),
                    )
                } else {
                    (help_clap.to_string(), TERM_FALLBACK_WIDTH)
                };

                // Stop wrapping for very narrow terminals, and leave a 2 wide margin on the right.
                let wrap_width =
                    wrap_width.clamp(TERM_FALLBACK_WIDTH - 22, TERM_FALLBACK_WIDTH + 22) - 2;

                let mut help = utils::helpwrap::wrap(&help_clap, wrap_width, "", "", "");
                let indent_with = "  ";
                let no_indent = ":no_Indent:";
                let no_wrap = ":no_Wrap:";
                let after_help = utils::helpwrap::wrap(
                    &get_after_long_help(term.is_term(), no_indent, no_wrap),
                    wrap_width,
                    indent_with,
                    no_indent,
                    no_wrap,
                );
                help.push_str(&after_help);

                Call::Help(help)
            }
            Err(e) => {
                // Calls `e.exit()` if error persists.
                let (matches, subcmd) = subcommands::extract(args, e);
                Call::SubCommand(matches, subcmd)
            }
            Ok(matches) => {
                // subcommands take precedence over diffs
                let minus_file = matches.get_one::<PathBuf>("minus_file").map(PathBuf::from);
                if let Some(subcmd) = &minus_file {
                    if let Some(arg) = subcmd.to_str() {
                        if subcommands::SUBCOMMANDS.contains(&arg) {
                            let unreachable_error =
                                Error::new(clap::error::ErrorKind::InvalidSubcommand);
                            let (matches, subcmd) = subcommands::extract(args, unreachable_error);
                            return Call::SubCommand(matches, subcmd);
                        }
                    }
                }

                match (
                    minus_file,
                    matches.get_one::<PathBuf>("plus_file").map(PathBuf::from),
                ) {
                    (Some(minus_file), Some(plus_file)) => {
                        Call::DeltaDiff(matches, minus_file, plus_file)
                    }
                    _ => Call::Delta(matches),
                }
            }
        }
    }

    pub fn from_args_and_git_config(
        args: Vec<OsString>,
        env: &DeltaEnv,
        assets: HighlightingAssets,
    ) -> (Call<()>, Option<Opt>) {
        #[cfg(test)]
        // Set argv[0] when called in tests:
        let args = {
            let mut args = args;
            args.insert(0, OsString::from("delta"));
            args
        };
        let (matches, call) = match Self::handle_help_and_version(&args) {
            Call::Delta(t) => (t, Call::Delta(())),
            Call::DeltaDiff(t, a, b) => (t, Call::DeltaDiff((), a, b)),
            Call::SubCommand(t, cmd) => (t, Call::SubCommand((), cmd)),
            Call::Help(help) => return (Call::Help(help), None),
            Call::Version(ver) => return (Call::Version(ver), None),
        };

        let mut final_config = if *matches.get_one::<bool>("no_gitconfig").unwrap_or(&false) {
            None
        } else {
            GitConfig::try_create(env)
        };

        if let Some(path) = matches.get_one::<String>("config") {
            if !path.is_empty() {
                let path = Path::new(path);
                final_config = Some(GitConfig::from_path(env, path, true));
            }
        }

        let opt = Self::from_clap_and_git_config(env, matches, final_config, assets);
        (call, Some(opt))
    }

    pub fn from_iter_and_git_config<I>(
        env: &DeltaEnv,
        iter: I,
        git_config: Option<GitConfig>,
    ) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        let assets = utils::bat::assets::load_highlighting_assets();
        Self::from_clap_and_git_config(
            env,
            Self::command().get_matches_from(iter),
            git_config,
            assets,
        )
    }

    fn from_clap_and_git_config(
        env: &DeltaEnv,
        arg_matches: clap::ArgMatches,
        mut git_config: Option<GitConfig>,
        assets: HighlightingAssets,
    ) -> Self {
        let mut opt = Opt::from_arg_matches(&arg_matches)
            .unwrap_or_else(|_| delta_unreachable("Opt::from_arg_matches failed"));
        opt.env = env.clone();
        options::set::set_options(&mut opt, &mut git_config, &arg_matches, assets);
        opt.git_config = git_config;
        opt
    }

    pub fn get_argument_and_option_names() -> HashMap<String, String> {
        let command = Self::command();
        command
            .get_opts()
            .chain(command.get_arguments())
            .filter_map(|arg| match (arg.get_id(), arg.get_long()) {
                (id, Some(long)) => {
                    if IGNORED_OPTION_NAMES.contains(long) {
                        None
                    } else {
                        Some((id.to_string(), long.to_owned()))
                    }
                }
                _ => None,
            })
            .collect()
    }

    pub fn git_config(&self) -> Option<&GitConfig> {
        self.git_config.as_ref()
    }
}

// Option names to exclude when listing options to process for various purposes. These are all
// pseudo-flag commands such as --list-languages
lazy_static! {
    static ref IGNORED_OPTION_NAMES: HashSet<&'static str> = vec![
        "generate-completion",
        "list-languages",
        "list-syntax-themes",
        "show-config",
        "show-syntax-themes",
    ]
    .into_iter()
    .collect();
}
