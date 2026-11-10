# TUI mode

`delta --tui` is an interactive git client and pager whose diffs are rendered by delta. It is based on [gitu](https://github.com/altsem/gitu) by altsem, a git client inspired by Magit, and is embedded via a [fork](https://github.com/dandavison/gitu). It is experimental. `--gitu` is an alias of `--tui`.

```sh
delta --tui                            # status view of the current repository
delta --tui show <rev>                 # one commit
delta --tui blame <file>               # blame of a file
delta --tui rebase <upstream>          # edit an interactive rebase's instruction list
git diff | delta --tui --pager         # browse a patch
```

Diffs are rendered by the running delta binary, configured as usual by your `[delta]` git config. The TUI maps each rendered row back to its line in the patch, so side-by-side, line numbers etc. do not prevent staging individual lines.

## Trying it out

```sh
GIT_PAGER='delta --tui --pager' git log
```

`ctrl+down` / `ctrl+up` move from commit to commit, and `enter` opens one; `q` goes back.

```sh
GIT_PAGER='delta --tui --pager' git diff
```

`shift+tab` unfolds every file, and folds them again; `tab` folds the file or hunk under the cursor. `S` toggles side-by-side, `L` line numbers, and `T` steps through color themes. `?` lists the commands for the row under the cursor, and `q` quits. To use the TUI as git's pager all the time, see Git config below.

## Side-by-side, line numbers and +/- markers

`S` toggles side-by-side, `L` line numbers, and `+` keeps the `+`/`-` at the start of each line, for copying a diff as a patch (`plus-minus-markers`, which is delta's `keep-plus-minus-markers`). Each turns the feature on or off whether or not your delta config turns it on. This holds for the session; nothing is written to your `[delta]` config.

`-` lists these and any other delta features you configure (see TUI config below); `enter` turns the selected one on or off. In the list, typing a key makes it the selected feature's key, and typing the feature's own key removes it. The TUI saves the keys in git config. Or set them yourself:

```gitconfig
# These are the defaults. Keys set here replace all of them. A key that is
# already bound is refused.
[delta-tui "general.diff-renderer.feature-keys"]
    side-by-side = S
    line-numbers = L
    plus-minus-markers = +
```

## Color themes

`T` enters theme mode. A row at the bottom of the screen names a theme, and the screen is drawn with it. `down` / `up` (or `tab` / `shift+tab`) step through the themes; a digit selects the theme of that number, 0 being no theme (your own config as it is), so that two can be compared. `ctrl+l` shows the list of every theme, where typing searches. Feature keys such as `S` still work, to see a theme side-by-side. `enter` keeps the theme and saves it in git config; `esc` or `q` puts back the one you had.

A theme is overlaid on your delta config, after any features you turned on. The themes are those in your git config, as `delta --show-themes` finds them: features that set `dark` or `light`. To have delta's collection of themes, include its `themes.gitconfig` (see [Custom themes](./custom-themes.md)). Themes for the other background, light or dark, are only in the list. delta's builtin features can be offered as themes too:

```gitconfig
[delta-tui "general.diff-renderer"]
    builtin-themes = diff-so-fancy
    # Set by theme mode; or set it yourself.
    theme = my-theme
```

## Git config

```gitconfig
[core]
    pager = delta
[pager]
    diff = delta --tui --pager
    show = delta --tui --pager
    log = delta --tui --pager
[sequence]
    editor = delta --tui sequence-editor
```

With this, `git diff`, `git show`, `git log` and `git rebase -i` open in the TUI. The TUI finds the git command that started it and runs it again itself, in the same directory, after each change you make. git aliases are recognised.

With `--quit-if-one-screen` (as less's), output that fits on the screen is printed and the TUI exits; that also leaves nothing to stage from, so it suits `log` better than `diff`.

## Log and commit formats

`git log` and `git show` typed at the command line use git's `format.pretty`. The `git log` and `git show` that the TUI runs itself (`l l`, `enter` on a commit) take their arguments from `log-args` and `show-args` in the TUI config. A commit is one cursor stop only if its first row matches delta's `commit-regex`, so a format of your own needs a `commit-regex` that matches it. For example:

```gitconfig
[delta]
    commit-regex = ^(▸|commit )
[pretty]
    commit-format = tformat:%n%n▸ %h %C(blue)%an %C(blue)%ar%C(auto)%d%C(reset)%n%n    %C(green)%s%C(auto)
    commit-format-full = tformat:%n%n▸ %h %C(blue)%an %C(blue)%ar%C(auto)%d%C(reset)%n%n%w(0,4,4)%C(green)%s%C(reset)%n%+b
[delta-tui]
    log-args = --stat
    log-args = --date=relative
    log-args = --pretty=commit-format
    show-args = --pretty=commit-format-full
```

and, for `git log` typed at the command line, a shell alias:

```sh
alias gl='git log --stat --date=relative --pretty=commit-format'
```

## TUI config

The TUI is configured in git config's `delta-tui` section, beside your `[delta]` config. It also reads a TOML file, `~/.config/gitu/config.toml` (or `--config <file>`); where both set something, git config wins. The diff renderer is always delta, whatever either says. See the [default config](https://github.com/dandavison/gitu/blob/main/src/default_config.toml) for every setting and binding.

A setting's name in git config is its name in the TOML file with dashes for underscores. Settings under `[general]` go in `[delta-tui]`; others go in a subsection named by their path: `diff_renderer.features` is `features` in `[delta-tui "general.diff-renderer"]`, and `[style] hunk_header` is `hunk-header` in `[delta-tui "style"]`. A list is the key given once per item. A boolean or number is read as git reads one (`yes`, `4m`). A style is written as delta writes one: attributes, then a foreground and a background color. The bindings of menu arguments (such as `commit_menu.--all`) can be set only in the TOML file, since git config names cannot start with `-`.

Values below are the defaults except where marked as an example.

### delta features

```gitconfig
[delta-tui "general.diff-renderer"]
    # Features offered by `-`. `*` and `?` match the `[delta "name"]` features
    # in your git config. Example (the default omits "my-*"):
    features = side-by-side
    features = line-numbers
    features = plus-minus-markers
    features = my-*
```

### Other settings

```gitconfig
[delta-tui]
    # Arguments to the `git log` run by `l l` and `l o`, also used for the
    # commits in the rebase instruction list. See "Log and commit formats" above.
    # Example:
    log-args = --stat
    # Arguments to the `git show` run by enter on a commit and `delta --tui
    # show`. Example:
    show-args = --stat
    # Files left out of diff and log views, as git pathspecs. Example:
    hide = *.pb.go
    hide = vendor
    # What heads each file: delta's file header ("delta"), or the file's
    # `git diff --stat` row, counted by the TUI ("stat").
    file-header = delta
    # Open a patch of more than one file with its files folded, if it does not
    # fit on one screen.
    start-folded = false
    # Let the cursor stop on unchanged lines.
    visit-context-lines = true
    # Open the status view on the first unstaged file, or else the first staged one.
    status-cursor-on-changes = true
    # false: show only the branch name at the top of the status view.
    verbose-branch-status = true
    # Status view sections that start folded: "branch_status", "untracked",
    # "recent_commits". Example:
    collapsed-sections = branch_status
    # Commits in the status view's Recent commits; 0 removes the section.
    recent-commits-limit = 10
    # Stashes in the status view.
    stash-list-limit = 10
    # Ask before discarding: "line" (always), "hunk", "file" or "never".
    confirm-discard = line
    # Read at most this much of git's output; the rest is dropped.
    max-input-bytes = 2000000
    mouse-support = false
    mouse-scroll-lines = 3
[delta-tui "general.confirm-quit"]
    enabled = false
[delta-tui "general.always-show-help"]
    # Always show the key help at the bottom of the screen.
    enabled = false
[delta-tui "general.refresh-on-file-change"]
    # Refresh the status view when files change.
    enabled = true

# Each binding is the key given once per key; the binding names are in the
# table under Keybindings below. Examples:
[delta-tui "bindings.root"]
    quit = q
    quit = esc
    quit = left
[delta-tui "bindings.rebase-todo"]
    rebase-todo-drop = d
    rebase-todo-drop = ctrl+k

# Colors are a name ("blue"), a hex value ("#0086b3") or a 256-color index.
# Examples:
[delta-tui "style"]
    cursor = blue
    selection-line = bold
    section-header = yellow
[delta-tui "style.rebase-todo"]
    drop = bold red
```

The same in the TOML file:

```toml
[general]
log_args = ["--stat"]
collapsed_sections = ["branch_status"]
confirm_quit.enabled = false
diff_renderer.features = ["side-by-side", "line-numbers", "plus-minus-markers", "my-*"]

[bindings]
root.quit = ["q", "esc", "left"]

[style]
cursor = { symbol = "▌", fg = "blue" }
rebase_todo.drop = { fg = "red", mods = "BOLD" }
```

## Usage

`?` lists the commands available for the row under the cursor.

### Moving

- `j` / `k` (or `down` / `up`) move a line at a time, through a diff's lines too.
- `ctrl+j` / `ctrl+k` (or `ctrl+down` / `ctrl+up`) move the same way but skip over a diff's lines, stopping at file headers, hunk headers and commits.
- `alt+j` / `alt+k` (or `alt+down` / `alt+up`) move to the next / previous section at the same level; `alt+h` (or `alt+left`) moves to the file or commit containing the cursor.
- `space` / `backspace` move a page, `ctrl+d` / `ctrl+u` half a page, `g g` / `G` to the top / bottom; `ctrl+e` / `ctrl+y` scroll without moving the cursor.
- `/` searches (a regex over the text on screen); `n` / `N` go to the next / previous match. A match inside a fold is unfolded.
- `enter` on a commit opens it as `git show`; `q` goes back. `g s` opens the status view from any view.

### Folding

- `tab` folds or unfolds the file, hunk or commit under the cursor.
- `shift+tab` folds every file; pressing it again unfolds everything. In `git log -p` the second press folds every commit and the third unfolds everything.
- `git diff --stat` opens with every file folded to its stat row; `tab` unfolds a file to its diff.
- `file-header = stat` heads every file with its stat row, folded or not. After `shift+tab` unfolds everything, files are headed by delta's file headers.
- `start-folded = true` opens a large patch of several files folded.

### Staging

- `s` stages, `u` unstages and `K` discards the file, hunk or line under the cursor.
- `a` applies and `v` reverses it, for example to undo part of a commit shown by `git show`.
- `shift+down` / `shift+up` select several lines in one hunk; the next command acts on all of them.
- The commands offered depend on the git command: `git diff` can stage; `git diff --cached` can unstage; `git diff <rev>`, `git show` and `git log -p` can apply and reverse.
- In `git log -p`, each commit's diff is under the commit and can be folded, applied and reversed.

### Changing the git command

These re-run the git command and redraw the view.

- `:` edits the command, for example to add `--cached`, `-M` or `-S <text>`, or to change `git log` to `git log -p`. Only read-only git commands are accepted.
- `d` on a file header hides that file.
- `&` sets which files to show, as git pathspecs: `src !*_test.go` shows `src` without its tests.
- `U` sets the number of context lines; `W` shows the whole function; an empty answer restores the default.

### Commits

- `l l` shows the log of the current branch and `l o` of another; both use `log-args`.
- On a commit: `enter` opens it, `y` copies its hash, `r i` starts an interactive rebase from it, `V V` reverts it and `A A` cherry-picks it.
- `c c` commits, `c a` amends the last commit and `c e` adds what is staged to it without editing its message.
- `c f` and `c F` (fixup, instant fixup) open the log to choose the commit to fix up.
- `B` on a diff line opens the blame of its file, drawn by delta as it draws `git blame` (see delta's `blame-*` options); `enter` on a blame line opens that commit at that line.
- `Y` lists branches and tags; `enter` on one opens it as `git show`.

### Interactive rebase

With `sequence.editor` set as above, `git rebase -i` opens its instruction list in the TUI. `r i` and `delta --tui rebase <upstream>` open it directly; `r i` with no commit under the cursor opens the log to choose one. Commits are listed newest first, as in the log.

| key | action | binding |
|---|---|---|
| `p` | pick | `rebase_todo.rebase_todo_pick` |
| `r` | reword | `rebase_todo.rebase_todo_reword` |
| `e` | edit | `rebase_todo.rebase_todo_edit` |
| `s` | squash | `rebase_todo.rebase_todo_squash` |
| `f` | fixup | `rebase_todo.rebase_todo_fixup` |
| `d` | drop | `rebase_todo.rebase_todo_drop` |
| `alt+down` / `alt+up` | move the commit down / up | `rebase_todo.rebase_todo_move_down` / `rebase_todo.rebase_todo_move_up` |
| `enter` | start the rebase | `rebase_todo.rebase_todo_start` |
| `q` / `esc` | leave; as git's sequence editor, call the rebase off | `rebase_todo.quit` |

Movement and folding in the list are bound separately from the main view, as `rebase_todo.move_down` etc.

### Prompts

Prompts take readline keys (`ctrl+a`, `ctrl+e`, `alt+b`, `alt+f`, `ctrl+w`, `ctrl+k`, `ctrl+u`, `ctrl+y`, `ctrl+t`). `up` / `down` step through the history of `:` and `&`, kept per worktree.

### Other

- `ctrl+z` suspends the TUI; `fg` resumes it.
- `--debug` shows the output of the commands the TUI runs.

## Keybindings

`h` or `?` shows the bindings available for the row under the cursor. Each binding is rebound in the TUI config by the name in the last column, e.g. `hide-file = ctrl+k` in `[delta-tui "bindings.root"]`. The keys inside each menu are listed in the [default config](https://github.com/dandavison/gitu/blob/main/src/default_config.toml).

| key | action | binding |
|---|---|---|
| `j` / `k`, `down` / `up` | move down / up a line | `root.move_down_line` / `root.move_up_line` |
| `ctrl+j` / `ctrl+k`, `ctrl+down` / `ctrl+up` | move down / up, skipping diff lines | `root.move_down` / `root.move_up` |
| `shift+down` / `shift+up` | extend the line selection | `root.extend_selection_down` / `root.extend_selection_up` |
| `alt+j` / `alt+k`, `alt+down` / `alt+up` | next / previous section | `root.move_next_section` / `root.move_prev_section` |
| `alt+h`, `alt+left` | parent section | `root.move_parent_section` |
| `space` / `backspace` | page down / up | `root.full_page_down` / `root.full_page_up` |
| `ctrl+d` / `ctrl+u` | half page down / up | `root.half_page_down` / `root.half_page_up` |
| `g g` / `G` | top / bottom | `root.move_top` / `root.move_bottom` |
| `ctrl+e` / `ctrl+y` | scroll down / up | `root.scroll_view_down` / `root.scroll_view_up` |
| `tab` | fold | `root.toggle_section` |
| `shift+tab` | fold all | `root.toggle_all_sections` |
| `enter` / `right` | show | `root.show` |
| `s` `u` `K` | stage, unstage, discard | `root.stage`, `root.unstage`, `root.discard` |
| `a` `v` | apply, reverse | `root.apply`, `root.reverse` |
| `:` | edit the git command | `root.edit_git_command` |
| `d` | hide the file under the cursor | `root.hide_file` |
| `&` | set which files to show | `root.file_patterns` |
| `U` | set the diff context | `root.diff_context` |
| `-` | delta features | `root.renderer_features` |
| `S` `L` `+` | side-by-side, line numbers, +/- markers | `general.diff_renderer.feature_keys` |
| `T` | color themes | `root.renderer_themes` |
| `/` `n` `N` | search, next, previous | `root.search`, `root.search_next`, `root.search_previous` |
| `y` | copy hash | `root.copy_hash` |
| `Y` | show refs | `root.show_refs` |
| `B` | blame | `root.blame` |
| `g s` | status view | `root.status` |
| `g r` | refresh | `root.refresh` |
| `ctrl+z` | suspend | `root.suspend` |
| `h` `?` | help | `root.help_menu` |
| `q` `esc` | quit | `root.quit` |
| `b` | branch menu | `root.branch_menu` |
| `c` | commit menu | `root.commit_menu` |
| `f` | fetch menu | `root.fetch_menu` |
| `F` | pull menu | `root.pull_menu` |
| `P` | push menu | `root.push_menu` |
| `l` | log menu | `root.log_menu` |
| `m` | merge menu | `root.merge_menu` |
| `M` | remote menu | `root.remote_menu` |
| `r` | rebase menu | `root.rebase_menu` |
| `X` | reset menu | `root.reset_menu` |
| `V` | revert menu | `root.revert_menu` |
| `A` | cherry-pick menu | `root.cherry_pick_menu` |
| `z` | stash menu | `root.stash_menu` |

## Limitations

- delta does not mark merge conflict rows or `\ No newline at end of file` rows, so the TUI does not show them.
- Output beyond `max-input-bytes` is dropped; a row at the end says so.
- A shell alias (`!…`) is not recognised as the git command it runs.
- A feature key cannot be set in git config for a feature whose name has capitals, since git lowercases config names.
