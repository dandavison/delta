# Get started

[Install](./installation.md) delta and add this to your `~/.gitconfig`:

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    navigate = true

[merge]
    conflictstyle = diff3

[diff]
    colorMoved = default

```
