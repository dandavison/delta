<p align="center">
  <img width=400px src="https://user-images.githubusercontent.com/52205/147996902-9829bd3f-cd33-466e-833e-49a6f3ebd623.png" alt="image" />
</p>
<p align="center">
  <a href="https://github.com/dandavison/delta/actions">
    <img src="https://github.com/dandavison/delta/workflows/Continuous%20Integration/badge.svg" alt="CI">
  </a>
  <a href="https://coveralls.io/github/dandavison/delta?branch=master">
    <img src="https://coveralls.io/repos/github/dandavison/delta/badge.svg?branch=master" alt="Coverage Status">
  </a>
  <a href="https://gitter.im/dandavison-delta/community?utm_source=badge&amp;utm_medium=badge&amp;utm_campaign=pr-badge">
    <img src="https://badges.gitter.im/dandavison-delta/community.svg" alt="Gitter">
  </a>
</p>

## Get Started

[Install it](https://dandavison.github.io/delta/installation.html) (the package is called "git-delta" in most package managers, but the executable is just `delta`) and add this to your `~/.gitconfig`:

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    navigate = true    # use n and N to move between diff sections
    light = false      # set to true if you're in a terminal w/ a light background color (e.g. the default macOS terminal)

[merge]
    conflictstyle = diff3

[diff]
    colorMoved = default
```

Delta has many features and is very customizable; please see the [user manual](https://dandavison.github.io/delta/).

## Features

- Language syntax highlighting with the same syntax-highlighting themes as [bat](https://github.com/sharkdp/bat#readme)
- Word-level diff highlighting using a Levenshtein edit inference algorithm
- Side-by-side view with line-wrapping
- Line numbering
- `n` and `N` keybindings to move between files in large diffs, and between diffs in `log -p` views (`--navigate`)
- Improved merge conflict display
- Improved `git blame` display (syntax highlighting; `--hyperlinks` formats commits as links to hosting provider etc. Supported hosting providers are: GitHub, GitLab, SourceHut, Codeberg)
- Syntax-highlights grep output from `rg`, `git grep`, `grep`, etc
- Support for Git's `--color-moved` feature.
- Code can be copied directly from the diff (`-/+` markers are removed by default).
- `diff-highlight` and `diff-so-fancy` emulation modes
- Commit hashes can be formatted as terminal [hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda) to the hosting provider page (`--hyperlinks`).
  File paths can also be formatted as hyperlinks for opening in your OS.
- Stylable box/line decorations to draw attention to commit, file and hunk header sections.
- Style strings (foreground color, background color, font attributes) are supported for >20 stylable elements, using the same color/style language as git
- Handles traditional unified diff output in addition to git output

## A syntax-highlighting pager for git, diff, and grep output

Code evolves, and we all spend time studying diffs. Delta aims to make this both efficient and enjoyable: it allows you to make extensive changes to the layout and styling of diffs, as well as allowing you to stay arbitrarily close to the default git/diff output.

<table>
  <tr>
    <td>
      <img width=400px src="https://user-images.githubusercontent.com/52205/86275526-76792100-bba1-11ea-9e78-6be9baa80b29.png" alt="image" />
      <br>
      <sub>delta with <code>line-numbers</code> activated</sub>
    </td>
  </tr>
</table>

<table>
  <tr>
    <td>
      <img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" />
      <br>
      <sub>delta with <code>side-by-side</code> and <code>line-numbers</code> activated</sub>
    </td>
  </tr>
</table>

Here's what `git show` can look like with git configured to use delta:

<br>

<table>
  <tr>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/81058545-a5725f80-8e9c-11ea-912e-d21954586a44.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/81058911-6abcf700-8e9d-11ea-93be-e212824ec03d.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      "Dracula" theme
    </td>
    <td>
      "GitHub" theme
    </td>
  </tr>
</table>

<br>
<br>

### Syntax-highlighting themes

**All the syntax-highlighting color themes that are available with [bat](https://github.com/sharkdp/bat/) are available with delta:**

<br>
<table>
  <tr>
    <td>
      <img width=400px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/149431273-e3ad049d-771e-4186-869d-0e57967958a6.png"
           alt="image" />
    </td>
    <td>
      <img width=400px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/149431419-48836001-2afc-4fd0-97ad-561a69b71db7.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      <code>delta --show-syntax-themes --dark</code>
    </td>
    <td>
      <code>delta --show-syntax-themes --light</code>
    </td>
  </tr>
</table>

<br>

### Side-by-side view

[[User manual](https://dandavison.github.io/delta/side-by-side-view.html)]

```gitconfig
[delta]
    side-by-side = true
```

By default, side-by-side view has line-numbers activated, and has syntax highlighting in both the left and right panels: [[config](#side-by-side-view-1)]

<table><tr><td><img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" /></td></tr></table>

Side-by-side view wraps long lines automatically:

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/139064537-f8479504-16d3-429a-b4f6-d0122438adaa.png" alt="image" /></td></tr></table>

### Line numbers

[[User manual](https://dandavison.github.io/delta/line-numbers.html)]

```gitconfig
[delta]
    line-numbers = true
```

<table><tr><td><img width=400px src="https://user-images.githubusercontent.com/52205/86275526-76792100-bba1-11ea-9e78-6be9baa80b29.png" alt="image" /></td></tr></table>

### Merge conflicts

[[User manual](https://dandavison.github.io/delta/merge-conflicts.html)]

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/144783121-bb549100-69d8-41b8-ac62-1704f1f7b43e.png" alt="image" /></td></tr></table>

### Git blame

[[User manual](https://dandavison.github.io/delta/git-blame.html)]

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/141891376-1fdb87dc-1d9c-4ad6-9d72-eeb19a8aeb0b.png" alt="image" /></td></tr></table>

### Ripgrep, git grep

[[User manual](https://dandavison.github.io/delta/grep.html)]

<table><tr><td>
<img width="600px" alt="image" src="https://github.com/dandavison/open-in-editor/assets/52205/d203d380-5acb-4296-aeb9-e38c73d6c27f">
</td></tr></table>

### Installation and usage

Please see the [user manual](https://dandavison.github.io/delta/) and `delta --help`.
