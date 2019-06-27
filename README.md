## Δ
  A syntax-highlighting pager for git.

  _(This project is at a very early stage, but please feel free to open issues.)_

<img width=500px style="border: 1px solid black"
    src="https://user-images.githubusercontent.com/52205/60282969-230d2c80-98d6-11e9-8656-09073c8a0985.png"
    alt="image" />

## Usage
```
delta 0.1.0
Adds language-specific syntax highlighting to git output. Use 'delta | less -R' as core.pager in .gitconfig

USAGE:
    delta [FLAGS] [OPTIONS]

FLAGS:
        --dark       Use diff highlighting colors appropriate for a dark terminal background.
    -h, --help       Prints help information
        --light      Use diff highlighting colors appropriate for a light terminal background. This is the default.
    -V, --version    Prints version information

OPTIONS:
        --minus-color <minus_color>    The background color (RGB hex) to use for removed lines. The default is "#ffd0d0"
                                       if you are using --light, and "#3f0001" if you are using --dark.
        --plus-color <plus_color>      The background color (RGB hex) to use for added lines. The default is "#d0ffd0"
                                       if you are using --light, and "#013B01" if you are using --dark.
        --theme <theme>                The syntax highlighting theme to use. Options are Light: ("InspiredGitHub",
                                       "Solarized (light)", "base16-ocean.light"), Dark: ("Solarized, (dark)", "base16-
                                       eighties.dark", "base16-mocha.dark", "base16-ocean.dark").
    -w, --width <width>                The width (in characters) of the diff highlighting. By default, the highlighting
                                       extends to the last character on each line
```

## Installation

1. **Install the Rust development environment:**<br>
    See https://www.rust-lang.org/tools/install.

2. **Clone this repo**<br>

3. **Build the executable:**<br>
    ```sh
    cd delta
    cargo build
    ```
    This creates an executable inside the repo at `target/debug/delta`. Make sure this executable is found on your shell
    `$PATH`.

    For example, if `~/bin` is in your `$PATH`, then you could use a symlink:
    ```
    cd ~/bin
    ln -s /path/to/delta/target/debug/delta delta
    ```

    Alternatively, you can ignore `$PATH` and use
    `/path/to/delta/target/debug/delta | less -R` in the next step.

4. **Configure git to use delta:**<br>
    Edit your `~/.gitconfig`:
    ```
    [core]
        pager = delta | less -R
    ```
    Alternatively, run this command:
    ```
    git config --global core.pager 'delta | less -R'
    ```

All git commands that display diff output should now display syntax-highlighted output. For example:
  - `git diff`
  - `git show`
  - `git log -p`
  - `git stash show -p`

## 24 bit color

  delta works best if your terminal application supports 24 bit colors. See https://gist.github.com/XVilka/8346728. For example, on macos, iTerm2 works but Terminal.app does not.

  If you're using tmux, it's worth checking that 24 bit color is  working correctly. For example, run a color test script like [this  one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh),  or the others listed at https://gist.github.com/XVilka/8346728. If  you do not see smooth color gradients, see the discussion at  [tmux#696](https://github.com/tmux/tmux/issues/696). The short  version is you need something like this in your `~/.tmux.conf`:
  ```
  set -ga terminal-overrides ",xterm-256color:Tc"
  ```
  and you may then  need to quit tmux completely for it to take effect.

## Credit
  https://github.com/trishume/syntect<br>
  https://github.com/sharkdp/bat
