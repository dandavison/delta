## Δ
  A syntax-highlighting pager for git.

  _(This project is at a very early stage, but please feel free to open issues.)_

## Installation

#### 0. Use a terminal that supports 24 bit colors
  See https://gist.github.com/XVilka/8346728

  For example, on macos, iTerm2 works but Terminal.app does not. If you are using tmux, see the section at the bottom.

#### 1. Install the Rust development environment.
  See https://www.rust-lang.org/tools/install.

#### 2. Clone this repo.

#### 3. Build the executable:
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

#### 4. Configure git to use delta:
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

#### Tmux 24bit color configuration
 If you're using tmux, it's worth checking that 24 bit color is  working correctly. For example, run a color test script like [this  one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh),  or the others listed at https://gist.github.com/XVilka/8346728. If  you do not see smooth color gradients, see the discussion at  [tmux#696](https://github.com/tmux/tmux/issues/696). The short  version is you need something like this in your `~/.tmux.conf`:
 ```
 set -ga terminal-overrides ",xterm-256color:Tc"
 ```
 and you may then  need to quit tmux completely for it to take effect.

## Credit
  https://github.com/trishume/syntect<br>
  https://github.com/sharkdp/bat
