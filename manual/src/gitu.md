# Interactive mode (gitu)

`delta --gitu` is an interactive git client and pager whose diffs are rendered by delta. It is [gitu](https://github.com/altsem/gitu) by altsem, a git client inspired by Magit, embedded via a [fork](https://github.com/dandavison/gitu). It is experimental.

```sh
delta --gitu                            # status view of the current repository
git diff | delta --gitu --pager         # browse a patch
delta --gitu rebase <upstream>          # edit an interactive rebase's instruction list
```

Diffs are rendered by the running delta binary, configured as usual by your `[delta]` git config. gitu maps each rendered row back to its line in the patch, so side-by-side, line numbers etc. do not prevent staging individual lines.

## Git config

```gitconfig
[core]
    pager = delta
[pager]
    diff = delta --gitu --pager
    show = delta --gitu --pager
    log = delta --gitu --pager
[sequence]
    editor = delta --gitu sequence-editor
```

With `--quit-if-one-screen` (as less's), output that fits on the screen is printed and gitu exits; that also leaves nothing to stage from, so it suits `log` better than `diff`. Input larger than `general.max_input_bytes` (2 MB) is truncated.

## gitu config

Optional, at `~/.config/gitu/config.toml` (or `--config <file>`). The diff renderer is always delta, whatever this file says.

```toml
[general]
# Offered by `-` (default: side-by-side, line-numbers, keep-plus-minus-markers).
# `*` and `?` match the `[delta "name"]` features in your git config: "*"
# offers all of them.
diff_renderer.features = ["side-by-side", "line-numbers", "my-*"]
# Top-level keys that toggle features. In the `-` list, a letter sets the
# selected feature's key (again to unset it), and gitu saves it here.
diff_renderer.feature_keys = { side-by-side = "x" }
# Pathspecs excluded when the view is first built; `:` shows them and `&` edits them.
hide = ["*.pb.go"]
# Stop the cursor on unchanged lines too.
visit_context_lines = true

[bindings]
root.stage = ["s"]
```

See gitu's [default config](https://github.com/dandavison/gitu/blob/diff-renderer-and-pager/src/default_config.toml) for every setting and binding.

## Keybindings

`h` or `?` shows the bindings available in the current view. Each binding is rebound under `[bindings]` in the gitu config by the name in the last column, e.g. `root.hide_file = ["ctrl+k"]`. The keys inside each menu are listed in gitu's [default config](https://github.com/dandavison/gitu/blob/diff-renderer-and-pager/src/default_config.toml).

### Added in the fork

| key | action | binding |
|---|---|---|
| `:` | edit the git command that produced the view (read-only commands only) | `root.edit_git_command` |
| `d` | exclude the file whose header is under the cursor | `root.hide_file` |
| `&` | edit the pathspecs limiting the view (`!` excludes) | `root.file_patterns` |
| `U` | change the diff context (a number, or `W` for the whole function) | `root.diff_context` |
| `-` | toggle delta features; in the list, a letter sets the selected feature's key | `root.renderer_features` |
| `shift+tab` | fold all (folds all files in diff views) | `root.toggle_all_sections` |
| `space` / `backspace` | page down / up | `root.full_page_down` / `root.full_page_up` |
| `shift+down` / `shift+up` | extend the line selection | `root.extend_selection_down` / `root.extend_selection_up` |
| `right` | show (as `enter`) | `root.show` |
| `g s` | go to the status view | `root.status` |

In the rebase instruction list:

| key | action | binding |
|---|---|---|
| `p` | pick | `rebase_todo.rebase_todo_pick` |
| `r` | reword | `rebase_todo.rebase_todo_reword` |
| `e` | edit | `rebase_todo.rebase_todo_edit` |
| `s` | squash | `rebase_todo.rebase_todo_squash` |
| `f` | fixup | `rebase_todo.rebase_todo_fixup` |
| `d` | drop | `rebase_todo.rebase_todo_drop` |
| `alt+j` / `alt+k` | move the commit down / up | `rebase_todo.rebase_todo_move_down` / `rebase_todo.rebase_todo_move_up` |
| `enter` | start the rebase | `rebase_todo.rebase_todo_start` |
| `Q` | call the rebase off | `rebase_todo.rebase_todo_abort` |

Movement, folding and `q` in the list are bound separately from the main view, as `rebase_todo.move_down` etc.

### gitu

| key | action | binding |
|---|---|---|
| `j` / `k` | move down / up | `root.move_down` / `root.move_up` |
| `ctrl+j` / `ctrl+k` | move down / up a line | `root.move_down_line` / `root.move_up_line` |
| `alt+j` / `alt+k` | next / previous section | `root.move_next_section` / `root.move_prev_section` |
| `alt+h` | parent section | `root.move_parent_section` |
| `ctrl+d` / `ctrl+u` | half page down / up | `root.half_page_down` / `root.half_page_up` |
| `g g` / `G` | top / bottom | `root.move_top` / `root.move_bottom` |
| `ctrl+e` / `ctrl+y` | scroll down / up | `root.scroll_view_down` / `root.scroll_view_up` |
| `tab` | fold | `root.toggle_section` |
| `enter` | show | `root.show` |
| `s` `u` `K` | stage, unstage, discard | `root.stage`, `root.unstage`, `root.discard` |
| `a` `v` | apply, reverse | `root.apply`, `root.reverse` |
| `/` `n` `N` | search, next, previous | `root.search`, `root.search_next`, `root.search_previous` |
| `y` | copy hash | `root.copy_hash` |
| `Y` | show refs | `root.show_refs` |
| `B` | blame | `root.blame` |
| `g r` | refresh | `root.refresh` |
| `h` `?` | help | `root.help_menu` |
| `q` | quit | `root.quit` |
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
