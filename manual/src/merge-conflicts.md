# Merge conflicts

Consider setting [`merge.conflictStyle`](https://git-scm.com/docs/git-config#Documentation/git-config.txt-mergeconflictStyle) to `zdiff3`:

```gitconfig
[merge]
    conflictStyle = zdiff3
```

With that setting, when a merge conflict is encountered, Git will display merge conflicts with the contents of the merge base as well.
delta will then display this as two diffs, from the ancestor to each side of the conflict:

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/144783121-bb549100-69d8-41b8-ac62-1704f1f7b43e.png" alt="image" /></td></tr></table>

This display can be customized using `merge-conflict-begin-symbol`, `merge-conflict-end-symbol`, `merge-conflict-ours-diff-header-style`, `merge-conflict-ours-diff-header-decoration-style`, `merge-conflict-theirs-diff-header-style`, `merge-conflict-theirs-diff-header-decoration-style`.
