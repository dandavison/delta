## Δ
  A syntax-highlighting pager for git.

  _(This project is at a very early stage, but please feel free to open issues.)_

## Installation

#### 1. Install the Rust development environment.
  See https://www.rust-lang.org/tools/install.

#### 2. Clone this repo.

#### 3. Build the executable:
  ```sh
  cd delta
  cargo build
  ```
  This creates an executable inside the repo at `target/debug/delta`. Copy or symlink this executable to somewhere on your shell
  `$PATH`.

  If you're not sure what that means, ignore it, and use
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

## Credit
  https://github.com/trishume/syntect<br>
  https://github.com/sharkdp/bat
