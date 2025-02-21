# Configuration

## Git config file

Delta uses [git config](https://git-scm.com/docs/git-config#_configuration_file) (`~/.gitconfig`) for its configuration. Here's an example:

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

You do not even need to use git -- delta accepts `git diff` and unified diff formats and hence works with e.g. mercurial and jujutsu -- but you do need to use the git config format.

If you want to store your delta config at a different location, use [[git docs](https://git-scm.com/docs/git-config#Documentation/git-config.txt-GITCONFIGGLOBAL)]
```bash
export GIT_CONFIG_GLOBAL=/path/to/my/delta/config
```

If you want to keep your delta and git config separate, use [[git docs](https://git-scm.com/docs/git-config#_includes)]
```gitconfig
[include]
    path = ~/src/devenv/dotfiles/delta/delta.gitconfig
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
