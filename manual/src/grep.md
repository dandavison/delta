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

If you enable hyperlinks then grep hits will be formatted as [OSC8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda) in terminal emulators that support the feature. If you're using VSCode, IntelliJ, or PyCharm, then use the dedicated URL handlers. I.e. one of the following lines:

```gitconfig
[delta]
    hyperlinks = true
    hyperlinks-file-link-format = "vscode://file/{path}:{line}"
    # or: hyperlinks-file-link-format = "idea://open?file={path}&line={line}"
    # or: hyperlinks-file-link-format = "pycharm://open?file={path}&line={line}"
```

For editors that don't have special URL handlers, it is possible to use a tool like <https://github.com/dandavison/open-in-editor/> to make your OS handle a click on those links by opening your editor at the correct file and line number, e.g.

```gitconfig
[delta]
   hyperlinks = true
   hyperlinks-file-link-format = "file-line://{path}:{line}"
   # Now configure your OS to handle "file-line" URLs
```
