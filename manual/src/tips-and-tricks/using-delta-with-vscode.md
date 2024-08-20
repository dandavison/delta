# Using Delta with VSCode

All Delta features work correctly in VSCode's terminal emulator (please open an issue if that's not true).

To format file links for opening in VSCode from other terminal emulators, use the [VSCode URL handler](https://code.visualstudio.com/docs/editor/command-line#_opening-vs-code-with-urls):

```gitconfig
[delta]
   hyperlinks = true
   hyperlinks-file-link-format = "vscode://file/{path}:{line}"
```

(To use VSCode Insiders, change that to `vscode-insiders://file/{path}:{line}`).

 See [hyperlinks](./hyperlinks.md).
