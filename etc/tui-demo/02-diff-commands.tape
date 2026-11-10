Hide
# A branch whose diff from main is the 9 files of the commit implementing hunk styles with style
# strings, in a clone whose remote is delta on GitHub, for gh, which pages with delta --tui.
Type "git checkout -q -b hunk-styles 2245582f && git branch -q -f main 2245582f~" Enter
Type "git remote add origin https://github.com/dandavison/delta.git" Enter
Type "export GH_PAGER='delta --tui --pager' && clear" Enter
Sleep 500ms
Show

Announce@4s "How to use delta's TUI: changing what git diff shows" "delta's TUI is based on https://github.com/altsem/gitu"
Type "git diff main" Sleep 500ms Enter
Wait+Screen /9 files changed/
# Where the first diff chapter leaves off: every file unfolded, on the first hunk of delta.rs,
# side-by-side off.
Hide
# Until the whole diff has arrived, so that the moves land where they did in the first chapter.
Sleep 4s
Type "S" Sleep 1s
Down@80ms 2 Sleep 500ms Tab Sleep 1s Tab Sleep 1s
Shift+Tab Sleep 1s
Ctrl+Down@80ms 19 Sleep 1s
Type "S" Sleep 1s
Show
Sleep 1s

# scene
Announce "U sets the number of context lines"
Announce@3s "For example, set it to a large number to look at changes with full file context."
Type "U" Sleep 1s Type@150ms "777" Sleep 700ms Enter
Sleep 2s
# Through delta.rs, whose two hunks are now one, to its changes, side-by-side for a while.
Space@1200ms 3
Type "S" Sleep 1.5s
Space@1200ms 3 Sleep 1s
Type "S" Sleep 1.5s

# scene
Announce "shift+tab folds every file"
Shift+Tab Sleep 1.5s

# scene
Announce ": edits the git command, here to add -- src/delta.rs src/paint.rs"
Type ":" Sleep 1s Type@80ms " -- src/delta.rs src/paint.rs" Sleep 700ms Enter
Wait+Screen /2 files changed/
# The list opens scrolled to the cursor, below delta.rs.
Hide
Ctrl+Y Sleep 300ms
Show
Sleep 2.5s

# scene
# The cursor is on paint.rs, where it was in the list of every file.
Announce "d hides a file"
Type "d" Sleep 2.5s

# scene
Announce "U, then W, shows whole functions"
Type "U" Sleep 800ms Type "W" Sleep 700ms Enter Sleep 1s
Tab Sleep 3.5s
Hide
Type "q"
Sleep 500ms
Type "clear" Enter
Sleep 500ms
Show

# scene
Announce "gh pr diff opens in the TUI too"
Type "gh pr diff 2181" Sleep 500ms Enter
Wait+Screen@30s /files changed/
Hide
Type "S"
Sleep 1s
Show
Sleep 2s
Down@80ms 7 Sleep 500ms Tab Sleep 2s

# scene
Announce@3s "To use delta's TUI as shown in this video put the following in git config:"
Hide
Type "q" Sleep 1s
Type "clear; show_config \"# For gh pr diff, also in your shell:\" \"# export GH_PAGER='delta --tui --pager'\"" Enter
Sleep 1s
Show
Sleep 8s
