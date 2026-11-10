Hide
# A branch whose diff from main is the 9 files of the commit implementing hunk styles with style
# strings, in a clone whose remote is delta on GitHub, for gh, which pages with delta --tui.
Type "git checkout -q -b hunk-styles 2245582f && git branch -q -f main 2245582f~" Enter
Type "git remote add origin https://github.com/dandavison/delta.git" Enter
Type "export GH_PAGER='delta --tui --pager' && clear" Enter
Sleep 500ms
Show

Announce@4s "How to use delta's TUI: git diff" "delta's TUI is based on https://github.com/altsem/gitu"
Type "git diff main" Sleep 500ms Enter
Wait+Screen /9 files changed/
# Side-by-side is where a session starts, without saying so.
Hide
Type "S"
Sleep 1s
Show
Announce "Large diffs open folded"
Sleep 1s

# scene
Down@80ms 2 Sleep 500ms
Announce "tab unfolds and folds a file"
Tab Sleep 1.5s Tab Sleep 1.5s

# scene
Announce "shift+tab unfolds every file"
Shift+Tab Sleep 1s

# scene
# To the first hunk of delta.rs, removals and additions in pairs, which side-by-side and themes
# show well; hidden, since scrolling past 18 hunks would take much of the video's size.
Hide
Ctrl+Down@80ms 19 Sleep 1s
Show
Announce "S toggles side-by-side"
Type "S" Sleep 2.5s Type "S" Sleep 2.5s Type "S" Sleep 2.5s

# scene
# Up to delta.rs's file header, so that themes show how they head files and hunks.
Ctrl+Y@80ms 4 Sleep 1s
Announce "T enters color theme selection mode"
Type "T" Sleep 1.5s
Down@750ms 3 Sleep 750ms
Type "S" Sleep 2s
Down@750ms 3 Sleep 750ms
Type "S" Sleep 1.5s
# delta's diff-so-fancy look, after the eight themes the file themes gives each background.
Type "9" Sleep 1.25s
Escape Sleep 1.5s

# scene
Announce "L toggles line numbers"
Type "L" Sleep 2.5s Type "L" Sleep 2s

# scene
Announce@3s "To use delta's TUI as shown in this video put the following in git config:"
Hide
Type "q" Sleep 1s
Type "clear; show_config" Enter
Sleep 1s
Show
Sleep 8s
