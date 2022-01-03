# Merge conflicts

Consider setting

```gitconfig
[merge]
    conflictstyle = diff3
```

With that setting, when a merge conflict is encountered, delta will display diffs between the ancestral commit and each of the two merge parents:

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/144783121-bb549100-69d8-41b8-ac62-1704f1f7b43e.png" alt="image" /></td></tr></table>

This display can be customized using `merge-conflict-begin-symbol`, `merge-conflict-end-symbol`, `merge-conflict-ours-diff-header-style`, `merge-conflict-ours-diff-header-decoration-style`, `merge-conflict-theirs-diff-header-style`, `merge-conflict-theirs-diff-header-decoration-style`.
