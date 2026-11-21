# Git's diff colors overriding delta's colors

If added and removed lines are shown in plain colored text, without delta's background colors, when delta runs as git's pager (e.g. `git show`), but look fine when you pipe into delta yourself (e.g. `git show | delta`), check your gitconfig for a `[color "diff"]` section like this:

```gitconfig
[color "diff"]
    old = brightred
    new = brightgreen
```

When git sends its output to a pager it colors that output first, using these settings. Delta normally replaces git's colors with its own styles. But if a removed or added line is not in git's default colors, delta treats it as deliberately highlighted (this is how it handles [`--color-moved`](../color-moved-support.md)) and keeps git's colors for it. Piped output (`git show | delta`) is not colored by git, so delta's styles are used.

To fix this, remove the `old` and `new` entries (or the whole `[color "diff"]` section) from your gitconfig. You can list the ones that are set with:

```sh
git config --get-regexp '^color\.diff\.'
```
