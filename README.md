<p align="center">
  <img width=400px src="https://user-images.githubusercontent.com/52205/102416950-ae124300-3ff2-11eb-9d66-40d2aef4888f.png" alt="image" />
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

[Install](#installation) delta and add this to your `~/.gitconfig`:

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

# Contents

- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
  - [Git config file](#git-config-file)
- [How delta works](#how-delta-works)
- [Usage](#usage)
  - [Choosing colors (styles)](#choosing-colors-styles)
  - [Line numbers](#line-numbers)
  - [Side-by-side view](#side-by-side-view)
  - [Grep](#grep)
  - ["Features": named groups of settings](#features-named-groups-of-settings)
  - [Custom themes](#custom-themes)
  - [diff-highlight and diff-so-fancy emulation](#diff-highlight-and-diff-so-fancy-emulation)
  - [--color-moved support](#--color-moved-support)
  - [Navigation keybindings for large diffs](#navigation-keybindings-for-large-diffs)
  - [Merge conflicts](#merge-conflicts)
  - [Git blame](#git-blame)
  - [24 bit color (truecolor)](#24-bit-color-truecolor)
  - [Using Delta with GNU Screen](#using-delta-with-gnu-screen)
  - [Using Delta on Windows](#using-delta-on-windows)
  - [Mouse scrolling](#mouse-scrolling)
  - [Using Delta with Magit](#using-delta-with-magit)
  - [Supported languages and themes](#supported-languages-and-themes)
- [Comparisons with other tools](#comparisons-with-other-tools)
- [Build delta from source](#build-delta-from-source)
- [Related projects](#related-projects)
  - [Used by delta](#used-by-delta)
  - [Using delta](#using-delta)
  - [Similar projects](#similar-projects)
- [Full --help output](#full---help-output)
- [Delta configs used in screenshots](#delta-configs-used-in-screenshots)
  - [Side-by-side view](#side-by-side-view-1)

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

**All the syntax-highlighting color themes that are available with [bat](https://github.com/sharkdp/bat/) are available with delta:**

<br>
<table>
  <tr>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/80850197-b8f5a000-8be8-11ea-8c9e-29c5b213c4b7.png"
           alt="image" />
    </td>
    <td>
      <img width=450px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/80850237-e04c6d00-8be8-11ea-9027-0d2ea62f15c2.png"
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

## Features

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

## Installation

You can download an executable for your system:
[Linux (glibc)](https://github.com/dandavison/delta/releases/download/0.11.1/delta-0.11.1-x86_64-unknown-linux-gnu.tar.gz)
|
[Linux (musl)](https://github.com/dandavison/delta/releases/download/0.11.1/delta-0.11.1-x86_64-unknown-linux-musl.tar.gz)
|
[MacOS](https://github.com/dandavison/delta/releases/download/0.11.1/delta-0.11.1-x86_64-apple-darwin.tar.gz)
|
[Windows](https://github.com/dandavison/delta/releases/download/0.11.1/delta-0.11.1-x86_64-pc-windows-msvc.zip)
|
[All](https://github.com/dandavison/delta/releases)

Alternatively you can install delta using a package manager: see [repology.org/git-delta](https://repology.org/project/git-delta/versions).

Note that the package is often called `git-delta`, but the executable installed is called `delta`. Here is a quick sumary for selected package managers:

<table>
  <tr>
    <td><a href="https://archlinux.org/packages/community/x86_64/git-delta/">Arch Linux</a></td>
    <td><code>pacman -S git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://crates.io/crates/git-delta">Cargo</a></td>
    <td><code>cargo install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://src.fedoraproject.org/rpms/rust-git-delta">Fedora</a></td>
    <td><code>dnf install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://pkgs.org/download/git-delta">FreeBSD</a></td>
    <td><code>pkg install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://packages.gentoo.org/packages/dev-util/git-delta">Gentoo</a></td>
    <td><code>emerge dev-util/git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://formulae.brew.sh/formula/git-delta">Homebrew</a></td>
    <td><code>brew install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://ports.macports.org/port/git-delta/summary">MacPorts</a></td>
    <td><code>port install git-delta</code></td>
  </tr>
  <tr>
    <td><a href="https://search.nixos.org/packages?show=delta&query=delta">Nix</a></td>
    <td><code>nix-env -iA nixpkgs.delta</code>
  </tr>
  <tr>
    <td><a href="https://software.opensuse.org/package/git-delta">openSUSE Tumbleweed</a></td>
    <td><code>zypper install git-delta</code>
  </tr>
  <tr>
    <td><a href="https://github.com/void-linux/void-packages/tree/master/srcpkgs/delta">Void Linux</a></td>
    <td><code>xbps-install -S delta</code>
  </tr>
  <tr>
    <td>Windows (<a href="https://chocolatey.org/packages/delta">Chocolatey</a>)</td>
    <td><code>choco install delta</code></td>
  </tr>
  <tr>
    <td>Windows (<a href="https://scoop.sh/">Scoop</a>)</td>
    <td><code>scoop install delta</code></td>
  </tr>
  <tr>
    <td>Debian / Ubuntu</td>
    <td>
      <code>dpkg -i file.deb</code>
      <br>
      .deb files are on the <a href="https://github.com/dandavison/delta/releases">releases</a> page.
      <br>
      <sup><b>IMPORTANT</b>: If you are using Ubuntu <= 19.10 or are mixing apt sources, read <a href="https://github.com/dandavison/delta/issues/504">#504</a>, be extremely cautious, and try the versions linked against musl.</sup>
    </td>
  </tr>
</table>

Users of older MacOS versions (e.g. 10.11 El Capitan) should install using Homebrew, Cargo, or MacPorts: the binaries on the release page will not work.

Behind the scenes, delta uses [`less`](https://www.greenwoodsoftware.com/less/) for paging.
It's important to have a reasonably recent version of less installed.
On MacOS, install `less` from Homebrew. For Windows, see [Using Delta on Windows](#using-delta-on-windows).

## Configuration

#### Git config file

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

Delta can also be used as a shorthand for diffing two files: the following two commands do the same thing:

```
delta a.txt b.txt

git diff a.txt b.txt
```

Delta also handles unified diff format, e.g. `diff -u a.txt b.txt | delta`.

For Mercurial, you can add delta, with its command line options, to the `[pager]` section of `.hgrc`.

## How delta works

If you configure delta in gitconfig as above, then git will automatically send its output to delta.
Delta in turn passes its own output on to a "real" pager.
Note that git will only send its output to delta if git believes that its output is going to a terminal (a "tty") for a human to read.
In other words, if you do something like `git diff | grep ...` then you don't have to worry about delta changing the output from git, because delta will never be invoked at all.
If you need to force delta to be invoked when git itself would not invoke it, then you can always pipe to delta explicitly.
For example, `git diff | delta | something-that-expects-delta-output-with-colors` (in this example, git's output is being sent to a pipe, so git itself will not invoke delta).
In general however, delta's output is intended for humans, not machines.

The pager that delta uses is determined by consulting the following environment variables (in this order):

- `DELTA_PAGER`
- `PAGER`

If neither is set, delta's fallback is `less -R`.

The behavior of delta's default pager, `less`, can be controlled using the `LESS` environment variable.
It may contain any of the `less` command line options and/or interactive less-commands (prefixed by a leading `+` sign; these are executed every time right after less is launched).
For full documentation of `less` configuration options, please see the `less(1)` [manual](https://jlk.fjfi.cvut.cz/arch/manpages/man/core/less/less.1.en).

In addition to `DELTA_PAGER`, and `PAGER`, delta currently also consults `$BAT_PAGER` (with priority between the two).
However, this is deprecated: please use `DELTA_PAGER` instead.
No other [`bat`](https://github.com/sharkdp/bat) environment variables are used by delta.

If you are interested in the implementation of delta, please see [ARCHITECTURE.md](./ARCHITECTURE.md).

## Usage

### Choosing colors (styles)

All options that have a name like `--*-style` work in the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
https://git-scm.com/docs/git-config#Documentation/git-config.txt-color

Here's an example:

```gitconfig
[delta]
    minus-style = red bold ul "#ffeeee"
```

That means: For removed lines, set the foreground (text) color to 'red', make it bold and underlined, and set the background color to `#ffeeee`.

For full details, see the `STYLES` section in [`delta --help`](#full---help-output).

### Line numbers

```gitconfig
[delta]
    line-numbers = true
```

<table><tr><td><img width=400px src="https://user-images.githubusercontent.com/52205/86275526-76792100-bba1-11ea-9e78-6be9baa80b29.png" alt="image" /></td></tr></table>

The numbers are displayed in two columns and there are several configuration options: see the `LINE NUMBERS` section in [`delta --help`](#full---help-output) for details, and see the next section for an example of configuring line numbers.

### Side-by-side view

```gitconfig
[delta]
    side-by-side = true
```

By default, side-by-side view has line-numbers activated, and has syntax highlighting in both the left and right panels: [[config](#side-by-side-view-1)]

<table><tr><td><img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" /></td></tr></table>

To disable the line numbers in side-by-side view, but keep a vertical delimiter line between the left and right panels, use the line-numbers format options. For example:

```gitconfig
[delta]
    side-by-side = true
    line-numbers-left-format = ""
    line-numbers-right-format = "│ "
```

Long lines are wrapped if they do not fit in side-by-side mode.
In the image below, the long deleted line in the left panel overflows by a small amount, and the wrapped content is right-aligned in the next line.
In contrast, the long replacement line in the right panel overflows by almost an entire line, and so the wrapped content is left aligned in the next line. The arrow markers and ellipsis explain when and how text has been wrapped.

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/139064537-f8479504-16d3-429a-b4f6-d0122438adaa.png" alt="image" /></td></tr></table>

For control over the details of line wrapping, see `--wrap-max-lines`, `--wrap-left-symbol`, `--wrap-right-symbol`, `--wrap-right-percent`, `--wrap-right-prefix-symbol`, `--inline-hint-style`.
Line wrapping was implemented by @th1000s.

### Grep

Delta applies syntax-highlighting and other enhancements to standard grep output such as from `git grep`, [ripgrep](https://github.com/BurntSushi/ripgrep/) (aka `rg`), grep, etc.
To use with `git grep`, set delta as the pager for `grep` in the `[pager]` section of your gitconfig. See the example at the [top of the page](#get-started).
Output from other grep tools can be piped to delta: e.g. `rg -Hn --color=always`, `grep -Hn --color=always`, etc.
To customize the colors and syntax highlighting, see `grep-match-line-style`, `grep-match-word-style`, `grep-contexct-line-style`, `grep-file-style`, `grep-line-number-style`.
Ripgrep's `rg --json` output format is supported; this avoids certain file name parsing ambiguities that are inevitable with the standard grep output formats.
Note that `git grep` can display the "function context" for matches and that delta handles this output specially: see the `-p` and `-W` options of `git grep`.

### "Features": named groups of settings

All delta options can go under the `[delta]` section in your git config file. However, you can also use named "features" to keep things organized: these are sections in git config like `[delta "my-feature"]`. Here's an example using two custom features:

```gitconfig
[delta]
    features = unobtrusive-line-numbers decorations
    whitespace-error-style = 22 reverse

[delta "unobtrusive-line-numbers"]
    line-numbers = true
    line-numbers-minus-style = "#444444"
    line-numbers-zero-style = "#444444"
    line-numbers-plus-style = "#444444"
    line-numbers-left-format = "{nm:>4}┊"
    line-numbers-right-format = "{np:>4}│"
    line-numbers-left-style = blue
    line-numbers-right-style = blue

[delta "decorations"]
    commit-decoration-style = bold yellow box ul
    file-style = bold yellow ul
    file-decoration-style = none
    hunk-header-decoration-style = yellow box
```

<table><tr><td><img width=400px src="https://user-images.githubusercontent.com/52205/86275048-a96ee500-bba0-11ea-8a19-584f69758aee.png" alt="image" /></td></tr></table>

### Custom themes

A "theme" in delta is just a collection of settings grouped together in a named [feature](#features-named-groups-of-settings). One of the available settings is `syntax-theme`: this dictates the colors and styles that are applied to foreground text by the syntax highlighter. Thus the concept of "theme" in delta encompasses not just the foreground syntax-highlighting color theme, but also background colors, decorations such as boxes and under/overlines, etc.

The delta git repo contains a [collection of themes](./themes.gitconfig) created by users. These focus on the visual appearance: colors etc. If you want features like `side-by-side` or `navigate`, you would set that yourself, after selecting the color theme. To use the delta themes, clone the delta repo (or download the [themes.gitconfig](./themes.gitconfig) file) and add the following entry in your gitconfig:

```gitconfig
[include]
    path = /PATH/TO/delta/themes.gitconfig
```

Then, add your chosen color theme to your features list, e.g.

```gitconfig
[delta]
    features = collared-trogon
    side-by-side = true
    ...
```

Note that this terminology differs from [bat](https://github.com/sharkdp/bat): bat does not apply background colors, and uses the term "theme" to refer to what delta calls `syntax-theme`. Delta does not have a setting named "theme": a theme is a "feature", so one uses `features` to select a theme.

### diff-highlight and diff-so-fancy emulation

Use `--diff-highlight` or `--diff-so-fancy` to activate the respective emulation mode.

You may want to know which delta configuration values the emulation mode has selected, so that you can adjust them. To do that, use e.g. `delta --diff-so-fancy --show-config`:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/86271121-5abe4c80-bb9a-11ea-950a-7c79502267d5.png" alt="image" /></td></tr></table>

[diff-highlight](https://github.com/git/git/tree/master/contrib/diff-highlight) is a perl script distributed with git that allows within-line edits to be identified and highlighted according to colors specified in git config. [diff-so-fancy](https://github.com/so-fancy/diff-so-fancy) builds on diff-highlight, making various additional improvements to the default git diff output. Both tools provide very helpful ways of viewing diffs, and so delta provides emulation modes for both of them.

The within-line highlighting rules employed by diff-highlight (and therefore by diff-so-fancy) are deliberately simpler than Delta's Levenshtein-type edit inference algorithm (see discussion in the [diff-highlight README](https://github.com/git/git/tree/master/contrib/diff-highlight)). diff-highlight's rules could be added to delta as an alternative highlighting algorithm, but that hasn't been done yet.

### `--color-moved` support

Recent versions of Git (≥ v2.17, April 2018) are able to detect moved blocks of code and style them differently from the usual removed/added lines. If you have activated this feature in Git, then Delta will automatically detect such differently-styled lines, and display them unchanged, i.e. with the raw colors it receives from Git.

To activate the Git feature, use

```gitconfig
[diff]
    colorMoved = default
```

and see the [Git documentation](https://git-scm.com/docs/git-diff#Documentation/git-diff.txt---color-movedltmodegt) for the other possible values and associated color configuration.

The `map-styles` option allows us to transform the styles that git emits for color-moved sections into delta styles.
Here's an example of using `map-styles` to assign delta styles to the raw color-moved styles output by git.
This feature allows all of git's color-moved options to be rendered using delta styles, including with syntax highlighting.

```gitconfig
[delta]
    map-styles = bold purple => syntax magenta, bold cyan => syntax blue
```

It is also possible to reference other styles.

```gitconfig
[delta]
    features = my-color-moved-theme

[delta "my-color-moved-theme"]
    git-moved-from-style = bold purple     # An ad-hoc named style (must end in "-style")

    map-styles = "my-color-moved-theme.git-moved-from-style => red #cccccc, \
                  bold cyan => syntax #cccccc"

    # we could also have defined git-moved-to-style = bold cyan
```

To make use of that, you need to know that git is emitting "bold cyan" and "bold purple".
But that's not always obvious.
To help with that, delta now has a `--parse-ansi` mode. E.g. `git show --color=always | delta --parse-ansi` outputs something like this:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/143238872-58a40754-ae50-4a9e-ba72-07e330e520e6.png" alt="image" /></td></tr></table>

As you see above, we can now define named styles in gitconfig and refer to them in places where a style string is expected.
We can also define custom named colors in git config, and styles can reference other styles; see the [hoopoe theme](https://github.com/dandavison/delta/blob/master/themes.gitconfig#L76-L91) for an example:

```gitconfig
[delta "hoopoe"]
    green = "#d0ffd0"  # ad-hoc named color
    plus-style = syntax hoopoe.green  # refer to named color
    plus-non-emph-style = plus-style  # styles can reference other styles
```

Additionally, we can now use the 140 color names that are standard in CSS. Use `delta --show-colors` to get a demo of the available colors, as background colors to see how they look with syntax highlighting:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/143237384-246db199-ef65-4ad2-ad4e-03d07d1ea41d.png" alt="image" /></td></tr></table>

### Navigation keybindings for large diffs

Use the `navigate` feature to activate navigation keybindings. In this mode, pressing `n` will jump forward to the next file in the diff, and `N` will jump backwards. If you are viewing multiple commits (e.g. via `git log -p`) then navigation will also visit commit boundaries.

### Merge conflicts

Please consider setting

```gitconfig
[merge]
    conflictstyle = diff3
```
With that setting, when a merge conflict is encountered, delta will display diffs between the ancestral commit and each of the two merge parents:

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/144783121-bb549100-69d8-41b8-ac62-1704f1f7b43e.png" alt="image" /></td></tr></table>

This display can be customized using `merge-conflict-begin-symbol`, `merge-conflict-end-symbol`, `merge-conflict-ours-diff-header-style`, `merge-conflict-ours-diff-header-decoration-style`, `merge-conflict-theirs-diff-header-style`, `merge-conflict-theirs-diff-header-decoration-style`.

### Git blame

Set delta as the pager for `blame` in the `[pager]` section of your gitconfig: see the [example gitconfig](#get-started).
If `hyperlinks` is enabled in the `[delta]` section then each blame commit will link to the commit on GitHub/GitLab/Bitbucket/etc.

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/141891376-1fdb87dc-1d9c-4ad6-9d72-eeb19a8aeb0b.png" alt="image" /></td></tr></table>

### 24 bit color (truecolor)

Delta looks best if your terminal application supports 24 bit colors. See https://github.com/termstandard/colors#readme. For example, on MacOS, iTerm2 supports 24-bit colors but Terminal.app does not.

If your terminal application does not support 24-bit color, delta will still work, by automatically choosing the closest color from those available. See the `Colors` section of the help output below.

If you're using tmux, it's worth checking that 24 bit color is working correctly. For example, run a color test script like [this one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh), or one of the others listed [here](https://gist.github.com/XVilka/8346728). If you do not see smooth color gradients, see the discussion at [tmux#696](https://github.com/tmux/tmux/issues/696). The short version is you need something like this in your `~/.tmux.conf`:

```Shell
set -ga terminal-overrides ",xterm-256color:Tc"
```

and you may then need to quit tmux completely for it to take effect.

True color output in GNU Screen is currently only possible when using a development build, as support for it is not yet implemented in the (v4) release versions. A snapshot of the latest Git trunk can be obtained via https://git.savannah.gnu.org/cgit/screen.git/snapshot/screen-master.tar.gz - the required build steps are described in the `src/INSTALL` file. After installing the program, 24-bit color support can be activated by including `truecolor on` in either the system's or the user's `screenrc` file.

### Using Delta with GNU Screen

When working in Screen without true color output, it might be that colors supposed to be different look the same in XTerm compatible terminals. If that is the case, make sure the following settings are included in your `screenrc` file:

```Shell
term screen-256color
termcapinfo xterm 'Co#256:AB=\E[48;5;%dm:AF=\E[38;5;%dm'  # ANSI (256-color) patterns - AB: background, AF: foreground
attrcolor b ".I"                                          # use bright colors for bold text
```

If despite having those settings you still only get a limited set of colors, your build of Screen might have been configured without the `--enable-colors256` flag. If this is the case, you have two options :

- If available for your OS, get a different package of Screen. Otherwise
- Build your own binary :
  - Download and extract a release tarball from https://ftp.gnu.org/gnu/screen/
  - `cd` into the newly extracted folder
  - Follow the instructions in the `INSTALL` file, and when running the `./configure` command apply the `--enable-colors256` flag.

### Using Delta on Windows

Delta works on Windows. However, it is essential to use a recent version of `less.exe`: you can download one from https://github.com/jftuga/less-Windows/releases/latest. If you see incorrect colors and/or strange characters in Delta output, then it is probably because Delta is picking up an old version of `less.exe` on your system.

### Mouse scrolling

If mouse scrolling isn't working correctly, ensure that you have the most recent version of `less`.

- For Windows you can download from https://github.com/jftuga/less-Windows/releases/latest
- For Mac you can install `brew install less; brew link less`

Alternatively try setting your `DELTA_PAGER` environment variable to (at least) `less -R`. See [issue #58](https://github.com/dandavison/delta/issues/58). See also [bat README / "Using a different pager"](https://github.com/sharkdp/bat#using-a-different-pager), since the `DELTA_PAGER` environment variable functions very similarly for delta.

### Using Delta with Magit

Delta can be used when displaying diffs in the Magit git client: see [magit-delta](https://github.com/dandavison/magit-delta). Here's a screenshot:

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/79934267-2acb2e00-8420-11ea-8bc4-546508fd3581.png" alt="image" /></td></tr></table>

### Supported languages and themes

To list the supported languages and color themes, use `delta --list-languages` and `delta --list-syntax-themes`. To see a demo of the color themes, use `delta --show-syntax-themes`:

To add your own custom color theme, or language, please follow the instructions in the Customization section of the [bat documentation](https://github.com/sharkdp/bat/#customization):

- [Adding a custom language](https://github.com/sharkdp/bat/#adding-new-syntaxes--language-definitions)
- [Adding a custom theme](https://github.com/sharkdp/bat/#adding-new-themes)

Delta automatically recognizes custom themes and languages added to bat. You will need to install bat in order to run the `bat cache --build` command.

The languages and color themes that ship with delta are those that ship with bat. So, to propose a new language or color theme for inclusion in delta, it would need to be a helpful addition to bat, in which case please open a PR against bat.

## Comparisons with other tools

(`delta --light`)

<table>
  <tr>
    <td>
      delta vs. git
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248525-32250480-daea-11e9-9965-1a05c6a4bdf4.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248441-14f03600-daea-11e9-88a1-d96bbb6947f8.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      delta vs. diff-so-fancy /<br>diff-highlight
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248525-32250480-daea-11e9-9965-1a05c6a4bdf4.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248407-07d34700-daea-11e9-9a8f-6d81f4021abf.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      delta vs. github
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248525-32250480-daea-11e9-9965-1a05c6a4bdf4.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65248749-9a73e600-daea-11e9-9c0d-29c8f1dea08e.png"
           alt="image" />
    </td>
  </tr>
</table>

## Build delta from source

You'll need to [install the rust tools](https://www.rust-lang.org/learn/get-started). Then:

```ShellSession
$ cargo build --release
```

and use the executable found at `./target/release/delta`.

## Related projects

### Used by delta

- [alacritty/vte](https://github.com/alacritty/vte)
- [mitsuhiko/console](https://github.com/mitsuhiko/console)
- [ogham/rust-ansi-term](https://github.com/ogham/rust-ansi-term)
- [sharkdp/bat](https://github.com/sharkdp/bat)
- [trishume/syntect](https://github.com/trishume/syntect)

### Using delta

- [bigH/git-fuzzy](https://github.com/bigH/git-fuzzy)
- [dandavison/magit-delta](https://github.com/dandavison/magit-delta)
- [jesseduffield/lazygit](https://github.com/jesseduffield/lazygit/)
- [junegunn/fzf.vim](https://github.com/junegunn/fzf.vim)
- [ms-jpq/sad](https://github.com/ms-jpq/sad)
- [wfxr/forgit](https://github.com/wfxr/forgit)

### Similar projects

- [banga/git-split-diffs](https://github.com/banga/git-split-diffs)
- [da-x/fancydiff](https://github.com/da-x/fancydiff)
- [git/diff-highlight](https://github.com/git/git/tree/master/contrib/diff-highlight)
- [jeffkaufman/icdiff](https://github.com/jeffkaufman/icdiff)
- [kovidgoyal/kitty-diff](https://sw.kovidgoyal.net/kitty/kittens/diff.html)
- [mookid/diffr](https://github.com/mookid/diffr)
- [nkouevda/pdiff](https://github.com/nkouevda/pdiff)
- [so-fancy/diff-so-fancy](https://github.com/so-fancy/diff-so-fancy)
- [Wilfred/difftastic](https://github.com/Wilfred/difftastic)

## Full --help output

```
delta 0.11.1
A viewer for git and diff output

USAGE:
    delta [FLAGS] [OPTIONS] [ARGS]

FLAGS:
        --light                      Use default colors appropriate for a light terminal background. For more control,
                                     see the style options and --syntax-theme
        --dark                       Use default colors appropriate for a dark terminal background. For more control,
                                     see the style options and --syntax-theme
    -n, --line-numbers               Display line numbers next to the diff. See LINE NUMBERS section
    -s, --side-by-side               Display a side-by-side diff view instead of the traditional view
        --diff-highlight             Emulate diff-highlight (https://github.com/git/git/tree/master/contrib/diff-highlight)
        --diff-so-fancy              Emulate diff-so-fancy (https://github.com/so-fancy/diff-so-fancy)
        --navigate                   Activate diff navigation: use n to jump forwards and N to jump backwards. To change
                                     the file labels used see --file-modified-label, --file-removed-label, --file-added-
                                     label, --file-renamed-label
        --relative-paths             Output all file paths relative to the current directory so that they resolve
                                     correctly when clicked on or used in shell commands
        --hyperlinks                 Render commit hashes, file names, and line numbers as hyperlinks, according to the
                                     hyperlink spec for terminal emulators:
                                     https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda. By default,
                                     file names and line numbers link to the local file using a file URL, whereas commit
                                     hashes link to the commit in GitHub, if the remote repository is hosted by GitHub.
                                     See --hyperlinks-file-link-format for full control over the file URLs emitted.
                                     Hyperlinks are supported by several common terminal emulators. To make them work,
                                     you must use less version >= 581 with the -R flag (or use -r with older less
                                     versions, but this will break e.g. --navigate). If you use tmux, then you will also
                                     need a patched fork of tmux (see https://github.com/dandavison/tmux)
        --keep-plus-minus-markers    Prefix added/removed lines with a +/- character, exactly as git does. By default,
                                     delta does not emit any prefix, so code can be copied directly from delta's output
        --show-config                Display the active values for all Delta options. Style options are displayed with
                                     foreground and background colors. This can be used to experiment with colors by
                                     combining this option with other options such as --minus-style, --zero-style,
                                     --plus-style, --light, --dark, etc
        --list-languages             List supported languages and associated file extensions
        --list-syntax-themes         List available syntax-highlighting color themes
        --show-syntax-themes         Show all available syntax-highlighting themes, each with an example of highlighted
                                     diff output. If diff output is supplied on standard input then this will be used
                                     for the demo. For example: `git show | delta --show-syntax-themes`
        --show-themes                Show available delta themes, each with an example of highlighted diff output. A
                                     delta theme is a delta named feature (see --features) that sets either `light` or
                                     `dark`. See https://github.com/dandavison/delta#custom-color-themes. If diff output
                                     is supplied on standard input then this will be used for the demo. For example:
                                     `git show | delta --show-themes`. By default shows dark or light themes only,
                                     according to whether delta is in dark or light mode (as set by the user or inferred
                                     from BAT_THEME). To control the themes shown, use --dark or --light, or both, on
                                     the command line together with this option
        --show-colors                Show available named colors. In addition to named colors, arbitrary colors can be
                                     specified using RGB hex codes. See COLORS section
        --parse-ansi                 Parse ANSI color escape sequences in input and display them as git style strings.
                                     Example usage: git show --color=always | delta --parse-ansi This can be used to
                                     help identify input style strings to use with map-styles
        --no-gitconfig               Do not take any settings from git config. See GIT CONFIG section
        --raw                        Do not alter the input in any way. This is mainly intended for testing delta
        --color-only                 Do not alter the input structurally in any way, but color and highlight hunk lines
                                     according to your delta configuration. This is mainly intended for other tools that
                                     use delta
        --highlight-removed          Deprecated: use --minus-style='syntax'
    -h, --help                       Prints help information
    -V, --version                    Prints version information

OPTIONS:
        --features <features>
            Name of delta features to use (space-separated). A feature is a named collection of delta options in
            ~/.gitconfig. See FEATURES section [env: DELTA_FEATURES=]  [default: ]
        --syntax-theme <syntax-theme>
            The code syntax-highlighting theme to use. Use --show-syntax-themes to demo available themes. If the syntax-
            highlighting theme is not set using this option, it will be taken from the BAT_THEME environment
            variable, if that contains a valid theme name. --syntax-theme=none disables all syntax highlighting [env:
            BAT_THEME=]
        --minus-style <minus-style>
            Style (foreground, background, attributes) for removed lines. See STYLES section [default: normal auto]
        --zero-style <zero-style>
            Style (foreground, background, attributes) for unchanged lines. See STYLES section [default: syntax normal]
        --plus-style <plus-style>
            Style (foreground, background, attributes) for added lines. See STYLES section [default: syntax auto]
        --minus-emph-style <minus-emph-style>
            Style (foreground, background, attributes) for emphasized sections of removed lines. See STYLES section
            [default: normal auto]
        --minus-non-emph-style <minus-non-emph-style>
            Style (foreground, background, attributes) for non-emphasized sections of removed lines that have an
            emphasized section. Defaults to --minus-style. See STYLES section [default: auto auto]
        --plus-emph-style <plus-emph-style>
            Style (foreground, background, attributes) for emphasized sections of added lines. See STYLES section
            [default: syntax auto]
        --plus-non-emph-style <plus-non-emph-style>
            Style (foreground, background, attributes) for non-emphasized sections of added lines that have an
            emphasized section. Defaults to --plus-style. See STYLES section [default: auto auto]
        --commit-style <commit-style>
            Style (foreground, background, attributes) for the commit hash line. See STYLES section. The style 'omit'
            can be used to remove the commit hash line from the output [default: raw]
        --commit-decoration-style <commit-decoration-style>
            Style (foreground, background, attributes) for the commit hash decoration. See STYLES section. The style
            string should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the
            combination 'ul ol' [default: ]
        --commit-regex <commit-regex>
            The regular expression used to identify the commit line when parsing git output [default: ^commit ]

        --file-style <file-style>
            Style (foreground, background, attributes) for the file section. See STYLES section. The style 'omit' can be
            used to remove the file section from the output [default: blue]
        --file-decoration-style <file-decoration-style>
            Style (foreground, background, attributes) for the file decoration. See STYLES section. The style string
            should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the combination
            'ul ol' [default: blue ul]
        --hyperlinks-commit-link-format <hyperlinks-commit-link-format>
            Format string for commit hyperlinks (requires --hyperlinks). The placeholder "{commit}" will be replaced by
            the commit hash. For example: --hyperlinks-commit-link-format='https://mygitrepo/{commit}/'
        --hyperlinks-file-link-format <hyperlinks-file-link-format>
            Format string for file hyperlinks (requires --hyperlinks). The placeholders "{path}" and "{line}" will be
            replaced by the absolute file path and the line number, respectively. The default value of this option
            creates hyperlinks using standard file URLs; your operating system should open these in the application
            registered for that file type. However, these do not make use of the line number. In order for the link to
            open the file at the correct line number, you could use a custom URL format such as "file-
            line://{path}:{line}" and register an application to handle the custom "file-line" URL scheme by
            opening the file in your editor/IDE at the indicated line number. See https://github.com/dandavison/open-in-
            editor for an example [default: file://{path}]
        --hunk-header-style <hunk-header-style>
            Style (foreground, background, attributes) for the hunk-header. See STYLES section. Special attributes
            'file' and 'line-number' can be used to include the file path, and number of first hunk line, in the hunk
            header. The style 'omit' can be used to remove the hunk header section from the output [default: line-
            number syntax]
        --hunk-header-file-style <hunk-header-file-style>
            Style (foreground, background, attributes) for the file path part of the hunk-header. See STYLES section.
            The file path will only be displayed if hunk-header-style contains the 'file' special attribute [default:
            blue]
        --hunk-header-line-number-style <hunk-header-line-number-style>
            Style (foreground, background, attributes) for the line number part of the hunk-header. See STYLES section.
            The line number will only be displayed if hunk-header-style contains the 'line-number' special attribute
            [default: blue]
        --hunk-header-decoration-style <hunk-header-decoration-style>
            Style (foreground, background, attributes) for the hunk-header decoration. See STYLES section. The style
            string should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the
            combination 'ul ol' [default: blue box]
        --merge-conflict-begin-symbol <merge-conflict-begin-symbol>
            A string that is repeated to form the line marking the beginning of a merge conflict region [default:
            ▼]
        --merge-conflict-end-symbol <merge-conflict-end-symbol>
            A string that is repeated to form the line marking the end of a merge conflict region [default: ▲]

        --merge-conflict-ours-diff-header-style <merge-conflict-ours-diff-header-style>
            Style (foreground, background, attributes) for the header above the diff between the ancestral commit and
            'our' branch. See STYLES section [default: normal]
        --merge-conflict-ours-diff-header-decoration-style <merge-conflict-ours-diff-header-decoration-style>
            Style (foreground, background, attributes) for the decoration of the header above the diff between the
            ancestral commit and 'our' branch.  See STYLES section. The style string should contain one of the special
            attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol' [default: box]
        --merge-conflict-theirs-diff-header-style <merge-conflict-theirs-diff-header-style>
            Style (foreground, background, attributes) for the header above the diff between the ancestral commit and
            'their' branch. See STYLES section [default: normal]
        --merge-conflict-theirs-diff-header-decoration-style <merge-conflict-theirs-diff-header-decoration-style>
            Style (foreground, background, attributes) for the decoration of the header above the diff between the
            ancestral commit and 'their' branch.  See STYLES section. The style string should contain one of the special
            attributes 'box', 'ul' (underline), 'ol' (overline), or the combination 'ul ol' [default: box]
        --map-styles <map-styles>
            A string specifying a mapping styles encountered in raw input to desired output styles. An example is --map-
            styles='bold purple => red "#eeeeee", bold cyan => syntax "#eeeeee"'
        --blame-format <blame-format>
            Format string for git blame commit metadata. Available placeholders are "{timestamp}", "{author}", and
            "{commit}" [default: {timestamp:<15} {author:<15.14} {commit:<8} │ ]
        --blame-palette <blame-palette>
            Background colors used for git blame lines (space-separated string). Lines added by the same commit are
            painted with the same color; colors are recycled as needed
        --blame-timestamp-format <blame-timestamp-format>
            Format of `git blame` timestamp in raw git output received by delta [default: %Y-%m-%d %H:%M:%S %z]

        --grep-match-line-style <grep-match-line-style>
            Style (foreground, background, attributes) for matching lines of code in grep output. See STYLES section.
            Defaults to plus-style
        --grep-match-word-style <grep-match-word-style>
            Style (foreground, background, attributes) for the specific matching substrings within a matching line of
            code in grep output. See STYLES section. Defaults to plus-style
        --grep-context-line-style <grep-context-line-style>
            Style (foreground, background, attributes) for non-matching lines of code in grep output. See STYLES
            section. Defaults to zero-style
        --grep-file-style <grep-file-style>
            Style (foreground, background, attributes) for file paths in grep output. See STYLES section. Defaults to
            hunk-header-file-path-style
        --grep-line-number-style <grep-line-number-style>
            Style (foreground, background, attributes) for line numbers in grep output. See STYLES section. Defaults to
            hunk-header-line-number-style
        --grep-separator-symbol <grep-separator-symbol>
            Symbol used in grep output to separate file path (and line number) from the line of file contents. Defaults
            to ":" for both match and context lines, since many terminal emulators recognize constructs like
            "/path/to/file:7:". However, standard grep output uses "-" for context lines: set this option to "keep" to
            keep the original separator symbols [default: :]
        --default-language <default-language>
            Default language used for syntax highlighting when this cannot be inferred from a filename. It will
            typically make sense to set this in per-repository git config (.git/config)
        --inline-hint-style <inline-hint-style>
            Style (foreground, background, attributes) for content added by delta to the original diff such as special
            characters to highlight tabs, and the symbols used to indicate wrapped lines. See STYLES section [default:
            blue]
        --word-diff-regex <tokenization-regex>
            The regular expression used to decide what a word is for the within-line highlight algorithm. For less fine-
            grained matching than the default try --word-diff-regex="\S+" --max-line-distance=1.0 (this is more
            similar to `git --word-diff`) [default: \w+]
        --max-line-distance <max-line-distance>
            The maximum distance between two lines for them to be inferred to be homologous. Homologous line pairs are
            highlighted according to the deletion and insertion operations transforming one into the other [default: 0.6]
        --line-numbers-minus-style <line-numbers-minus-style>
            Style (foreground, background, attributes) for line numbers in the old (minus) version of the file. See
            STYLES and LINE NUMBERS sections [default: auto]
        --line-numbers-zero-style <line-numbers-zero-style>
            Style (foreground, background, attributes) for line numbers in unchanged (zero) lines. See STYLES and LINE
            NUMBERS sections [default: auto]
        --line-numbers-plus-style <line-numbers-plus-style>
            Style (foreground, background, attributes) for line numbers in the new (plus) version of the file. See
            STYLES and LINE NUMBERS sections [default: auto]
        --line-numbers-left-format <line-numbers-left-format>
            Format string for the left column of line numbers. A typical value would be "{nm:^4}⋮" which means to
            display the line numbers of the minus file (old version), center-aligned, padded to a width of 4 characters,
            followed by a dividing character. See the LINE NUMBERS section [default: {nm:^4}⋮]
        --line-numbers-right-format <line-numbers-right-format>
            Format string for the right column of line numbers. A typical value would be "{np:^4}│ " which means to
            display the line numbers of the plus file (new version), center-aligned, padded to a width of 4 characters,
            followed by a dividing character, and a space. See the LINE NUMBERS section [default: {np:^4}│]
        --line-numbers-left-style <line-numbers-left-style>
            Style (foreground, background, attributes) for the left column of line numbers. See STYLES and LINE NUMBERS
            sections [default: auto]
        --line-numbers-right-style <line-numbers-right-style>
            Style (foreground, background, attributes) for the right column of line numbers. See STYLES and LINE NUMBERS
            sections [default: auto]
        --wrap-max-lines <wrap-max-lines>
            How often a line should be wrapped if it does not fit. Zero means to never wrap. Any content which does not
            fit will be truncated. A value of "unlimited" means a line will be wrapped as many times as required
            [default: 2]
        --wrap-left-symbol <wrap-left-symbol>
            Symbol added to the end of a line indicating that the content has been wrapped onto the next line and
            continues left-aligned [default: ↵]
        --wrap-right-symbol <wrap-right-symbol>
            Symbol added to the end of a line indicating that the content has been wrapped onto the next line and
            continues right-aligned [default: ↴]
        --wrap-right-percent <wrap-right-percent>
            Threshold for right-aligning wrapped content. If the length of the remaining wrapped content, as a
            percentage of width, is less than this quantity it will be right-aligned. Otherwise it will be left-aligned
            [default: 37.0]
        --wrap-right-prefix-symbol <wrap-right-prefix-symbol>
            Symbol displayed in front of right-aligned wrapped content [default: …]

        --navigate-regex <navigate-regex>
            A regexp to use in the less pager when navigating (auto-generated when unspecified)

        --file-modified-label <file-modified-label>
            Text to display in front of a modified file path [default: ]

        --file-removed-label <file-removed-label>
            Text to display in front of a removed file path [default: removed:]

        --file-added-label <file-added-label>
            Text to display in front of a added file path [default: added:]

        --file-copied-label <file-copied-label>
            Text to display in front of a copied file path [default: copied:]

        --file-renamed-label <file-renamed-label>
            Text to display in front of a renamed file path [default: renamed:]

        --right-arrow <right-arrow>
            Text to display with a changed value such as a diff heading, a rename, or a chmod [default: ⟶  ]

        --hunk-label <hunk-label>
            Text to display in front of a hunk header [default: ]

        --max-line-length <max-line-length>
            Truncate lines longer than this. To prevent any truncation, set to zero. Note that delta will be slow on
            very long lines (e.g. minified .js) if truncation is disabled. When wrapping lines it is automatically set
            to fit at least all visible characters [default: 512]
        --line-fill-method <line-fill-method>
            How to extend the background color to the end of the line in side-by-side mode. Can be ansi (default) or
            spaces (default if output is not to a terminal). Has no effect if --width=variable is given
    -w, --width <width>
            The width of underline/overline decorations. Examples: "72" (exactly 72 characters), "-2" (auto-detected
            terminal width minus 2). An expression such as "74-2" is also valid (equivalent to 72 but may be useful if
            the caller has a variable holding the value "74"). Use --width=variable to extend decorations and background
            colors to the end of the text only. Otherwise background colors extend to the full terminal width
        --diff-stat-align-width <diff-stat-align-width>
            Width allocated for file paths in a diff stat section. If a relativized file path exceeds this width then
            the diff stat will be misaligned [default: 48]
        --tabs <tab-width>
            The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters through directly,
            but note that in that case delta will calculate line widths assuming tabs occupy one character's width on
            the screen: if your terminal renders tabs as more than than one character wide then delta's output will look
            incorrect [default: 4]
        --true-color <true-color>
            Whether to emit 24-bit ("true color") RGB color codes. Options are auto, always, and never. "auto" means
            that delta will emit 24-bit color codes if the environment variable COLORTERM has the value "truecolor" or
            "24bit". If your terminal application (the application you use to enter commands at a shell prompt) supports
            24 bit colors, then it probably already sets this environment variable, in which case you don't need to do
            anything [default: auto]
        --24-bit-color <24-bit-color>                                      Deprecated: use --true-color
        --inspect-raw-lines <inspect-raw-lines>
            Whether to examine ANSI color escape sequences in raw lines received from Git and handle lines colored in
            certain ways specially. This is on by default: it is how Delta supports Git's --color-moved feature. Set
            this to "false" to disable this behavior [default: true]
        --pager <pager>
            Which pager to use. The default pager is `less`. You can also change pager by setting the environment
            variables DELTA_PAGER, BAT_PAGER, or PAGER (and that is their order of priority). This option overrides all
            environment variables above
        --paging <paging-mode>
            Whether to use a pager when displaying output. Options are: auto, always, and never [default: auto]
        --minus-empty-line-marker-style <minus-empty-line-marker-style>
            Style for removed empty line marker (used only if --minus-style has no background color) [default: normal auto]
        --plus-empty-line-marker-style <plus-empty-line-marker-style>
            Style for added empty line marker (used only if --plus-style has no background color) [default: normal auto]
        --whitespace-error-style <whitespace-error-style>
            Style for whitespace errors. Defaults to color.diff.whitespace if that is set in git config, or else
            'magenta reverse' [default: auto auto]
        --line-buffer-size <line-buffer-size>
            Size of internal line buffer. Delta compares the added and removed versions of nearby lines in order to
            detect and highlight changes at the level of individual words/tokens. Therefore, nearby lines must be
            buffered internally before they are painted and emitted. Increasing this value might improve highlighting of
            some large diff hunks. However, setting this to a high value will adversely affect delta's performance when
            entire files are added/removed [default: 32]
        --minus-color <deprecated-minus-background-color>
            Deprecated: use --minus-style='normal my_background_color'

        --minus-emph-color <deprecated-minus-emph-background-color>
            Deprecated: use --minus-emph-style='normal my_background_color'

        --plus-color <deprecated-plus-background-color>
            Deprecated: Use --plus-style='syntax my_background_color' to change the background color while retaining
            syntax-highlighting
        --plus-emph-color <deprecated-plus-emph-background-color>
            Deprecated: Use --plus-emph-style='syntax my_background_color' to change the background color while
            retaining syntax-highlighting
        --commit-color <deprecated-commit-color>
            Deprecated: use --commit-style='my_foreground_color' --commit-decoration-style='my_foreground_color'

        --file-color <deprecated-file-color>
            Deprecated: use --file-style='my_foreground_color' --file-decoration-style='my_foreground_color'

        --hunk-style <deprecated-hunk-style>
            Deprecated: synonym of --hunk-header-decoration-style

        --hunk-color <deprecated-hunk-color>
            Deprecated: use --hunk-header-style='my_foreground_color' --hunk-header-decoration-
            style='my_foreground_color'
        --theme <deprecated-theme>                                         Deprecated: use --syntax-theme

ARGS:
    <minus-file>    First file to be compared when delta is being used in diff mode: `delta file_1 file_2` is
                    equivalent to `diff -u file_1 file_2 | delta`
    <plus-file>     Second file to be compared when delta is being used in diff mode

GIT CONFIG
----------

By default, delta takes settings from a section named "delta" in git config files, if one is
present. The git config file to use for delta options will usually be ~/.gitconfig, but delta
follows the rules given in https://git-scm.com/docs/git-config#FILES. Most delta options can be
given in a git config file, using the usual option names but without the initial '--'. An example
is

[delta]
    line-numbers = true
    zero-style = dim syntax

FEATURES
--------
A feature is a named collection of delta options in git config. An example is:

[delta "my-delta-feature"]
    syntax-theme = Dracula
    plus-style = bold syntax "#002800"

To activate those options, you would use:

delta --features my-delta-feature

A feature name may not contain whitespace. You can activate multiple features:

[delta]
    features = my-highlight-styles-colors-feature my-line-number-styles-feature

If more than one feature sets the same option, the last one wins.

STYLES
------

All options that have a name like --*-style work the same way. It is very similar to how
colors/styles are specified in a gitconfig file:
https://git-scm.com/docs/git-config#Documentation/git-config.txt-color

Here is an example:

--minus-style 'red bold ul "#ffeeee"'

That means: For removed lines, set the foreground (text) color to 'red', make it bold and
            underlined, and set the background color to '#ffeeee'.

See the COLORS section below for how to specify a color. In addition to real colors, there are 4
special color names: 'auto', 'normal', 'raw', and 'syntax'.

Here is an example of using special color names together with a single attribute:

--minus-style 'syntax bold auto'

That means: For removed lines, syntax-highlight the text, and make it bold, and do whatever delta
            normally does for the background.

The available attributes are: 'blink', 'bold', 'dim', 'hidden', 'italic', 'reverse', 'strike',
and 'ul' (or 'underline').

The attribute 'omit' is supported by commit-style, file-style, and hunk-header-style, meaning to
remove the element entirely from the output.

A complete description of the style string syntax follows:

- If the input that delta is receiving already has colors, and you want delta to output those
  colors unchanged, then use the special style string 'raw'. Otherwise, delta will strip any colors
  from its input.

- A style string consists of 0, 1, or 2 colors, together with an arbitrary number of style
  attributes, all separated by spaces.

- The first color is the foreground (text) color. The second color is the background color.
  Attributes can go in any position.

- This means that in order to specify a background color you must also specify a foreground (text)
  color.

- If you want delta to choose one of the colors automatically, then use the special color 'auto'.
  This can be used for both foreground and background.

- If you want the foreground/background color to be your terminal's foreground/background color,
  then use the special color 'normal'.

- If you want the foreground text to be syntax-highlighted according to its language, then use the
  special foreground color 'syntax'. This can only be used for the foreground (text).

- The minimal style specification is the empty string ''. This means: do not apply any colors or
  styling to the element in question.

COLORS
------

There are four ways to specify a color (this section applies to foreground and background colors
within a style string):

1. CSS color name

   Any of the 140 color names used in CSS: https://www.w3schools.com/colors/colors_groups.asp

2. RGB hex code

   An example of using an RGB hex code is:
   --file-style="#0e7c0e"

3. ANSI color name

   There are 8 ANSI color names:
   black, red, green, yellow, blue, magenta, cyan, white.

   In addition, all of them have a bright form:
   brightblack, brightred, brightgreen, brightyellow, brightblue, brightmagenta, brightcyan, brightwhite.

   An example of using an ANSI color name is:
   --file-style="green"

   Unlike RGB hex codes, ANSI color names are just names: you can choose the exact color that each
   name corresponds to in the settings of your terminal application (the application you use to
   enter commands at a shell prompt). This means that if you use ANSI color names, and you change
   the color theme used by your terminal, then delta's colors will respond automatically, without
   needing to change the delta command line.

   "purple" is accepted as a synonym for "magenta". Color names and codes are case-insensitive.

4. ANSI color number

   An example of using an ANSI color number is:
   --file-style=28

   There are 256 ANSI color numbers: 0-255. The first 16 are the same as the colors described in
   the "ANSI color name" section above. See https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit.
   Specifying colors like this is useful if your terminal only supports 256 colors (i.e. doesn't
   support 24-bit color).


LINE NUMBERS
------------

To display line numbers, use --line-numbers.

Line numbers are displayed in two columns. Here's what it looks like by default:

 1  ⋮ 1  │ unchanged line
 2  ⋮    │ removed line
    ⋮ 2  │ added line

In that output, the line numbers for the old (minus) version of the file appear in the left column,
and the line numbers for the new (plus) version of the file appear in the right column. In an
unchanged (zero) line, both columns contain a line number.

The following options allow the line number display to be customized:

--line-numbers-left-format:  Change the contents of the left column
--line-numbers-right-format: Change the contents of the right column
--line-numbers-left-style:   Change the style applied to the left column
--line-numbers-right-style:  Change the style applied to the right column
--line-numbers-minus-style:  Change the style applied to line numbers in minus lines
--line-numbers-zero-style:   Change the style applied to line numbers in unchanged lines
--line-numbers-plus-style:   Change the style applied to line numbers in plus lines

Options --line-numbers-left-format and --line-numbers-right-format allow you to change the contents
of the line number columns. Their values are arbitrary format strings, which are allowed to contain
the placeholders {nm} for the line number associated with the old version of the file and {np} for
the line number associated with the new version of the file. The placeholders support a subset of
the string formatting syntax documented here: https://doc.rust-lang.org/std/fmt/#formatting-parameters.
Specifically, you can use the alignment and width syntax.

For example, the default value of --line-numbers-left-format is '{nm:^4}⋮'. This means that the
left column should display the minus line number (nm), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (⋮).

Similarly, the default value of --line-numbers-right-format is '{np:^4}│'. This means that the
right column should display the plus line number (np), center-aligned, padded with spaces to a
width of 4 characters, followed by a unicode dividing-line character (│).

Use '<' for left-align, '^' for center-align, and '>' for right-align.


If something isn't working correctly, or you have a feature request, please open an issue at
https://github.com/dandavison/delta/issues.
```

## Delta configs used in screenshots

### Side-by-side view

https://github.com/vuejs/vue/commit/7ec4627902020cccd7b3f4fbc63e1b0d6b9798cd

```gitconfig
[delta]
    features = side-by-side line-numbers decorations
    syntax-theme = Dracula
    plus-style = syntax "#003800"
    minus-style = syntax "#3f0001"

[delta "decorations"]
    commit-decoration-style = bold yellow box ul
    file-style = bold yellow ul
    file-decoration-style = none
    hunk-header-decoration-style = cyan box ul

[delta "line-numbers"]
    line-numbers-left-style = cyan
    line-numbers-right-style = cyan
    line-numbers-minus-style = 124
    line-numbers-plus-style = 28
```
