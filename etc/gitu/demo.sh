#!/usr/bin/env bash
#
#   ./etc/gitu/demo.sh
#   rm -rf /tmp/gitu-delta-demo   # clean up

set -euo pipefail

DEMO_DIR=${DEMO_DIR:-/tmp/gitu-delta-demo}
DELTA_REMOTE=${DELTA_REMOTE:-https://github.com/dandavison/delta}
DELTA_BRANCH=${DELTA_BRANCH:-osc-1717-metadata-extensions}
GITU_REMOTE=${GITU_REMOTE:-https://github.com/dandavison/gitu}
GITU_BRANCH=${GITU_BRANCH:-diff-colorizer-w-extra-features}

HOME_DIR="$DEMO_DIR/home"
BIN_DIR="$DEMO_DIR/bin"

main() {
    preflight
    mkdir -p "$HOME_DIR/.config/gitu" "$BIN_DIR"

    clone "$DELTA_REMOTE" "$DELTA_BRANCH" "$DEMO_DIR/delta"
    clone "$GITU_REMOTE" "$GITU_BRANCH" "$DEMO_DIR/gitu"

    echo "==> Building (the first time takes a few minutes)"
    build "$DEMO_DIR/delta" delta
    build "$DEMO_DIR/gitu" gitu

    write_git_config
    write_gitu_config
    write_shell_config
    make_something_to_look_at "$DEMO_DIR/delta"

    banner
    cd "$DEMO_DIR/delta"
    exec env \
        HOME="$HOME_DIR" \
        XDG_CONFIG_HOME="$HOME_DIR/.config" \
        GIT_CONFIG_GLOBAL="$HOME_DIR/.gitconfig" \
        GIT_CONFIG_NOSYSTEM=1 \
        CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
        RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" \
        PATH="$BIN_DIR:$PATH" \
        GITU_DELTA_DEMO=1 \
        "${SHELL:-/bin/sh}"
}

preflight() {
    local missing=0

    for tool in git cargo rustc cc; do
        command -v "$tool" > /dev/null || {
            echo "!!! $tool is not on PATH" >&2
            missing=1
        }
    done

    local version
    version=$(rustc --version 2>/dev/null | cut -d' ' -f2 || echo 0.0.0)
    if [ "$(printf '%s\n1.88.0\n' "$version" | sort -V | head -n1)" != "1.88.0" ]; then
        echo "!!! rustc $version is too old; gitu needs 1.88 or newer" >&2
        missing=1
    fi

    if [ "$missing" = 1 ]; then
        echo "    (on Debian/Ubuntu: apt install build-essential; for rust: rustup update)" >&2
        exit 1
    fi

    echo "==> The clones and their builds need about 2G under $DEMO_DIR"
}

clone() {
    local remote=$1 branch=$2 dir=$3
    if [ -d "$dir" ]; then
        echo "==> $dir is already there"
        return
    fi
    echo "==> Cloning $remote ($branch)"
    git clone --single-branch --branch "$branch" "$remote" "$dir"
}

build() {
    local dir=$1 name=$2
    echo "==> $name"
    cargo build --release --manifest-path "$dir/Cargo.toml" --bin "$name"
    ln -sf "$dir/target/release/$name" "$BIN_DIR/$name"
}

write_git_config() {
    cat > "$HOME_DIR/.gitconfig" <<'EOF'
[user]
    name = Gitu Delta Demo
    email = demo@example.com

[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    side-by-side = true
    line-numbers = true
    navigate = true
    file-style = bold yellow
    file-decoration-style = none
    hunk-header-decoration-style = blue box

    commit-style = raw
    commit-regex = ^▸
    commit-decoration-style = blue ol

# `git rebase -i` hands its instruction list to gitu.
[sequence]
    editor = gitu sequence-editor
EOF
}

write_gitu_config() {
    cat > "$HOME_DIR/.config/gitu/config.toml" <<'EOF'
[general]
diff_colorizer.enabled = true
diff_colorizer.command = ["delta", "--width", "{width}"]

log_renderer.enabled = true
log_renderer.command = [
  "sh", "-c",
  "git log --stat --date relative --color=always --format='{commit}%n%n▸ %h %C(blue)%an %C(blue)%ar%C(auto)%d%C(reset)%n%n    %C(green)%s%C(auto)' \"$@\" | delta --width {width}",
  "gitu",
]

# Colours, so gitu's own furniture sits with what delta draws. Named colours
# rather than hex, to stay legible on a light or a dark terminal.
[style]
separator = { mods = "DIM" }

info_msg = { fg = "green", mods = "BOLD" }
error_msg = { fg = "red", mods = "BOLD" }
command = { fg = "blue", mods = "BOLD" }

menu.heading = { fg = "blue", mods = "BOLD" }
menu.key = { fg = "magenta" }
menu.active_arg = { fg = "yellow", mods = "BOLD" }
menu.inactive_arg = {}

prompt = { fg = "blue", mods = "DIM" }

section_header = { mods = "BOLD" }
file_header = { fg = "cyan" }
hunk_header = { fg = "blue" }

cursor = { symbol = "▌", fg = "blue" }
selection_bar = { symbol = "▌", fg = "blue", mods = "DIM" }
selection_line = { mods = "BOLD" }
selection_area = {}

picker.prompt = { fg = "blue" }
picker.info = { mods = "DIM" }
picker.selection_line = { mods = "BOLD" }
picker.matched = { fg = "magenta", mods = "BOLD" }

hash = { fg = "blue" }
branch = { fg = "green" }
remote = { fg = "yellow" }
tag = { fg = "blue" }
rebase_todo_action = { fg = "magenta" }

blame.line_num = { mods = "DIM" }
blame.code_line = { mods = "DIM" }
EOF
}

write_shell_config() {
    cat > "$HOME_DIR/.zshrc" <<'EOF'
# gitu + delta demo shell. `exit` leaves it.
PROMPT='%F{cyan}gitu-delta-demo%f %1~ %# '
EOF

    cat > "$HOME_DIR/.bashrc" <<'EOF'
# gitu + delta demo shell. `exit` leaves it.
PS1='\[\e[36m\]gitu-delta-demo\[\e[0m\] \W \$ '
EOF

    echo '. ~/.bashrc' > "$HOME_DIR/.bash_profile"
}

make_something_to_look_at() {
    local dir=$1
    git -C "$dir" checkout -- . 2>/dev/null || true

    printf '\n# A change to look at in the demo.\n' >> "$dir/README.md"

    local main="$dir/src/main.rs"
    {
        echo '// A change to look at in the demo.'
        cat "$main"
    } > "$main.demo"
    mv "$main.demo" "$main"
}

banner() {
    cat <<EOF

    Everything is under $DEMO_DIR, including the HOME this shell runs with.
    Your own config is untouched; delete that directory to undo the lot.

    You are in the delta clone, which has unstaged changes to look at.

      gitu            		diffs and log rendered by delta
	      r i             	edit an interactive rebase in gitu
    	  c f             	pick the commit to fix up from the log
	      h               	list what the keys do on those screens
      git rebase -i HEAD~6  opens the instruction list in gitu too

    exit  leaves the demo.

EOF
}

main "$@"
