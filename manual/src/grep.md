# Grep

Delta applies syntax-highlighting and other enhancements to standard grep output such as from `git grep`, [ripgrep](https://github.com/BurntSushi/ripgrep/) (aka `rg`), grep, etc.
To use with `git grep`, set delta as the pager for `grep` in the `[pager]` section of your gitconfig. See the example at the [top of the page](./get-started.md).
Output from other grep tools can be piped to delta: e.g. `rg -Hn --color=always`, `grep -Hn --color=always`, etc.
To customize the colors and syntax highlighting, see `grep-match-line-style`, `grep-match-word-style`, `grep-context-line-style`, `grep-file-style`, `grep-line-number-style`.
Ripgrep's `rg --json` output format is supported; this avoids certain file name parsing ambiguities that are inevitable with the standard grep output formats.
Note that `git grep` can display the "function context" for matches and that delta handles this output specially: see the `-p` and `-W` options of `git grep`.
