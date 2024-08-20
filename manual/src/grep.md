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
