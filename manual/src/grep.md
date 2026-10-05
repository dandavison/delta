# Grep

Delta applies syntax-highlighting and other enhancements to standard grep output such as from [ripgrep](https://github.com/BurntSushi/ripgrep/) (aka `rg`), `git grep`, grep, etc.
If you don't need special features of `git grep`, then for best results pipe `rg --json` output to delta: this avoids parsing ambiguities that are inevitable with the output of `git grep` and `grep`.
To customize the colors and syntax highlighting, see the `grep-*` options in `delta --help`.

Note that `git grep` can display the "function context" for matches and that delta handles this output specially: see the `-p` and `-W` options of `git grep`.

```sh
rg --json -C 2 handle | delta
```

<table><tr><td>
<img width="600px" alt="image" src="https://github.com/dandavison/open-in-editor/assets/52205/d203d380-5acb-4296-aeb9-e38c73d6c27f">
</td></tr></table>

With `hyperlinks` enabled, the line numbers in the grep output will be clickable links. See [hyperlinks](./hyperlinks.md).

If you add `{column}` to your `hyperlinks-file-link-format`, the line numbers from `rg --json` will be clickable links at the **first** match on a line instead of only on the line itself. For a URL handler that accepts `file://path:line:column`, use `hyperlinks-file-link-format = "file://{path}:{line}:{column}"`. Columns are one-based byte positions, unaffected by Delta's tab expansion. Ripgrep's `--column` option is not needed. Context lines and records without a match column use column `1`. See [linking to a match column](./hyperlinks.md#linking-to-a-match-column) for configuration details.