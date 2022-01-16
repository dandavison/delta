# Full --help output

```
delta 0.11.3
A viewer for git and diff output

USAGE:
    delta [FLAGS] [OPTIONS] [ARGS]

FLAGS:
        --light                      Use default colors appropriate for a light terminal background. For more control,
                                     see the style options and --syntax-theme
        --dark                       Use default colors appropriate for a dark terminal background. For more control,
                                     see the style options and --syntax-theme
    -n, --line-numbers               Display line numbers next to the diff. See LINE NUMBERS section
    -s, --side-by-side               Display a side-by-side diff view instead of the traditional view
        --diff-highlight             Emulate diff-highlight (https://github.com/git/git/tree/master/contrib/diff-highlight)
        --diff-so-fancy              Emulate diff-so-fancy (https://github.com/so-fancy/diff-so-fancy)
        --navigate                   Activate diff navigation: use n to jump forwards and N to jump backwards. To change
                                     the file labels used see --file-modified-label, --file-removed-label, --file-added-
                                     label, --file-renamed-label
        --relative-paths             Output all file paths relative to the current directory so that they resolve
                                     correctly when clicked on or used in shell commands
        --hyperlinks                 Render commit hashes, file names, and line numbers as hyperlinks, according to the
                                     hyperlink spec for terminal emulators:
                                     https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda. By default,
                                     file names and line numbers link to the local file using a file URL, whereas commit
                                     hashes link to the commit in GitHub, if the remote repository is hosted by GitHub.
                                     See --hyperlinks-file-link-format for full control over the file URLs emitted.
                                     Hyperlinks are supported by several common terminal emulators. To make them work,
                                     you must use less version >= 581 with the -R flag (or use -r with older less
                                     versions, but this will break e.g. --navigate). If you use tmux, then you will also
                                     need a patched fork of tmux (see https://github.com/dandavison/tmux)
        --keep-plus-minus-markers    Prefix added/removed lines with a +/- character, exactly as git does. By default,
                                     delta does not emit any prefix, so code can be copied directly from delta's output
        --show-config                Display the active values for all Delta options. Style options are displayed with
                                     foreground and background colors. This can be used to experiment with colors by
                                     combining this option with other options such as --minus-style, --zero-style,
                                     --plus-style, --light, --dark, etc
        --list-languages             List supported languages and associated file extensions
        --list-syntax-themes         List available syntax-highlighting color themes
        --show-syntax-themes         Show all available syntax-highlighting themes, each with an example of highlighted
                                     diff output. If diff output is supplied on standard input then this will be used
                                     for the demo. For example: `git show | delta --show-syntax-themes`
        --show-themes                Show available delta themes, each with an example of highlighted diff output. A
                                     delta theme is a delta named feature (see --features) that sets either `light` or
                                     `dark`. See https://github.com/dandavison/delta#custom-color-themes. If diff output
                                     is supplied on standard input then this will be used for the demo. For example:
                                     `git show | delta --show-themes`. By default shows dark or light themes only,
                                     according to whether delta is in dark or light mode (as set by the user or inferred
                                     from BAT_THEME). To control the themes shown, use --dark or --light, or both, on
                                     the command line together with this option
        --show-colors                Show available named colors. In addition to named colors, arbitrary colors can be
                                     specified using RGB hex codes. See COLORS section
        --parse-ansi                 Parse ANSI color escape sequences in input and display them as git style strings.
                                     Example usage: git show --color=always | delta --parse-ansi This can be used to
                                     help identify input style strings to use with map-styles
        --no-gitconfig               Do not take any settings from git config. See GIT CONFIG section
        --raw                        Do not alter the input in any way. This is mainly intended for testing delta
        --color-only                 Do not alter the input structurally in any way, but color and highlight hunk lines
                                     according to your delta configuration. This is mainly intended for other tools that
                                     use delta
        --highlight-removed          Deprecated: use --minus-style='syntax'
    -h, --help                       Prints help information
    -V, --version                    Prints version information

OPTIONS:
        --features <features>
            Name of delta features to use (space-separated). A feature is a named collection of delta options in
            ~/.gitconfig. See FEATURES section. The environment variable DELTA_FEATURES can be set to a space-separated
            list of feature names. If this is preceded with a space, the features from the environment variable will be
            added to those specified in git config. E.g. DELTA_FEATURES=+side-by-side can be used to activate side-by-
            side temporarily
        --syntax-theme <syntax-theme>
            The code syntax-highlighting theme to use. Use --show-syntax-themes to demo available themes. Defaults to
            the value of the BAT_THEME environment variable, if that contains a valid theme name. --syntax-theme=none
            disables all syntax highlighting
        --minus-style <minus-style>
            Style (foreground, background, attributes) for removed lines. See STYLES section [default: normal auto]
        --zero-style <zero-style>
            Style (foreground, background, attributes) for unchanged lines. See STYLES section [default: syntax normal]
        --plus-style <plus-style>
            Style (foreground, background, attributes) for added lines. See STYLES section [default: syntax auto]
        --minus-emph-style <minus-emph-style>
            Style (foreground, background, attributes) for emphasized sections of removed lines. See STYLES section
            [default: normal auto]
        --minus-non-emph-style <minus-non-emph-style>
            Style (foreground, background, attributes) for non-emphasized sections of removed lines that have an
            emphasized section. See STYLES section [default: minus-style]
        --plus-emph-style <plus-emph-style>
            Style (foreground, background, attributes) for emphasized sections of added lines. See STYLES section
            [default: syntax auto]
        --plus-non-emph-style <plus-non-emph-style>
            Style (foreground, background, attributes) for non-emphasized sections of added lines that have an
            emphasized section. See STYLES section [default: plus-style]
        --commit-style <commit-style>
            Style (foreground, background, attributes) for the commit hash line. See STYLES section. The style 'omit'
            can be used to remove the commit hash line from the output [default: raw]
        --commit-decoration-style <commit-decoration-style>
            Style (foreground, background, attributes) for the commit hash decoration. See STYLES section. The style
            string should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the
            combination 'ul ol' [default: ]
        --commit-regex <commit-regex>
            The regular expression used to identify the commit line when parsing git output [default: ^commit ]

        --file-style <file-style>
            Style (foreground, background, attributes) for the file section. See STYLES section. The style 'omit' can be
            used to remove the file section from the output [default: blue]
        --file-decoration-style <file-decoration-style>
            Style (foreground, background, attributes) for the file decoration. See STYLES section. The style string
            should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the combination
            'ul ol' [default: blue ul]
        --hyperlinks-commit-link-format <hyperlinks-commit-link-format>
            Format string for commit hyperlinks (requires --hyperlinks). The placeholder "{commit}" will be replaced by
            the commit hash. For example: --hyperlinks-commit-link-format='https://mygitrepo/{commit}/'
        --hyperlinks-file-link-format <hyperlinks-file-link-format>
            Format string for file hyperlinks (requires --hyperlinks). The placeholders "{path}" and "{line}" will be
            replaced by the absolute file path and the line number, respectively. The default value of this option
            creates hyperlinks using standard file URLs; your operating system should open these in the application
            registered for that file type. However, these do not make use of the line number. In order for the link to
            open the file at the correct line number, you could use a custom URL format such as "file-
            line://{path}:{line}" and register an application to handle the custom "file-line" URL scheme by
            opening the file in your editor/IDE at the indicated line number. See https://github.com/dandavison/open-in-
            editor for an example [default: file://{path}]
        --hunk-header-style <hunk-header-style>
            Style (foreground, background, attributes) for the hunk-header. See STYLES section. Special attributes
            'file' and 'line-number' can be used to include the file path, and number of first hunk line, in the hunk
            header. The style 'omit' can be used to remove the hunk header section from the output [default: line-
            number syntax]
        --hunk-header-file-style <hunk-header-file-style>
            Style (foreground, background, attributes) for the file path part of the hunk-header. See STYLES section.
            The file path will only be displayed if hunk-header-style contains the 'file' special attribute [default:
            blue]
        --hunk-header-line-number-style <hunk-header-line-number-style>
            Style (foreground, background, attributes) for the line number part of the hunk-header. See STYLES section.
            The line number will only be displayed if hunk-header-style contains the 'line-number' special attribute
            [default: blue]
        --hunk-header-decoration-style <hunk-header-decoration-style>
            Style (foreground, background, attributes) for the hunk-header decoration. See STYLES section. The style
            string should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the
            combination 'ul ol' [default: blue box]
        --merge-conflict-begin-symbol <merge-conflict-begin-symbol>
            A string that is repeated to form the line marking the beginning of a merge conflict region [default:
            ▼]
        --merge-conflict-end-symbol <merge-conflict-end-symbol>
            A string that is repeated to form the line marking the end of a merge conflict region [default: ▲]

        --merge-conflict-ours-diff-header-style <merge-conflict-ours-diff-header-style>
            Style (foreground, background, attributes) for the header above the diff between the ancestral commit and
            'our' branch. See STYLES section [default: normal]
        --merge-conflict-ours-diff-header-decoration-style <merge-conflict-ours-diff-header-decoration-style>
            Style (foreground, background, attributes) for the decoration of the header above the diff between the
            ancestral commit and 'our' branch.  See STYLES section. The style string should contain one of the special
            attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol' [default: box]
        --merge-conflict-theirs-diff-header-style <merge-conflict-theirs-diff-header-style>
            Style (foreground, background, attributes) for the header above the diff between the ancestral commit and
            'their' branch. See STYLES section [default: normal]
        --merge-conflict-theirs-diff-header-decoration-style <merge-conflict-theirs-diff-header-decoration-style>
            Style (foreground, background, attributes) for the decoration of the header above the diff between the
            ancestral commit and 'their' branch.  See STYLES section. The style string should contain one of the special
            attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol' [default: box]
        --map-styles <map-styles>
            A string specifying a mapping styles encountered in raw input to desired output styles. An example is --map-
            styles='bold purple => red "#eeeeee", bold cyan => syntax "#eeeeee"'
        --blame-format <blame-format>
            Format string for git blame commit metadata. Available placeholders are "{timestamp}", "{author}", and
            "{commit}" [default: {timestamp:<15} {author:<15.14} {commit:<8} │ ]
        --blame-palette <blame-palette>
            Background colors used for git blame lines (space-separated string). Lines added by the same commit are
            painted with the same color; colors are recycled as needed
        --blame-timestamp-format <blame-timestamp-format>
            Format of `git blame` timestamp in raw git output received by delta [default: %Y-%m-%d %H:%M:%S %z]

        --grep-match-line-style <grep-match-line-style>
            Style (foreground, background, attributes) for matching lines of code in grep output. See STYLES section.
            Defaults to plus-style
        --grep-match-word-style <grep-match-word-style>
            Style (foreground, background, attributes) for the specific matching substrings within a matching line of
            code in grep output. See STYLES section. Defaults to plus-style
        --grep-context-line-style <grep-context-line-style>
            Style (foreground, background, attributes) for non-matching lines of code in grep output. See STYLES
            section. Defaults to zero-style
        --grep-file-style <grep-file-style>
            Style (foreground, background, attributes) for file paths in grep output. See STYLES section. Defaults to
            hunk-header-file-path-style
        --grep-line-number-style <grep-line-number-style>
            Style (foreground, background, attributes) for line numbers in grep output. See STYLES section. Defaults to
            hunk-header-line-number-style
        --grep-separator-symbol <grep-separator-symbol>
            Symbol used in grep output to separate file path (and line number) from the line of file contents. Defaults
            to ":" for both match and context lines, since many terminal emulators recognize constructs like
            "/path/to/file:7:". However, standard grep output uses "-" for context lines: set this option to "keep" to
            keep the original separator symbols [default: :]
        --default-language <default-language>
            Default language used for syntax highlighting when this cannot be inferred from a filename. It will
            typically make sense to set this in per-repository git config (.git/config)
        --inline-hint-style <inline-hint-style>
            Style (foreground, background, attributes) for content added by delta to the original diff such as special
            characters to highlight tabs, and the symbols used to indicate wrapped lines. See STYLES section [default:
            blue]
        --word-diff-regex <tokenization-regex>
            The regular expression used to decide what a word is for the within-line highlight algorithm. For less fine-
            grained matching than the default try --word-diff-regex="\S+" --max-line-distance=1.0 (this is more
            similar to `git --word-diff`) [default: \w+]
        --max-line-distance <max-line-distance>
            The maximum distance between two lines for them to be inferred to be homologous. Homologous line pairs are
            highlighted according to the deletion and insertion operations transforming one into the other [default: 0.6]
        --line-numbers-minus-style <line-numbers-minus-style>
            Style (foreground, background, attributes) for line numbers in the old (minus) version of the file. See
            STYLES and LINE NUMBERS sections [default: auto]
        --line-numbers-zero-style <line-numbers-zero-style>
            Style (foreground, background, attributes) for line numbers in unchanged (zero) lines. See STYLES and LINE
            NUMBERS sections [default: auto]
        --line-numbers-plus-style <line-numbers-plus-style>
            Style (foreground, background, attributes) for line numbers in the new (plus) version of the file. See
            STYLES and LINE NUMBERS sections [default: auto]
        --line-numbers-left-format <line-numbers-left-format>
            Format string for the left column of line numbers. A typical value would be "{nm:^4}⋮" which means to
            display the line numbers of the minus file (old version), center-aligned, padded to a width of 4 characters,
            followed by a dividing character. See the LINE NUMBERS section [default: {nm:^4}⋮]
        --line-numbers-right-format <line-numbers-right-format>
            Format string for the right column of line numbers. A typical value would be "{np:^4}│ " which means to
            display the line numbers of the plus file (new version), center-aligned, padded to a width of 4 characters,
            followed by a dividing character, and a space. See the LINE NUMBERS section [default: {np:^4}│]
        --line-numbers-left-style <line-numbers-left-style>
            Style (foreground, background, attributes) for the left column of line numbers. See STYLES and LINE NUMBERS
            sections [default: auto]
        --line-numbers-right-style <line-numbers-right-style>
            Style (foreground, background, attributes) for the right column of line numbers. See STYLES and LINE NUMBERS
            sections [default: auto]
        --wrap-max-lines <wrap-max-lines>
            How often a line should be wrapped if it does not fit. Zero means to never wrap. Any content which does not
            fit will be truncated. A value of "unlimited" means a line will be wrapped as many times as required
            [default: 2]
        --wrap-left-symbol <wrap-left-symbol>
            Symbol added to the end of a line indicating that the content has been wrapped onto the next line and
            continues left-aligned [default: ↵]
        --wrap-right-symbol <wrap-right-symbol>
            Symbol added to the end of a line indicating that the content has been wrapped onto the next line and
            continues right-aligned [default: ↴]
        --wrap-right-percent <wrap-right-percent>
            Threshold for right-aligning wrapped content. If the length of the remaining wrapped content, as a
            percentage of width, is less than this quantity it will be right-aligned. Otherwise it will be left-aligned
            [default: 37.0]
        --wrap-right-prefix-symbol <wrap-right-prefix-symbol>
            Symbol displayed in front of right-aligned wrapped content [default: …]

        --navigate-regex <navigate-regex>
            A regexp to use in the less pager when navigating (auto-generated when unspecified)

        --file-modified-label <file-modified-label>
            Text to display in front of a modified file path [default: ]

        --file-removed-label <file-removed-label>
            Text to display in front of a removed file path [default: removed:]

        --file-added-label <file-added-label>
            Text to display in front of a added file path [default: added:]

        --file-copied-label <file-copied-label>
            Text to display in front of a copied file path [default: copied:]

        --file-renamed-label <file-renamed-label>
            Text to display in front of a renamed file path [default: renamed:]

        --right-arrow <right-arrow>
            Text to display with a changed value such as a diff heading, a rename, or a chmod [default: ⟶  ]

        --hunk-label <hunk-label>
            Text to display in front of a hunk header [default: ]

        --max-line-length <max-line-length>
            Truncate lines longer than this. To prevent any truncation, set to zero. Note that delta will be slow on
            very long lines (e.g. minified .js) if truncation is disabled. When wrapping lines it is automatically set
            to fit at least all visible characters [default: 512]
        --line-fill-method <line-fill-method>
            How to extend the background color to the end of the line in side-by-side mode. Can be ansi (default) or
            spaces (default if output is not to a terminal). Has no effect if --width=variable is given
    -w, --width <width>
            The width of underline/overline decorations. Examples: "72" (exactly 72 characters), "-2" (auto-detected
            terminal width minus 2). An expression such as "74-2" is also valid (equivalent to 72 but may be useful if
            the caller has a variable holding the value "74"). Use --width=variable to extend decorations and background
            colors to the end of the text only. Otherwise background colors extend to the full terminal width
        --diff-stat-align-width <diff-stat-align-width>
            Width allocated for file paths in a diff stat section. If a relativized file path exceeds this width then
            the diff stat will be misaligned [default: 48]
        --tabs <tab-width>
            The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters through directly,
            but note that in that case delta will calculate line widths assuming tabs occupy one character's width on
            the screen: if your terminal renders tabs as more than than one character wide then delta's output will look
            incorrect [default: 4]
        --true-color <true-color>
            Whether to emit 24-bit ("true color") RGB color codes. Options are auto, always, and never. "auto" means
            that delta will emit 24-bit color codes if the environment variable COLORTERM has the value "truecolor" or
            "24bit". If your terminal application (the application you use to enter commands at a shell prompt) supports
            24 bit colors, then it probably already sets this environment variable, in which case you don't need to do
            anything [default: auto]
        --24-bit-color <24-bit-color>
            Deprecated: use --true-color

        --inspect-raw-lines <inspect-raw-lines>
            Whether to examine ANSI color escape sequences in raw lines received from Git and handle lines colored in
            certain ways specially. This is on by default: it is how Delta supports Git's --color-moved feature. Set
            this to "false" to disable this behavior [default: true]
        --pager <pager>
            Which pager to use. The default pager is `less`. You can also change pager by setting the environment
            variables DELTA_PAGER, BAT_PAGER, or PAGER (and that is their order of priority). This option overrides all
            environment variables above
        --paging <paging-mode>
            Whether to use a pager when displaying output. Options are: auto, always, and never [default: auto]
        --minus-empty-line-marker-style <minus-empty-line-marker-style>
            Style for removed empty line marker (used only if --minus-style has no background color) [default: normal auto]
        --plus-empty-line-marker-style <plus-empty-line-marker-style>
            Style for added empty line marker (used only if --plus-style has no background color) [default: normal auto]
        --whitespace-error-style <whitespace-error-style>
            Style for whitespace errors. Defaults to color.diff.whitespace if that is set in git config, or else
            'magenta reverse' [default: auto auto]
        --line-buffer-size <line-buffer-size>
            Size of internal line buffer. Delta compares the added and removed versions of nearby lines in order to
            detect and highlight changes at the level of individual words/tokens. Therefore, nearby lines must be
            buffered internally before they are painted and emitted. Increasing this value might improve highlighting of
            some large diff hunks. However, setting this to a high value will adversely affect delta's performance when
            entire files are added/removed [default: 32]
        --minus-color <deprecated-minus-background-color>
            Deprecated: use --minus-style='normal my_background_color'

        --minus-emph-color <deprecated-minus-emph-background-color>
            Deprecated: use --minus-emph-style='normal my_background_color'

        --plus-color <deprecated-plus-background-color>
            Deprecated: Use --plus-style='syntax my_background_color' to change the background color while retaining
            syntax-highlighting
        --plus-emph-color <deprecated-plus-emph-background-color>
            Deprecated: Use --plus-emph-style='syntax my_background_color' to change the background color while
            retaining syntax-highlighting
        --commit-color <deprecated-commit-color>
            Deprecated: use --commit-style='my_foreground_color' --commit-decoration-style='my_foreground_color'

        --file-color <deprecated-file-color>
            Deprecated: use --file-style='my_foreground_color' --file-decoration-style='my_foreground_color'

        --hunk-style <deprecated-hunk-style>
            Deprecated: synonym of --hunk-header-decoration-style

        --hunk-color <deprecated-hunk-color>
            Deprecated: use --hunk-header-style='my_foreground_color' --hunk-header-decoration-
            style='my_foreground_color'
        --theme <deprecated-theme>
            Deprecated: use --syntax-theme


ARGS:
    <minus-file>    First file to be compared when delta is being used in diff mode: `delta file_1 file_2` is
                    equivalent to `diff -u file_1 file_2 | delta`
    <plus-file>     Second file to be compared when delta is being used in diff mode

GIT CONFIG
----------

By default, delta takes settings from a section named "delta" in git config files, if one is
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

[delta "my-delta-feature"]
    syntax-theme = Dracula
    plus-style = bold syntax "#002800"

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

--minus-style 'red bold ul "#ffeeee"'

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

There are four ways to specify a color (this section applies to foreground and background colors
within a style string):

1. CSS color name

   Any of the 140 color names used in CSS: https://www.w3schools.com/colors/colors_groups.asp

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

   Unlike RGB hex codes, ANSI color names are just names: you can choose the exact color that each
   name corresponds to in the settings of your terminal application (the application you use to
   enter commands at a shell prompt). This means that if you use ANSI color names, and you change
   the color theme used by your terminal, then delta's colors will respond automatically, without
   needing to change the delta command line.

   "purple" is accepted as a synonym for "magenta". Color names and codes are case-insensitive.

4. ANSI color number

   An example of using an ANSI color number is:
   --file-style=28

   There are 256 ANSI color numbers: 0-255. The first 16 are the same as the colors described in
   the "ANSI color name" section above. See https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit.
   Specifying colors like this is useful if your terminal only supports 256 colors (i.e. doesn't
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
```
