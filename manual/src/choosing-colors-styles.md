# Choosing colors (styles)

Delta detects your terminal background color automatically and chooses appropriate default colors.
To override automatic detection use `dark` or `light`, e.g.

```gitconfig
[delta]
    dark = true
```
This is necessary when running delta in some contexts such as `lazygit` or `zellij`.

Automatic detection works by querying the terminal, which needs an interactive terminal and so
fails when delta's output is piped, for example through a pager TUI such as
[diffnav](https://github.com/dlvhdr/diffnav) — in that case delta falls back to dark. Set
`detect-dark-light` to `system-global` to use the OS-wide light/dark appearance instead, which
needs no terminal query and works when piped. `detect-dark-light` is honored from git config, so
a pager that passes delta no flags still picks it up:

```gitconfig
[delta]
    detect-dark-light = system-global
```

This assumes the terminal follows the OS appearance. It is supported on macOS, Windows, Linux and
the BSDs (the latter two via the XDG desktop portal). If the OS reports no preference, delta falls
back to querying the terminal when it can.

All options that have a name like `--*-style` work in the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
<https://git-scm.com/docs/git-config#Documentation/git-config.txt-color>

Here's an example:

```gitconfig
[delta]
    minus-style = red bold ul "#ffeeee"
```

That means: For removed lines, set the foreground (text) color to 'red', make it bold and underlined, and set the background color to `#ffeeee`.

For full details, see the `STYLES` section in [`delta --help`](./full---help-output.md).
