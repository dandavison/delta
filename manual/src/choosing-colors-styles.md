# Choosing colors (styles)

All options that have a name like `--*-style` work in the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
https://git-scm.com/docs/git-config#Documentation/git-config.txt-color

Here's an example:

```gitconfig
[delta]
    minus-style = red bold ul "#ffeeee"
```

That means: For removed lines, set the foreground (text) color to 'red', make it bold and underlined, and set the background color to `#ffeeee`.

For full details, see the `STYLES` section in [`delta --help`](./full---help-output.md).
