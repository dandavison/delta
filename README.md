[![Build Status](https://travis-ci.com/dandavison/delta.svg?branch=master)](https://travis-ci.com/dandavison/delta)

## A syntax-highlighting pager for git

Delta brings language syntax highlighting, within-line insertion/deletion detection, and restructured diff output to git on the command line. Here's an example of `git show` output with git configured to use delta as its pager:

<img width=800px src="https://user-images.githubusercontent.com/52205/65245123-e8392000-dae3-11e9-88ef-fcccf6ade952.png" alt="image" />


## Features
|                                                | delta | git | diff-so-fancy | github/gitlab |
|------------------------------------------------|-------|-----|---------------|---------------|
| language syntax highlighting                   | ✅    | ❌  | ❌            | ✅             |
| within-line insertion/deletion detection       | ✅    | ❌  | ✅            | ✅             |
| multiple insertion/deletions detected per line | ✅    | ❌  | ❌            | ✅             |
| matching of unequal numbers of changed lines   | ✅    | ❌  | ❌            | ❌             |

## Installation

Executables: [Linux](https://github.com/dandavison/delta/releases/download/0.0.11/delta-0.0.11-x86_64-unknown-linux-musl.tar.gz) | [MacOS](https://github.com/dandavison/delta/releases/download/0.0.11/delta-0.0.11-x86_64-apple-darwin.tar.gz) | [Windows](https://github.com/dandavison/delta/releases/download/0.0.11/delta-0.0.11-x86_64-pc-windows-msvc.zip) | [All](https://github.com/dandavison/delta/releases)

Homebrew:
```sh
brew tap dandavison/delta https://github.com/dandavison/delta
brew install dandavison/delta/git-delta
...
brew upgrade git-delta
```



#### Configure git to use delta

```sh
git config --global core.pager "delta --dark"  # --light for light terminal backgrounds
```

Alternatively, you can edit your `.gitconfig` directly. Delta accepts many command line options to alter colors and other details of the output. An example is
```
[core]
    pager = delta --dark --plus-color="#012800" --minus-color="#340001" --theme="base16-ocean.dark"
```

All git commands that display diff output should now display syntax-highlighted output. For example:
  - `git diff`
  - `git show`
  - `git log -p`
  - `git stash show -p`


<br>


<br>

<table>
  <tr>
    <td>
      delta --dark (default)
    </td>
    <td>
      delta --light
    </td>
  </tr>
  <tr>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65245970-85488880-dae5-11e9-9fd2-d358071bcf7f.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/65246093-c93b8d80-dae5-11e9-8346-eb8bc0a89e75.png"
           alt="image" />
    </td>
  </tr>
</table>

<br>


## Supported languages and themes
Delta supports the same languages and themes as [bat](https://github.com/sharkdp/bat), included here as the files `assets/syntaxes.bin` and `assets/themes.bin`. Please see the [bat](https://github.com/sharkdp/bat) documentation for instructions on how to create new versions of these files.


## 24 bit color

Delta works best if your terminal application supports 24 bit colors. See https://gist.github.com/XVilka/8346728. For example, on MacOS, iTerm2 works but Terminal.app does not.

If you're using tmux, it's worth checking that 24 bit color is  working correctly. For example, run a color test script like [this  one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh),  or one of the others listed [here](https://gist.github.com/XVilka/8346728). If  you do not see smooth color gradients, see the discussion at  [tmux#696](https://github.com/tmux/tmux/issues/696). The short  version is you need something like this in your `~/.tmux.conf`:
```
set -ga terminal-overrides ",xterm-256color:Tc"
```
and you may then  need to quit tmux completely for it to take effect.

<br>

## Options
Here's the output of `delta --help`. To use these options, add them to the delta command line in your `.gitconfig` file.

```
USAGE:
    delta [FLAGS] [OPTIONS]

FLAGS:
        --compare-themes       Compare available syntax highlighting themes. To use this option, supply git diff output
                               to delta on standard input. For example: `git show --color=always | delta --compare-
                               themes`.
        --dark                 Use colors appropriate for a dark terminal background.  For more control, see --theme,
                               --plus-color, and --minus-color.
    -h, --help                 Prints help information
        --highlight-removed    Apply syntax highlighting to removed lines. The default is to apply syntax highlighting
                               to unchanged and new lines only.
        --light                Use colors appropriate for a light terminal background. For more control, see --theme,
                               --plus-color, and --minus-color.
        --list-languages       List supported languages and associated file extensions.
        --list-themes          List available syntax highlighting themes.
        --show-colors          Show the command-line arguments for the current colors.
    -V, --version              Prints version information

OPTIONS:
        --commit-style <commit_style>
            Formatting style for commit section of git output. Options are: plain, box. [default: plain]

        --file-style <file_style>
            Formatting style for file section of git output. Options are: plain, box, underline. [default: underline]

        --hunk-style <hunk_style>
            Formatting style for hunk section of git output. Options are: plain, box. [default: box]

        --max-line-distance <max_line_distance>
            The maximum distance between two lines for them to be inferred to be homologous. Homologous line pairs are
            highlighted according to the deletion and insertion operations transforming one into the other. [default:
            0.3]
        --minus-color <minus_color>                The background color (RGB hex) to use for removed lines.
        --minus-emph-color <minus_emph_color>
            The background color (RGB hex) to use for emphasized sections of removed lines.

        --plus-color <plus_color>                  The background color (RGB hex) to use for added lines.
        --plus-emph-color <plus_emph_color>
            The background color (RGB hex) to use for emphasized sections of added lines.

        --theme <theme>                            The syntax highlighting theme to use.
    -w, --width <width>
            The width (in characters) of the background color highlighting. By default, the width is the current
            terminal width. Use --width=variable to apply background colors to the end of each line, without right
            padding to equal width.
```

<br>


## Comparisons

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
      delta vs. diff-so-fancy
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





## Credit
  https://github.com/trishume/syntect<br>
  https://github.com/sharkdp/bat<br>
  https://github.com/so-fancy/diff-so-fancy
