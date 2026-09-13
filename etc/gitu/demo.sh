#!/usr/bin/env bash
#
#   ./etc/gitu/demo.sh [--light]
#   rm -rf /tmp/gitu-delta-demo   # clean up

set -euo pipefail

DEMO_DIR=${DEMO_DIR:-/tmp/gitu-delta-demo}
DELTA_REMOTE=${DELTA_REMOTE:-https://github.com/dandavison/delta}
DELTA_BRANCH=${DELTA_BRANCH:-osc-1717-metadata-extensions}
GITU_REMOTE=${GITU_REMOTE:-https://github.com/dandavison/gitu}
GITU_BRANCH=${GITU_BRANCH:-diff-renderer-and-pager}

HOME_DIR="$DEMO_DIR/home"
BIN_DIR="$DEMO_DIR/bin"
COLOR_MODE=dark

main() {
    parse_args "$@"
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

    summary | tee "$DEMO_DIR/summary"
    cd "$DEMO_DIR/delta"
    # Do not let the caller's delta, git, or pager settings override the demo.
    exec env \
        -u BAT_THEME \
        -u BAT_PAGER \
        -u DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES \
        -u DELTA_FEATURES \
        -u DELTA_NAVIGATE \
        -u DELTA_PAGER \
        -u GIT_CONFIG_COUNT \
        -u GIT_CONFIG_PARAMETERS \
        -u GIT_PAGER \
        -u PAGER \
        HOME="$HOME_DIR" \
        XDG_CONFIG_HOME="$HOME_DIR/.config" \
        GIT_CONFIG_GLOBAL="$HOME_DIR/.gitconfig" \
        GIT_CONFIG_NOSYSTEM=1 \
        CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
        RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" \
        PATH="$BIN_DIR:$PATH" \
        GITU_DELTA_DEMO="$DEMO_DIR" \
        "${SHELL:-/bin/sh}"
}

parse_args() {
    if [ "$#" -eq 0 ]; then
        return
    fi
    if [ "$#" -eq 1 ] && [ "$1" = --light ]; then
        COLOR_MODE=light
        return
    fi

    echo "usage: $0 [--light]" >&2
    exit 2
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
    {
        cat <<'EOF'
[user]
    name = Gitu Delta Demo
    email = demo@example.com

# git launches gitu in place of less on the commands whose output is a patch or
# a log, handing it what it was going to page. gitu structures it and renders it
# with delta. delta pages everything else, as it would without gitu.
# `GIT_PAGER='gitu --pager' git <cmd>` reaches for gitu on one command, and
# `GIT_PAGER=cat git <cmd>` gets the plain text back.
[core]
    pager = delta

[pager]
    diff = gitu --pager
    show = gitu --pager
    log = gitu --pager

# For `git add -p` and friends, which gitu is not in front of.
[interactive]
    diffFilter = delta --color-only

# `git rebase -i` hands its instruction list to gitu.
[sequence]
    editor = gitu sequence-editor

# delta as gitu launches it, and as git launches it directly. Anything
# delta can do to a diff is available here, because gitu maps each rendered row
# back to its patch line from the metadata delta emits, not from how the row
# looks. Kept plain, so that turning features on with `|` shows something.
[delta]
EOF
        printf '    %s = true\n' "$COLOR_MODE"
        cat <<'EOF'

    line-numbers = true
    file-style = bold yellow
    file-decoration-style = none
    hunk-header-decoration-style = blue box

    # Both forms of commit line: 'commit <sha>' as git writes it, and the
    # '▸ <sha>' of the log format below. delta states which commit a row belongs
    # to only for the lines this matches, and that is what gitu navigates a
    # paged log by: with just '^▸', 'git log' arrives as text with no commits in
    # it.
    commit-style = raw
    commit-regex = ^(commit |▸ )
    commit-decoration-style = blue ol

# A feature of one's own, to show that `|` offers those too.
[delta "colorful"]
    syntax-theme = Monokai Extended
    minus-style = syntax "#450a15"
    plus-style = syntax "#0b3d20"
EOF
    } > "$HOME_DIR/.gitconfig"
}

write_gitu_config() {
    cat > "$HOME_DIR/.config/gitu/config.toml" <<'EOF'
[general]
# delta speaks the OSC-1717 diff-line-metadata protocol, so it may restructure
# the diff as it likes — side-by-side, gutters, no +/- markers — and gitu still
# knows which patch line each row is, which is what staging needs.
diff_renderer.enabled = true
diff_renderer.command = ["delta", "--width", "{width}"]

# What `|` offers, to turn on and off while reading. They are passed to delta
# as an overlay on the features git config already sets, held for the session
# and never written to disk. Every `[delta "name"]` feature of your own is
# offered too, without being named here: `colorful` in the demo's git config.
diff_renderer.features = ["side-by-side", "hyperlinks", "diff-so-fancy"]

# gitu's own log view, rendered by delta. `{commit}` is substituted with the
# git format directives that state which commit each row belongs to, so a
# commit taking several rows is still one cursor stop.
log_renderer.enabled = true
log_renderer.command = [
  "sh", "-c",
  "git log --stat --date relative --color=always --format='{commit}%n%n▸ %h %C(blue)%an %C(blue)%ar%C(auto)%d%C(reset)%n%n    %C(green)%s%C(auto)' \"$@\" | delta --width {width}",
  "gitu",
]

# Files never worth reading. They are added as pathspecs to the git command the
# view is the output of, so `:` shows them and `_` takes them back.
hide = ["*.lock"]

# Stop the cursor on unchanged lines too: nothing can be staged there, but
# reading a patch is line by line.
visit_context_lines = true

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

search_match = { mods = "REVERSED" }

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

rebase_todo.reword = { fg = "blue", mods = "BOLD" }
rebase_todo.edit = { fg = "magenta", mods = "BOLD" }
rebase_todo.squash = { fg = "cyan", mods = "BOLD" }
rebase_todo.fixup = { fg = "yellow", mods = "BOLD" }
rebase_todo.drop = { fg = "red", mods = "BOLD" }

blame.line_num = { mods = "DIM" }
blame.code_line = { mods = "DIM" }
EOF
}

write_shell_config() {
    cat > "$HOME_DIR/.zshrc" <<'EOF'
# gitu + delta demo shell. `exit` leaves it.
PROMPT='%F{cyan}gitu-delta-demo%f %1~ %# '
summary() { cat "$GITU_DELTA_DEMO/summary"; }
EOF

    cat > "$HOME_DIR/.bashrc" <<'EOF'
# gitu + delta demo shell. `exit` leaves it.
PS1='\[\e[36m\]gitu-delta-demo\[\e[0m\] \W \$ '
summary() { cat "$GITU_DELTA_DEMO/summary"; }
EOF

    echo '. ~/.bashrc' > "$HOME_DIR/.bash_profile"
}

make_something_to_look_at() {
    local dir=$1
    git -C "$dir" reset -q 2>/dev/null || true
    git -C "$dir" checkout -- . 2>/dev/null || true

    printf '\n# A change to look at in the demo.\n' >> "$dir/README.md"
    prepend '// A change to look at in the demo.' "$dir/src/main.rs"

    prepend '// A change already staged, to unstage in the demo.' "$dir/src/cli.rs"
    git -C "$dir" add src/cli.rs
}

prepend() {
    local line=$1 file=$2
    {
        echo "$line"
        cat "$file"
    } > "$file.demo"
    mv "$file.demo" "$file"
}

summary() {
    cat <<EOF

    Everything, including the HOME this shell runs with, is under
    $DEMO_DIR
    Your own config is untouched; delete that directory to undo the lot.

    You are in the delta clone. README.md and src/main.rs have unstaged
    changes, src/cli.rs a staged one.
    delta uses $COLOR_MODE-terminal colors.

  What is new here. Try 'git diff', 'git show HEAD', 'git log -n 256',
  'git log -p -n 64', and 'gitu'.

    git launches gitu in place of less for diff, show and log; delta pages
    the rest.
    delta renders every diff, and it can still be staged from line by line.
    delta renders the log too, one cursor stop per commit however tall it is.
    :               edits the git command the view came from.
    - _             drops the file under the cursor; sets which files to show.
    U               more context: a number, or W for the whole function.
    |               turns a delta feature on or off in place.
    shift+up/down   selects a run of lines to stage as one patch.
    shift+tab       folds the patch to its headings; space and backspace page.
    r i             edits an interactive rebase todo; 'git rebase -i' does too.
    c f             picks the commit to fix up from the log.
    Which ops are offered follows the command, and every op re-runs it.
    Config hides '*.lock', so 'git show 1502986' opens without the churn.
    Output with no diff in it is rendered rather than refused:
    GIT_PAGER='gitu --pager' git grep -n OSC1717

    summary  prints this again.  exit  leaves the demo.

EOF
}

main "$@"
