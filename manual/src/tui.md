# TUI mode

`delta --tui` is an interactive git client and pager whose diffs are rendered by delta. You can stage, fold and navigate delta's output whatever its layout: side-by-side, line numbers etc. do not prevent staging individual lines. It is based on [gitu](https://github.com/altsem/gitu) by altsem, a git client inspired by Magit, and is embedded via a [fork](https://github.com/dandavison/gitu). It is experimental. `--gitu` is an alias of `--tui`.

## Setup

In git config:

```gitconfig
[core]
    pager = delta
[pager]
    diff = delta --tui
    show = delta --tui
    log = delta --tui
[sequence]
    editor = delta --tui sequence-editor
```

With this, `git diff`, `git show`, `git log` and `git rebase -i` open in the TUI. `delta --tui` on its own opens the status view.

## Getting around

The TUI shows a stack of views. The first is the one you started in: `git log`, `git diff`, `git show`, or status. `right` goes in and `left` comes back out:

- `right` on a commit opens it as a new view on top. `right` on a folded file or hunk unfolds it.
- `enter` does the same for a commit, but on a file, hunk or line it opens your editor there.
- `left` folds what you unfolded, and otherwise returns to the view below.

So from `git log` you can go `right` into a commit, `right` into a folded file, `enter` into your editor at a line, and then `left` back out to the log.

Other basics: `j` / `k` move by line and `ctrl+down` / `ctrl+up` by file, hunk or commit; `tab` folds; `s` stages and `u` unstages; `q` goes back, and quits from the first view. `?` lists the keys that apply to the row under the cursor.

## Configuration

The TUI is configured in git config, in the `[delta-tui]` section and its subsections, beside your `[delta]` section. For example, to open a large diff as a list of its files, one row each, which `tab` or `right` unfolds:

```gitconfig
[delta-tui]
    # Head each file with its `git diff --stat` row.
    file-header = stat
    # Open a patch of several files folded, if it does not fit on one screen.
    start-folded = true
```

Every setting is listed under [TUI config](#tui-config).

## Keys

Everything the TUI does. `h` or `?` shows what is available for the row under the cursor. A binding is rebound in the TUI config by the name in the last column, e.g. `hide-file = ctrl+k` in `[delta-tui "bindings.root"]`.

### Moving and searching

| key | action | binding |
|---|---|---|
| `j` / `k`, `down` / `up` | down / up a line, through a diff's lines too | `root.move_down_line` / `root.move_up_line` |
| `ctrl+j` / `ctrl+k`, `ctrl+down` / `ctrl+up` | down / up, stopping only at file headers, hunk headers and commits | `root.move_down` / `root.move_up` |
| `alt+j` / `alt+k`, `alt+down` / `alt+up` | next / previous section at the same level | `root.move_next_section` / `root.move_prev_section` |
| `alt+h`, `alt+left` | the file or commit containing the cursor | `root.move_parent_section` |
| `space` / `backspace`, `pagedown` / `pageup` | page down / up | `root.full_page_down` / `root.full_page_up` |
| `ctrl+d` / `ctrl+u` | half page down / up | `root.half_page_down` / `root.half_page_up` |
| `g g` / `G` | top / bottom | `root.move_top` / `root.move_bottom` |
| `ctrl+e` / `ctrl+y` | scroll down / up without moving the cursor | `root.scroll_view_down` / `root.scroll_view_up` |
| `/` | search | `root.search` |
| `n` / `N` | next / previous match | `root.search_next` / `root.search_previous` |
| (none) | search backward | `root.search_backward` |

A search is a regex over the text on screen, and moves to the first match as you type; `enter` keeps it and `esc` goes back to where you were. Case is ignored unless the search has a capital. A match inside a fold is unfolded.

### Opening, folding and going back

| key | action | binding |
|---|---|---|
| `right` | open the commit, branch, tag, stash or blame line as `git show`; unfold a folded file or hunk | `root.show_or_unfold` |
| `enter` | open the commit, branch, tag, stash or blame line as `git show`; open a file, hunk or line in your editor | `root.show` |
| `left` | fold what you unfolded on this screen, if the cursor is in it; otherwise go back | `root.fold_or_back` |
| `q` `esc` | go back; on the first screen, quit | `root.quit` |
| `tab` | fold / unfold | `root.toggle_section` |
| `shift+tab` | fold every file; again, unfold everything | `root.toggle_all_sections` |
| `g s` | status view, from any view | `root.status` |
| `g r` | refresh | `root.refresh` |

- `left` folds a section you unfolded on this screen (with `right` or `tab`) when the cursor is on it or anywhere inside it, and moves the cursor onto it. So `right` then `left` returns to where you were, whether `right` opened a commit or unfolded a file. On the first screen `left` does not quit.
- `right` on a blame line opens its commit at that line. `enter` opens a file in the editor named by the first of `GITU_SHOW_EDITOR`, `VISUAL`, `EDITOR` and `GIT_EDITOR` that is set, at the hunk or line.
- In `git log -p`, the second `shift+tab` folds every commit and the third unfolds everything.
- `git diff --stat` opens with every file folded to its stat row; `tab` unfolds a file to its diff. A patch asked for with its stat (`git log -p --stat`, `--patch-with-stat`) has each file headed by its stat row, unfolded. `git log --stat` and `git show --stat` show git's own stat. `file-header = stat` heads every file with its stat row, folded or not; after `shift+tab` unfolds everything, files are headed by delta's file headers. `start-folded = true` opens a large patch of several files folded.

### Staging

| key | action | binding |
|---|---|---|
| `s` `u` `K` | stage, unstage, discard the file, hunk or line | `root.stage`, `root.unstage`, `root.discard` |
| `a` `v` | apply, reverse it | `root.apply`, `root.reverse` |
| `shift+down` / `shift+up` | select lines in a hunk, for the next of these to act on | `root.extend_selection_down` / `root.extend_selection_up` |

What is offered depends on the git command: `git diff` can stage; `git diff --cached` can unstage; `git diff <rev>`, `git show` and `git log -p` can apply and reverse, for example to undo part of a commit. In `git log -p`, each commit's diff is under the commit and can be folded, applied and reversed.

### Changing the git command

These run the git command again and redraw the view.

| key | action | binding |
|---|---|---|
| `:` | edit the git command | `root.edit_git_command` |
| `d` | hide the file under the cursor | `root.hide_file` |
| `&` | set which files to show | `root.file_patterns` |
| `U` | set the diff context | `root.diff_context` |
| `+` / `-` | wider / narrower diff context | `root.wider_diff_context` / `root.narrower_diff_context` |

- `:` takes only read-only git commands: for example, add `--cached`, `-M` or `-S <text>`, or change `git log` to `git log -p`.
- `&` takes git pathspecs: `src !*_test.go` shows `src` without its tests.
- `U` takes a number of context lines, or `W` for the whole function; an empty answer restores the default. `+` and `-` step by powers of two: from git's 3 to 4, 8, 16, and back down to none.

### delta

| key | action | binding |
|---|---|---|
| `H` | list features, shell commands and views, and set their keys | `root.renderer_features` |
| `S` `L` | side-by-side, line numbers | `general.diff_renderer.feature_keys` |
| your keys | your features, shell commands and views | `general.diff_renderer.feature_keys` |
| `T` | theme mode | `root.renderer_themes` |

- In the `H` list, `up` / `down` move within a column and `left` / `right` between columns. `space` turns the selected feature on or off and keeps the list open; `enter` does so and closes it, runs a command, or opens a view. Typing a key makes it the selected item's key, and typing its own key removes it.
- In theme mode, `down` / `up` (or `tab` / `shift+tab`) step through the themes and a digit selects one; `ctrl+l` lists every theme, where typing searches. `enter` keeps the theme; `esc` or `q` puts back the one you had.

### Commits and other

| key | action | binding |
|---|---|---|
| `y` | copy the commit's hash | `root.copy_hash` |
| `Y` | list branches and tags | `root.show_refs` |
| `B` | blame the file of the diff line under the cursor | `root.blame` |
| `h` `?` | help | `root.help_menu` |
| `ctrl+z` | suspend; `fg` resumes | `root.suspend` |
| mouse | with `mouse-support`: click selects; a second click folds a section, or does what `enter` does; right-click does what `enter` does; the wheel scrolls | |

Blame is drawn by delta as it draws `git blame` (see delta's `blame-*` options). `--debug` shows the output of the commands the TUI runs.

### Menus

A menu opens with its key, lists what is in it, and closes with `q`, `esc` or `left`. A key starting with `-` turns a git argument on or off for the menu's commands. Bindings in a menu are named as in the [default config](https://github.com/dandavison/gitu/blob/main/src/default_config.toml), e.g. `branch_menu.checkout`.

| key | menu | in it |
|---|---|---|
| `b` | branch | `b` checkout, `c` checkout new branch, `s` spin off, `m` rename, `K` delete |
| `c` | commit | `c` commit, `a` amend, `e` extend, `f` fixup, `F` instant fixup; `-a` `--all`, `-e` `--allow-empty`, `-v` `--verbose`, `-n` `--no-verify`, `-R` `--reset-author`, `-s` `--signoff` |
| `f` | fetch | `a` all remotes, `p` push remote, `u` upstream, `e` elsewhere; `-p` `--prune`, `-t` `--tags` |
| `F` | pull | `p` push remote, `u` upstream, `e` elsewhere; `-r` `--rebase` |
| `P` | push | `p` push remote, `u` upstream, `e` elsewhere; `-f` `--force-with-lease`, `-F` `--force`, `-h` `--no-verify`, `-n` `--dry-run` |
| `l` | log | `l` current branch, `o` other; `-n` limit, `-F` `--grep` |
| `m` | merge | `m` merge, `a` abort, `c` continue; `-f` `--ff-only`, `-n` `--no-ff` |
| `M` | remote | `a` add, `r` rename, `K` remove |
| `r` | rebase | `i` interactive, `e` onto elsewhere, `f` autosquash, `a` abort, `c` continue; `-i` `--interactive`, `-a` `--autosquash`, `-A` `--autostash`, `-k` `--keep-empty`, `-p` `--preserve-merges`, `-d` `--committer-date-is-author-date`, `-h` `--no-verify` |
| `X` | reset | `s` soft, `m` mixed, `h` hard |
| `V` | revert | `V` revert, `a` abort, `c` continue; `-e` `--edit`, `-E` `--no-edit`, `-s` `--signoff` |
| `A` | cherry-pick | `A` cherry-pick, `a` abort, `c` continue; `-e` `--edit`, `-n` `--no-commit`, `-s` `--signoff` |
| `z` | stash | `z` index and worktree, `i` index, `w` worktree, `x` keeping the index, `p` pop, `a` apply, `k` drop; `-u` `--include-untracked`, `-a` `--all` |

- `l l` and `l o` use `log-args`.
- `c a` amends the last commit; `c e` adds what is staged to it without editing its message. `c f` and `c F` open the log to choose the commit to fix up, where moving is as in the log and `enter` chooses (bindings `commit_picker.*`).
- `r i`, `V V` and `A A` act on the commit under the cursor.

### Interactive rebase list

With `sequence.editor` set as in Setup, `git rebase -i` opens its instruction list in the TUI. `r i` and `delta --tui rebase <upstream>` open it directly; `r i` with no commit under the cursor opens the log to choose one. Commits are listed newest first, as in the log.

| key | action | binding |
|---|---|---|
| `p` | pick | `rebase_todo.rebase_todo_pick` |
| `r` | reword | `rebase_todo.rebase_todo_reword` |
| `e` | edit | `rebase_todo.rebase_todo_edit` |
| `s` | squash | `rebase_todo.rebase_todo_squash` |
| `f` | fixup | `rebase_todo.rebase_todo_fixup` |
| `d` | drop | `rebase_todo.rebase_todo_drop` |
| `alt+j` / `alt+k`, `alt+down` / `alt+up` | move the commit down / up | `rebase_todo.rebase_todo_move_down` / `rebase_todo.rebase_todo_move_up` |
| `enter` | start the rebase | `rebase_todo.rebase_todo_start` |
| `q` / `esc` | leave; as git's sequence editor, call the rebase off | `rebase_todo.quit` |

Moving a commit, or marking one and moving on to the next, keeps the commits either side of the cursor in view, so you can see whether to go further.

Movement and folding in the list are bound separately from the main view, as `rebase_todo.move_down` etc.

### Lists and prompts

- A list to choose from, such as the branches to check out: typing filters it; `down` / `up` (or `ctrl+n` / `ctrl+p`, `tab` / `shift+tab`) move; `enter` chooses, or takes what was typed where the list allows that; `esc` / `ctrl+c` cancels. Bindings: `picker.next`, `picker.previous`, `picker.done`, `picker.cancel`.
- A prompt takes readline keys (`ctrl+a`, `ctrl+e`, `alt+b`, `alt+f`, `ctrl+w`, `ctrl+k`, `ctrl+u`, `ctrl+y`, `ctrl+t`); `enter` answers and `esc` cancels. `up` / `down` step through the history of `:`, `&` and `/`, kept per worktree. A prompt with a default shows it in parentheses, and an empty answer takes it.
- A question ending in `[y/n]` is answered by `y`; any other key says no.

## delta features

`S` toggles side-by-side and `L` line numbers. Each enables or disables the feature whether or not your delta config enables it. This holds for the session; nothing is written to your `[delta]` config.

`H` lists these, `plus-minus-markers` (which keeps the `+`/`-` at the start of each line, for copying a diff as a patch; delta calls it `keep-plus-minus-markers`), and every feature defined in your git config as `[delta "name"]` (see TUI config), in a column beside your shell commands and views. The keys for using the list are under Keys; a key you give a feature there is saved in git config. Or set them yourself:

```gitconfig
# These are the defaults. Keys set here replace all of them. A key that is
# already bound is refused.
[delta-tui "general.diff-renderer.feature-keys"]
    side-by-side = S
    line-numbers = L
```

## Color themes

`T` enters theme mode. A row at the bottom of the screen names a theme, and the screen is drawn with it; its keys are listed under Keys. A digit selects the theme of that number, 0 being no theme (your own config as it is), so that two can be compared. Feature keys such as `S` still work, to see a theme side-by-side. A theme you keep is saved in git config.

A theme is overlaid on your delta config, after any features you turned on. The themes are those in your git config, as `delta --show-themes` finds them: features that set `dark` or `light`. To have delta's collection of themes, include its `themes.gitconfig` (see [Custom themes](./custom-themes.md)). Themes for the other background, light or dark, are only in the list.

A theme cannot change a setting made in `[delta]` itself, since those win over every feature. So that themes can restyle what you have styled yourself, put your styles in a feature of your own instead:

```gitconfig
[delta]
    features = my-style
[delta "my-style"]
    file-style = bold cyan
    file-decoration-style = magenta ul
```

delta's builtin features can be offered as themes too:

```gitconfig
[delta-tui "general.diff-renderer"]
    builtin-themes = diff-so-fancy
    # Set by theme mode; or set it yourself.
    theme = my-theme
```

## Shell commands

Shell commands of your own can be run from a key, with the commit, file or branch under the cursor:

```gitconfig
[delta-tui "general.commands"]
    open-pr = gh pr view --web {commit}
    test-file = cargo test -- {file}
```

`H` lists them in a column of their own, and a key is set for one as for a feature; `?` lists it by its name. Or set the key yourself, beside the feature keys:

```gitconfig
[delta-tui "general.diff-renderer.feature-keys"]
    open-pr = o
```

The key runs the command with `sh -c` in the repository's root, so pipes, redirection and the rest of POSIX shell syntax work. It has `COLUMNS` and `DELTA_FEATURES` set as the TUI sets them for the delta it draws with, so a delta it runs draws as the screen is drawn, with the features and theme chosen in the TUI, though without the TUI's folds and counted stat rows. For example, to open the screen's git command as a web page (git drops colors in a pipe, hence `--color=always`):

```gitconfig
[delta-tui "general.commands"]
    html = {command} --color=always | delta | aha > /tmp/diff.html && open /tmp/diff.html
```

`{commit}`, `{file}` and `{branch}` are filled in, quoted for the shell, from the row under the cursor: a commit, a file or a line of its diff, or a branch (or a commit a branch points to). `{command}` is the git command the screen is showing, as `:` shows it. A command using one that is not there is not run, and says which. The TUI stays usable while the command runs, and redraws its views when it exits.

The command's output is not shown, unless with `--debug`; a command that fails says so, with its exit code. One command runs at a time, and it cannot be cancelled. Commands need `sh`, so they do not run on Windows without one on the `PATH`.

## Views

A view is a `git diff`, `git show` or `git log` of your own, opened from a key, with the commit, file or branch under the cursor:

```gitconfig
[delta-tui "general.views"]
    diff-since = git diff {commit}...
    word-diff = {command} --word-diff
[delta-tui "general.diff-renderer.feature-keys"]
    diff-since = D
    word-diff = W
```

The key opens the view on top of the one it is pressed on, as `right` opens a commit, and `left` goes back. `{commit}`, `{file}` and `{branch}` are filled in as for shell commands, but a view is not run by a shell: `{command}` is a word of its own there, standing for the words of the git command the screen is showing, so that a view such as `word-diff` above changes the command in view. A view is a git command the TUI shows, run again after each change, and `:`, `U` and `&` change it as they change any other. A view that is not `git diff`, `git show` or `git log` is refused. `H` lists views in a column of their own.

## Log and commit formats

`git log` and `git show` typed at the command line use git's `format.pretty`. The `git log` and `git show` that the TUI runs itself (`l l`, `right` on a commit) take their arguments from `log-args` and `show-args` in the TUI config. A commit is one cursor stop only if its first row matches delta's `commit-regex`, so a format of your own needs a `commit-regex` that matches it. For example:

```gitconfig
[delta]
    commit-regex = ^(▸|commit )
    commit-decoration-style = blue ol
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

## As git's pager

The TUI finds the git command that started it and runs it again itself, in the same directory, after each change you make. git aliases are recognised.

With `--quit-if-one-screen` (as less's), output that fits on the screen is printed and the TUI exits. That leaves nothing to stage from, so it suits `log` better than `diff`:

```gitconfig
[pager]
    log = delta --tui --quit-if-one-screen
```

Other programs that page a diff can use the TUI too, e.g. for `gh pr diff`:

```sh
export GH_PAGER='delta --tui'
```

## TUI config

Besides git config's `delta-tui` section, the TUI reads a TOML file, `~/.config/gitu/config.toml` (or `--config <file>`); where both set something, git config wins. The diff renderer is always delta, whatever either says. See the [default config](https://github.com/dandavison/gitu/blob/main/src/default_config.toml) for every setting and binding.

A setting's name in git config is its name in the TOML file with dashes for underscores. Settings under `[general]` go in `[delta-tui]`; others go in a subsection named by their path: `diff_renderer.features` is `features` in `[delta-tui "general.diff-renderer"]`, and `[style] hunk_header` is `hunk-header` in `[delta-tui "style"]`. A list is the key given once per item. A boolean or number is read as git reads one (`yes`, `4m`). A style is written as delta writes one: attributes, then a foreground and a background color. The bindings of menu arguments (such as `commit_menu.--all`) can be set only in the TOML file, since git config names cannot start with `-`.

Values below are the defaults except where marked as an example.

### Features listed by `H`

```gitconfig
[delta-tui "general.diff-renderer"]
    # Features offered by `H`. `*` and `?` match the `[delta "name"]` features
    # in your git config. These are the defaults:
    features = side-by-side
    features = line-numbers
    features = plus-minus-markers
    features = *
```

### Other settings

```gitconfig
[delta-tui]
    # Arguments to the `git log` run by `l l` and `l o`, also used for the
    # commits in the rebase instruction list. See "Log and commit formats" above.
    # Example:
    log-args = --stat
    # Arguments to the `git show` run by right or enter on a commit and `delta --tui
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
# tables under Keys above. Examples:
[delta-tui "bindings.root"]
    quit = q
    quit = ctrl+c
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
root.quit = ["q", "ctrl+c"]

[style]
cursor = { symbol = "▌", fg = "blue" }
rebase_todo.drop = { fg = "red", mods = "BOLD" }
```

## Limitations

- delta does not mark merge conflict rows or `\ No newline at end of file` rows, so the TUI does not show them.
- Output beyond `max-input-bytes` is dropped; a row at the end says so.
- A shell alias (`!…`) is not recognised as the git command it runs.
- A feature key cannot be set in git config for a feature whose name has capitals, since git lowercases config names. For the same reason, the name of a shell command or view given in git config is lowercased.
