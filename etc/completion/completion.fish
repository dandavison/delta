complete -c delta -l blame-code-style -d 'Style string for the code section of a git blame line' -r
complete -c delta -l blame-format -d 'Format string for git blame commit metadata' -r
complete -c delta -l blame-palette -d 'Background colors used for git blame lines (space-separated string)' -r
complete -c delta -l blame-separator-format -d 'Separator between the blame format and the code section of a git blame line' -r
complete -c delta -l blame-separator-style -d 'Style string for the blame-separator-format' -r
complete -c delta -l blame-timestamp-format -d 'Format of `git blame` timestamp in raw git output received by delta' -r
complete -c delta -l blame-timestamp-output-format -d 'Format string for git blame timestamp output' -r
complete -c delta -l config -d 'Load the config file at PATH instead of ~/.gitconfig' -r -F
complete -c delta -l commit-decoration-style -d 'Style string for the commit hash decoration' -r
complete -c delta -l commit-regex -d 'Regular expression used to identify the commit line when parsing git output' -r
complete -c delta -l commit-style -d 'Style string for the commit hash line' -r
complete -c delta -l default-language -d 'Default language used for syntax highlighting' -r
complete -c delta -l diff-stat-align-width -d 'Width allocated for file paths in a diff stat section' -r
complete -c delta -l features -d 'Names of delta features to activate (space-separated)' -r
complete -c delta -l file-added-label -d 'Text to display before an added file path' -r
complete -c delta -l file-copied-label -d 'Text to display before a copied file path' -r
complete -c delta -l file-decoration-style -d 'Style string for the file decoration' -r
complete -c delta -l file-modified-label -d 'Text to display before a modified file path' -r
complete -c delta -l file-removed-label -d 'Text to display before a removed file path' -r
complete -c delta -l file-renamed-label -d 'Text to display before a renamed file path' -r
complete -c delta -l file-style -d 'Style string for the file section' -r
complete -c delta -l file-transformation -d 'Sed-style command transforming file paths for display' -r
complete -c delta -l generate-completion -d 'Print completion file for the given shell' -r -f -a "{bash	'',elvish	'',fish	'',powershell	'',zsh	''}"
complete -c delta -l grep-context-line-style -d 'Style string for non-matching lines of grep output' -r
complete -c delta -l grep-file-style -d 'Style string for file paths in grep output' -r
complete -c delta -l grep-header-decoration-style -d 'Style string for the header decoration in grep output' -r
complete -c delta -l grep-header-file-style -d 'Style string for the file path part of the header in grep output' -r
complete -c delta -l grep-line-number-style -d 'Style string for line numbers in grep output' -r
complete -c delta -l grep-output-type -d 'Grep output format. Possible values: "ripgrep" - file name printed once, followed by matching lines within that file, each preceded by a line number. "classic" - file name:line number, followed by matching line. Default is "ripgrep" if `rg --json` format is detected, otherwise "classic"' -r -f -a "{ripgrep	'',classic	''}"
complete -c delta -l grep-match-line-style -d 'Style string for matching lines of grep output' -r
complete -c delta -l grep-match-word-style -d 'Style string for the matching substrings within a matching line of grep output' -r
complete -c delta -l grep-separator-symbol -d 'Separator symbol printed after the file path and line number in grep output' -r
complete -c delta -l hunk-header-decoration-style -d 'Style string for the hunk-header decoration' -r
complete -c delta -l hunk-header-file-style -d 'Style string for the file path part of the hunk-header' -r
complete -c delta -l hunk-header-line-number-style -d 'Style string for the line number part of the hunk-header' -r
complete -c delta -l hunk-header-style -d 'Style string for the hunk-header' -r
complete -c delta -l hunk-label -d 'Text to display before a hunk header' -r
complete -c delta -l hyperlinks-commit-link-format -d 'Format string for commit hyperlinks (requires --hyperlinks)' -r
complete -c delta -l hyperlinks-file-link-format -d 'Format string for file hyperlinks (requires --hyperlinks)' -r
complete -c delta -l inline-hint-style -d 'Style string for short inline hint text' -r
complete -c delta -l inspect-raw-lines -d 'Kill-switch for --color-moved support' -r -f -a "{true	'',false	''}"
complete -c delta -l line-buffer-size -d 'Size of internal line buffer' -r
complete -c delta -l line-fill-method -d 'Line-fill method in side-by-side mode' -r -f -a "{ansi spaces	''}"
complete -c delta -l line-numbers-left-format -d 'Format string for the left column of line numbers' -r
complete -c delta -l line-numbers-left-style -d 'Style string for the left column of line numbers' -r
complete -c delta -l line-numbers-minus-style -d 'Style string for line numbers in the old (minus) version of the file' -r
complete -c delta -l line-numbers-plus-style -d 'Style string for line numbers in the new (plus) version of the file' -r
complete -c delta -l line-numbers-right-format -d 'Format string for the right column of line numbers' -r
complete -c delta -l line-numbers-right-style -d 'Style string for the right column of line numbers' -r
complete -c delta -l line-numbers-zero-style -d 'Style string for line numbers in unchanged (zero) lines' -r
complete -c delta -l map-styles -d 'Map styles encountered in raw input to desired output styles' -r
complete -c delta -l max-line-distance -d 'Maximum line pair distance parameter in within-line diff algorithm' -r
complete -c delta -l max-line-length -d 'Truncate lines longer than this' -r
complete -c delta -l merge-conflict-begin-symbol -d 'String marking the beginning of a merge conflict region' -r
complete -c delta -l merge-conflict-end-symbol -d 'String marking the end of a merge conflict region' -r
complete -c delta -l merge-conflict-ours-diff-header-decoration-style -d 'Style string for the decoration of the header above the \'ours\' merge conflict diff' -r
complete -c delta -l merge-conflict-ours-diff-header-style -d 'Style string for the header above the \'ours\' branch merge conflict diff' -r
complete -c delta -l merge-conflict-theirs-diff-header-decoration-style -d 'Style string for the decoration of the header above the \'theirs\' merge conflict diff' -r
complete -c delta -l merge-conflict-theirs-diff-header-style -d 'Style string for the header above the \'theirs\' branch merge conflict diff' -r
complete -c delta -l minus-empty-line-marker-style -d 'Style string for removed empty line marker' -r
complete -c delta -l minus-emph-style -d 'Style string for emphasized sections of removed lines' -r
complete -c delta -l minus-non-emph-style -d 'Style string for non-emphasized sections of removed lines that have an emphasized section' -r
complete -c delta -l minus-style -d 'Style string for removed lines' -r
complete -c delta -l navigate-regex -d 'Regular expression defining navigation stop points' -r
complete -c delta -l pager -d 'Which pager to use' -r
complete -c delta -l paging -d 'Whether to use a pager when displaying output' -r -f -a "{auto	'',always	'',never	''}"
complete -c delta -l plus-emph-style -d 'Style string for emphasized sections of added lines' -r
complete -c delta -l plus-empty-line-marker-style -d 'Style string for added empty line marker' -r
complete -c delta -l plus-non-emph-style -d 'Style string for non-emphasized sections of added lines that have an emphasized section' -r
complete -c delta -l plus-style -d 'Style string for added lines' -r
complete -c delta -l right-arrow -d 'Text to display with a changed file path' -r
complete -c delta -l syntax-theme -d 'The syntax-highlighting theme to use' -r
complete -c delta -l tabs -d 'The number of spaces to replace tab characters with' -r
complete -c delta -l true-color -d 'Whether to emit 24-bit ("true color") RGB color codes' -r -f -a "{auto	'',always	'',never	''}"
complete -c delta -l whitespace-error-style -d 'Style string for whitespace errors' -r
complete -c delta -s w -l width -d 'The width of underline/overline decorations' -r
complete -c delta -l word-diff-regex -d 'Regular expression defining a \'word\' in within-line diff algorithm' -r
complete -c delta -l wrap-left-symbol -d 'End-of-line wrapped content symbol (left-aligned)' -r
complete -c delta -l wrap-max-lines -d 'How often a line should be wrapped if it does not fit' -r
complete -c delta -l wrap-right-percent -d 'Threshold for right-aligning wrapped content' -r
complete -c delta -l wrap-right-prefix-symbol -d 'Pre-wrapped content symbol (right-aligned)' -r
complete -c delta -l wrap-right-symbol -d 'End-of-line wrapped content symbol (right-aligned)' -r
complete -c delta -l zero-style -d 'Style string for unchanged lines' -r
complete -c delta -l 24-bit-color -d 'Deprecated: use --true-color' -r -f -a "{auto	'',always	'',never	''}"
complete -c delta -l color-only -d 'Do not alter the input structurally in any way'
complete -c delta -l dark -d 'Use default colors appropriate for a dark terminal background'
complete -c delta -l diff-highlight -d 'Emulate diff-highlight'
complete -c delta -l diff-so-fancy -d 'Emulate diff-so-fancy'
complete -c delta -l hyperlinks -d 'Render commit hashes, file names, and line numbers as hyperlinks'
complete -c delta -l keep-plus-minus-markers -d 'Prefix added/removed lines with a +/- character, as git does'
complete -c delta -l light -d 'Use default colors appropriate for a light terminal background'
complete -c delta -s n -l line-numbers -d 'Display line numbers next to the diff'
complete -c delta -l list-languages -d 'List supported languages and associated file extensions'
complete -c delta -l list-syntax-themes -d 'List available syntax-highlighting color themes'
complete -c delta -l navigate -d 'Activate diff navigation'
complete -c delta -l no-gitconfig -d 'Do not read any settings from git config'
complete -c delta -l parse-ansi -d 'Display ANSI color escape sequences in human-readable form'
complete -c delta -l raw -d 'Do not alter the input in any way'
complete -c delta -l relative-paths -d 'Output all file paths relative to the current directory'
complete -c delta -l show-colors -d 'Show available named colors'
complete -c delta -l show-config -d 'Display the active values for all Delta options'
complete -c delta -l show-syntax-themes -d 'Show example diff for available syntax-highlighting themes'
complete -c delta -l show-themes -d 'Show example diff for available delta themes'
complete -c delta -s s -l side-by-side -d 'Display diffs in side-by-side layout'
complete -c delta -s h -l help -d 'Print help (see more with \'--help\')'
complete -c delta -s V -l version -d 'Print version'
