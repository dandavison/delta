# Navigation keybindings for large diffs

Use the `navigate` feature to activate navigation keybindings. By default, pressing `n` will jump forward to the next file or hunk header in the diff, and `N` will jump backwards. If you are viewing multiple commits (e.g. via `git log -p`) then navigation will also visit commit headers.

To skip hunk headers while navigating between files and commits, set an empty `hunk-label` in your `~/.gitconfig`:

```gitconfig
[delta]
    navigate = true
    hunk-label = ""
```

This removes the label from hunk headers but leaves the headers visible. If you have set a custom `navigate-regex`, that regular expression determines the navigation stops instead.
