# Side-by-side view

```gitconfig
[delta]
    side-by-side = true
```

By default, side-by-side view has line-numbers activated, and has syntax highlighting in both the left and right panels: [[config](./side-by-side-view-1.md)]

<table><tr><td><img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" /></td></tr></table>

To disable the line numbers in side-by-side view, but keep a vertical delimiter line between the left and right panels, use the line-numbers format options. For example:

```gitconfig
[delta]
    side-by-side = true
    line-numbers-left-format = ""
    line-numbers-right-format = "│ "
```

Long lines are wrapped if they do not fit in side-by-side mode.
In the image below, the long deleted line in the left panel overflows by a small amount, and the wrapped content is right-aligned in the next line.
In contrast, the long replacement line in the right panel overflows by almost an entire line, and so the wrapped content is left aligned in the next line. The arrow markers and ellipsis explain when and how text has been wrapped.

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/139064537-f8479504-16d3-429a-b4f6-d0122438adaa.png" alt="image" /></td></tr></table>

For control over the details of line wrapping, see `--wrap-max-lines`, `--wrap-left-symbol`, `--wrap-right-symbol`, `--wrap-right-percent`, `--wrap-right-prefix-symbol`, `--inline-hint-style`.
Line wrapping was implemented by @th1000s.
