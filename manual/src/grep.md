# Grep

Delta applies syntax-highlighting and other enhancements to standard grep output such as from [ripgrep](https://github.com/BurntSushi/ripgrep/) (aka `rg`), `git grep`, grep, etc.
If you don't need special features of `git grep`, then for best results pipe `rg --json` output to delta: this avoids parsing ambiguities that are inevitable with the output of `git grep` and `grep`.
To customize the colors and syntax highlighting, see `grep-match-line-style`, `grep-match-word-style`, `grep-context-line-style`, `grep-file-style`, `grep-line-number-style`.
Note that `git grep` can display the "function context" for matches and that delta handles this output specially: see the `-p` and `-W` options of `git grep`.

```sh
rg --json handle | delta
```

<table><tr><td><img width=400px src="https://user-images.githubusercontent.com/52205/225271024-a01367f0-af1b-466a-9b9e-b1ced7f80031.png" alt="image" /></td></tr></table>

If you enable hyperlinks then grep hits will be formatted as [OSC8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda) in terminal emulators that support the feature. It is then possible to make your OS handle a click on those links by opening your editor at the correct file and line number (e.g. via <https://github.com/dandavison/open-in-editor/>).

```gitconfig
[delta]
   hyperlinks = true
   hyperlinks-file-link-format = "file-line://{path}:{line}"
```
