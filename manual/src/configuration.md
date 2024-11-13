# Configuration

## Git config file

The most convenient way to configure delta is with a `[delta]` section in `~/.gitconfig`. Here's an example:

<sub>

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    navigate = true  # use n and N to move between diff sections
    dark = true      # or light = true, or omit for auto-detection

[merge]
    conflictstyle = zdiff3
```

</sub>

Use `delta --help` to see all the available options.

Note that delta style argument values in ~/.gitconfig should be in double quotes, like `--minus-style="syntax #340001"`. For theme names and other values, do not use quotes as they will be passed on to delta, like `theme = Monokai Extended`.

All git commands that display diff output should now display syntax-highlighted output. For example:

- `git diff`
- `git show`
- `git log -p`
- `git stash show -p`
- `git reflog -p`
- `git add -p`

To change your delta options in a one-off git command, use `git -c`. For example

```sh
git -c delta.line-numbers=false show
```

There are several important environment variables that affect delta configuration and which can be used to configure delta dynamically.
Please see [Environment variables](./environment-variables.md).
