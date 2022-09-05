# Configuration

## Git config file

The most convenient way to configure delta is with a `[delta]` section in `~/.gitconfig`. Here's an example:

<sub>

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only --features=interactive

[delta]
    features = decorations

[delta "interactive"]
    keep-plus-minus-markers = false

[delta "decorations"]
    commit-decoration-style = blue ol
    commit-style = raw
    file-style = omit
    hunk-header-decoration-style = blue box
    hunk-header-file-style = red
    hunk-header-line-number-style = "#067a00"
    hunk-header-style = file line-number syntax
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

```
git -c delta.line-numbers=false show
```

There are several important environment variables that affect delta configuration and which can be used to configure delta dynamically.
Please see [Environment variables](./environment-variables.md).
In particular, note that delta does not currently honor all relevant [git environment variables](https://git-scm.com/docs/git-config#_environment), since delta uses [libgit2](https://github.com/libgit2/libgit2) to read git config.
