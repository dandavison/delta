# Features

- Language syntax highlighting with color themes
- Within-line highlights based on a Levenshtein edit inference algorithm
- Side-by-side view with line-wrapping
- Line numbering
- `n` and `N` keybindings to move between files in large diffs, and between diffs in `log -p` views (`--navigate`)
- Improved merge conflict display
- Improved `git blame` display (syntax highlighting; `--hyperlinks` formats commits as links to GitHub/GitLab/Bitbucket etc)
- Syntax-highlights grep output from `rg`, `git grep`, `grep`, etc
- Support for Git's `--color-moved` feature.
- Code can be copied directly from the diff (`-/+` markers are removed by default).
- `diff-highlight` and `diff-so-fancy` emulation modes
- Commit hashes can be formatted as terminal [hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda) to the GitHub/GitLab/Bitbucket page (`--hyperlinks`).
  File paths can also be formatted as hyperlinks for opening in your OS.
- Stylable box/line decorations to draw attention to commit, file and hunk header sections.
- Git style strings (foreground color, background color, font attributes) are supported for >20 stylable elements

|                                                | delta | git | [diff-so-fancy] /<br>[diff-highlight] | github/gitlab |
| ---------------------------------------------- | ----- | --- | ------------------------------------- | ------------- |
| language syntax highlighting                   | ✅    | ❌  | ❌                                    | ✅            |
| within-line insertion/deletion detection       | ✅    | ❌  | ✅                                    | ✅            |
| multiple insertion/deletions detected per line | ✅    | ❌  | ❌                                    | ✅            |
| matching of unequal numbers of changed lines   | ✅    | ❌  | ❌                                    | ❌            |
| independently stylable elements                | ✅    | ✅  | ✅                                    | ❌            |
| line numbering                                 | ✅    | ❌  | ❌                                    | ✅            |
| side-by-side view                              | ✅    | ❌  | ❌                                    | ✅            |

In addition, delta handles traditional unified diff output.

[diff-so-fancy]: https://github.com/so-fancy/diff-so-fancy
[diff-highlight]: https://github.com/git/git/tree/master/contrib/diff-highlight
