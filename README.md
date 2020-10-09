[![Build Status](https://travis-ci.com/dandavison/delta.svg?branch=master)](https://travis-ci.com/dandavison/delta)
[![Gitter](https://badges.gitter.im/dandavison-delta/community.svg)](https://gitter.im/dandavison-delta/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

## A viewer for git and diff output

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

#### Delta's main features are:

- Language syntax highlighting with color themes
- Within-line highlights based on a Levenshtein edit inference algorithm
- Git style strings (foreground color, background color, font attributes) are supported for >20 stylable elements
- Side-by-side view
- Line numbering
- `diff-highlight` and `diff-so-fancy` emulation modes
- Stylable box/line decorations to draw attention to commit, file and hunk header sections.
- Support for Git's `--color-moved` feature.
- Code can be copied directly from the diff (`-/+` markers are removed by default).
- `n` and `N` keybindings to move between files in large diffs, and between diffs in `log -p` views (`--navigate`)

The most convenient way to configure delta is with a `[delta]` section in `~/.gitconfig`. Here's a quick example:

<sub>

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    features = side-by-side line-numbers decorations
    whitespace-error-style = 22 reverse

[delta "decorations"]
    commit-decoration-style = bold yellow box ul
    file-style = bold yellow ul
    file-decoration-style = none
```

</sub>

To quickly change delta configuration on-the-fly, use the `git config --global` command. For example

```bash
git config --global delta.side-by-side true
```

Contents
========

* [Installation](#installation)
* [Configuration](#configuration)
* [Usage](#usage)
   * [Choosing colors (styles)](#choosing-colors-styles)
   * [Line numbers](#line-numbers)
   * [Side-by-side view](#side-by-side-view)
   * [Custom features](#custom-features)
   * [diff-highlight and diff-so-fancy emulation](#diff-highlight-and-diff-so-fancy-emulation)
   * [--color-moved support](#--color-moved-support)
   * [Navigation keybindings for large diffs](#navigation-keybindings-for-large-diffs)
   * [24 bit color (truecolor)](#24-bit-color-truecolor)
   * [Using Delta on Windows](#using-delta-on-windows)
   * [Mouse scrolling](#mouse-scrolling)
   * [Using Delta with Magit](#using-delta-with-magit)
   * [Supported languages and themes](#supported-languages-and-themes)
* [Comparisons with other tools](#comparisons-with-other-tools)
* [Build delta from source](#build-delta-from-source)
* [Related projects](#related-projects)
   * [Used by delta](#used-by-delta)
   * [Using delta](#using-delta)
   * [Similar projects](#similar-projects)
* [Full --help output](#full---help-output)
* [Delta configs used in screenshots](#delta-configs-used-in-screenshots)
   * [Side-by-side view](#side-by-side-view-1)


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

**By default, delta restructures the git output slightly to make the hunk markers human-readable:**

<br>
<table><tr><td>
  <img width=650px src="https://user-images.githubusercontent.com/52205/81059276-254cf980-8e9e-11ea-95c3-8b757a4c11b5.png" alt="image" />
</td></tr></table>


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

|                                                | delta | git | [diff-so-fancy] /<br>[diff-highlight] | github/gitlab |
|------------------------------------------------|-------|-----|---------------------------------------|---------------|
| language syntax highlighting                   | ✅    | ❌  | ❌                                    | ✅           |
| within-line insertion/deletion detection       | ✅    | ❌  | ✅                                    | ✅           |
| multiple insertion/deletions detected per line | ✅    | ❌  | ❌                                    | ✅           |
| matching of unequal numbers of changed lines   | ✅    | ❌  | ❌                                    | ❌           |
| independently stylable elements                | ✅    | ✅  | ✅                                    | ❌           |
| line numbering                                 | ✅    | ❌  | ❌                                    | ✅           |
| side-by-side view                              | ✅    | ❌  | ❌                                    | ✅           |

In addition, delta handles traditional unified diff output.

[diff-so-fancy]: https://github.com/so-fancy/diff-so-fancy
[diff-highlight]: https://github.com/git/git/tree/master/contrib/diff-highlight

## Installation

You can download an executable for your system: [Linux](https://github.com/dandavison/delta/releases/download/0.4.1/delta-0.4.1-x86_64-unknown-linux-musl.tar.gz) | [MacOS](https://github.com/dandavison/delta/releases/download/0.4.1/delta-0.4.1-x86_64-apple-darwin.tar.gz) | [Windows](https://github.com/dandavison/delta/releases/download/0.4.1/delta-0.4.1-x86_64-pc-windows-gnu.zip) | [All](https://github.com/dandavison/delta/releases)

Alternatively, delta is available in the following package managers:

<table>
  <tr>
    <td>Arch Linux AUR<br>
        (<a href="https://aur.archlinux.org/packages/git-delta">build from source</a>)</td>
    <td><code>yay -S git-delta</code>
        <br>or<br>
        <code>git clone https://aur.archlinux.org/git-delta.git</code><br>
        <code>cd git-delta</code><br>
        <code>makepkg -csri</code></td>
  </tr>
  <tr>
    <td>Arch Linux AUR<br>
        (<a href="https://aur.archlinux.org/packages/git-delta-bin">binary, no compilation required</a>)</td>
    <td><code>yay -S git-delta-bin</code>
        <br>or<br>
        <code>git clone https://aur.archlinux.org/git-delta-bin.git</code><br>
        <code>cd git-delta-bin</code><br>
        <code>makepkg -si</code></td>
  </tr>
  <tr>
    <td>Debian</td>
    <td><br>.deb files are on the <a href="https://github.com/dandavison/delta/releases">releases</a> page and at <a href="https://github.com/barnumbirr/delta-debian/releases">barnumbirr/delta-debian</a><br>
    <code>dpkg -i file.deb</code></td>
  </tr>
  <tr>
    <td>Fedora</td>
    <td><code>dnf install git-delta</code></td>
  </tr>
  <tr>
    <td>FreeBSD</td>
    <td><code>pkg install git-delta</code></td>
  </tr>
  <tr>
    <td>Homebrew</td>
    <td><code>brew install git-delta</code></td>
  </tr>
  <tr>
    <td>MacPorts</td>
    <td><code>port install git-delta</code></td>
  </tr>
  <tr>
    <td>Nix</td>
    <td><code>nix-env -iA nixpkgs.gitAndTools.delta</code>
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
    <td>Cargo</td>
    <td><code>cargo install git-delta</code></td>
  </tr>
</table>

## Configuration

Set delta to be git's pager in your `.gitconfig`. Delta has many options to alter colors and other details of the output. An example is
```gitconfig
[core]
    pager = delta

[delta]
    plus-color = "#012800"
    minus-color = "#340001"
    syntax-theme = Monokai Extended

[interactive]
    diffFilter = delta --color-only
```

Note that delta color argument values in ~/.gitconfig should be in double quotes, like `--minus-color="#340001"`. For theme names and other values, do not use quotes as they will be passed on to delta, like `theme = Monokai Extended`.

All git commands that display diff output should now display syntax-highlighted output. For example:
  - `git diff`
  - `git show`
  - `git log -p`
  - `git stash show -p`
  - `git reflog -p`
  - `git add -p`

For Mercurial, you can add delta, with its command line options, to the `[pager]` section of `.hgrc`.

Delta also handles unified diff output, and can be used as an alternative way of invoking `diff -u`. The following two commands do the same thing:
```
delta a.txt b.txt

diff -u a.txt b.txt | delta
```

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

Wide lines in the left or right panel are currently truncated. If the truncation is a problem, one approach is to set the width of Delta's output to be  larger than your terminal (e.g. `delta --width 250`) and ensure that `less` doesn't wrap long lines (e.g. `export LESS=-RS`); then one can scroll right to view the full content. (Another approach is to decrease font size in your terminal.)

### Custom features

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

In order to support this feature, Delta has to look at the raw colors it receives in a line from Git, and use them to judge whether it is a typical removed/added line, or a specially-colored moved line. This should just work. However, if it causes problems, the behavior can be disabled using

```gitconfig
[delta]
    inspect-raw-lines = false
```

### Navigation keybindings for large diffs

Use the `navigate` feature to activate navigation keybindings. In this mode, pressing `n` will jump forward to the next file in the diff, and `N` will jump backwards. If you are viewing multiple commits (e.g. via `git log -p`) then navigation will also visit commit boundaries.

The recommended way to use `navigate` is to activate it only when needed, for example by using the environment variable `DELTA_NAVIGATE`:

```bash
DELTA_NAVIGATE=1 git diff
```

Please note that if the environment variable is set to _anything at all_ (even `"false"` or `"0"` or `""`) then that is interpreted as true. The above command sets the environment variable in the child process only, so it has no permanent effect on your shell session. But if, for whatever reason, you do have it set in your shell environment then to deactivate it you must _unset_ the environment variable (e.g. using `unset DELTA_NAVIGATE`). You cannot deactivate it by assigning a value to it.

An alternative is to mutate your git config file from the command line:

```bash
git config --global delta.navigate true
```

The reason that `navigate` should not be used all the time is that Delta uses `less` as its pager, and the `navigate` feature works by doing `less --pattern <regex-matching-file-and-commit-lines>`. When the git output does not contain file/commit diff lines, `less --pattern` behaves unhelpfully (see [#234](https://github.com/dandavison/delta/issues/234), [#237](https://github.com/dandavison/delta/issues/2)).


### 24 bit color (truecolor)

Delta looks best if your terminal application supports 24 bit colors. See https://gist.github.com/XVilka/8346728. For example, on MacOS, iTerm2 supports 24-bit colors but Terminal.app does not.

If your terminal application does not support 24-bit color, delta will still work, by automatically choosing the closest color from those available. See the `Colors` section of the help output below.

If you're using tmux, it's worth checking that 24 bit color is  working correctly. For example, run a color test script like [this  one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh),  or one of the others listed [here](https://gist.github.com/XVilka/8346728). If  you do not see smooth color gradients, see the discussion at  [tmux#696](https://github.com/tmux/tmux/issues/696). The short  version is you need something like this in your `~/.tmux.conf`:
```
set -ga terminal-overrides ",xterm-256color:Tc"
```
and you may then  need to quit tmux completely for it to take effect.


### Using Delta on Windows

Delta works on Windows. However, the `less.exe` installed with git has some bugs: you will see incorrect colors and other problems. A patched version of `less.exe` and instructions for installing can be found [here](https://github.com/lzybkr/less/releases/tag/fix_windows_vt).


### Mouse scrolling

If mouse scrolling isn't working correctly, try setting your `BAT_PAGER` environment variable to (at least) `less -R` .
See [issue #58](https://github.com/dandavison/delta/issues/58) and [bat README / "Using a different pager"](https://github.com/sharkdp/bat#using-a-different-pager).


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

```
cargo build --release
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
- [ms-jpq/sad](https://github.com/ms-jpq/sad)
- [wfxr/forgit](https://github.com/wfxr/forgit)

### Similar projects
- [da-x/fancydiff](https://github.com/da-x/fancydiff)
- [git/diff-highlight](https://github.com/git/git/tree/master/contrib/diff-highlight)
- [jeffkaufman/icdiff](https://github.com/jeffkaufman/icdiff)
- [kovidgoyal/kitty-diff](https://sw.kovidgoyal.net/kitty/kittens/diff.html)
- [mookid/diffr](https://github.com/mookid/diffr)
- [so-fancy/diff-so-fancy](https://github.com/so-fancy/diff-so-fancy)


## Full --help output

```
delta 0.4.3
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
        --hyperlinks                 Render commit hashes, file names, and line numbers as hyperlinks, according to the
                                     hyperlink spec for terminal emulators:
                                     https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda. By default,
                                     file names and line numbers link to the local file using a file URL, whereas commit
                                     hashes link to the commit in GitHub, if the remote repository is hosted by GitHub.
                                     See --hyperlinks-file-link-format for full control over the file URLs emitted.
                                     Hyperlinks are supported by several common terminal emulators. However, they are
                                     not yet supported by less, so they will not work in delta unless you install a
                                     patched fork of less (see https://github.com/dandavison/less). If you use tmux,
                                     then you will also need a patched fork of tmux (see
                                     https://github.com/dandavison/tmux)
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
                                     for the demo. For example: `git show --color=always | delta --show-syntax-themes`
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
            ~/.gitconfig. See FEATURES section [default: ]
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
        --file-style <file-style>
            Style (foreground, background, attributes) for the file section. See STYLES section. The style 'omit' can be
            used to remove the file section from the output [default: blue]
        --file-decoration-style <file-decoration-style>
            Style (foreground, background, attributes) for the file decoration. See STYLES section. The style string
            should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the combination
            'ul ol' [default: blue ul]
        --hyperlinks-file-link-format <hyperlinks-file-link-format>
            Format string for file hyperlinks. The placeholders "{path}" and "{line}" will be replaced by the absolute
            file path and the line number, respectively. The default value of this option creates hyperlinks using
            standard file URLs; your operating system should open these in the application registered for that file
            type. However, these do not make use of the line number. In order for the link to open the file at the
            correct line number, you could use a custom URL format such as "file-line://{path}:{line}" and register an
            application to handle the custom "file-line" URL scheme by opening the file in your editor/IDE at the
            indicated line number. See https://github.com/dandavison/open-in-editor for an example [default: file://{path}]
        --hunk-header-style <hunk-header-style>
            Style (foreground, background, attributes) for the hunk-header. See STYLES section. The style 'omit' can be
            used to remove the hunk header section from the output [default: syntax]
        --hunk-header-decoration-style <hunk-header-decoration-style>
            Style (foreground, background, attributes) for the hunk-header decoration. See STYLES section. The style
            string should contain one of the special attributes 'box', 'ul' (underline), 'ol' (overline), or the
            combination 'ul ol' [default: blue box]
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
        --file-modified-label <file-modified-label>
            Text to display in front of a modified file path [default: ]

        --file-removed-label <file-removed-label>
            Text to display in front of a removed file path [default: removed:]

        --file-added-label <file-added-label>
            Text to display in front of a added file path [default: added:]

        --file-renamed-label <file-renamed-label>
            Text to display in front of a renamed file path [default: renamed:]

        --max-line-length <max-line-length>
            Truncate lines longer than this. To prevent any truncation, set to zero. Note that syntax-highlighting very
            long lines (e.g. minified .js) will be very slow if they are not truncated [default: 512]
    -w, --width <width>
            The width of underline/overline decorations. Use --width=variable to extend decorations and background
            colors to the end of the text only. Otherwise background colors extend to the full terminal width
        --tabs <tab-width>
            The number of spaces to replace tab characters with. Use --tabs=0 to pass tab characters through directly,
            but note that in that case delta will calculate line widths assuming tabs occupy one character's width on
            the screen: if your terminal renders tabs as more than than one character wide then delta's output will look
            incorrect [default: 4]
        --24-bit-color <true-color>
            Whether to emit 24-bit ("true color") RGB color codes. Options are auto, always, and never. "auto" means
            that delta will emit 24-bit color codes if the environment variable COLORTERM has the value "truecolor" or
            "24bit". If your terminal application (the application you use to enter commands at a shell prompt) supports
            24 bit colors, then it probably already sets this environment variable, in which case you don't need to do
            anything [default: auto]
        --inspect-raw-lines <inspect-raw-lines>
            Whether to examine ANSI color escape sequences in raw lines received from Git and handle lines colored in
            certain ways specially. This is on by default: it is how Delta supports Git's --color-moved feature. Set
            this to "false" to disable this behavior [default: true]
        --paging <paging-mode>
            Whether to use a pager when displaying output. Options are: auto, always, and never. The default pager is
            `less`: this can be altered by setting the environment variables BAT_PAGER or PAGER (BAT_PAGER has priority)
            [default: auto]
        --minus-empty-line-marker-style <minus-empty-line-marker-style>
            Style for removed empty line marker (used only if --minus-style has no background color) [default: normal auto]
        --plus-empty-line-marker-style <plus-empty-line-marker-style>
            Style for added empty line marker (used only if --plus-style has no background color) [default: normal auto]
        --whitespace-error-style <whitespace-error-style>
            Style for whitespace errors. Defaults to color.diff.whitespace if that is set in git config, or else
            'magenta reverse' [default: auto auto]
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
    number = true
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

There are three ways to specify a color (this section applies to foreground and background colors
within a style string):

1. RGB hex code

   An example of using an RGB hex code is:
   --file-style="#0e7c0e"

2. ANSI color name

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

3. ANSI color number

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
