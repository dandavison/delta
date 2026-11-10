Hide
# Unstaged changes: the OSC-1717 commit's changes to src, over the commit before it. Recent
# commits are shown so that the commit and the amend can be seen.
Type "git checkout -q -b osc-1717 42a3e1a~ && git checkout 42a3e1a -- src && git reset -q -N && git config delta-tui.recent-commits-limit 3 && clear" Enter
Sleep 500ms
Show

Announce@4s "How to use delta's TUI: staging & committing" "delta's TUI is based on https://github.com/altsem/gitu"
Announce "delta --tui opens the status view"
Type "delta --tui" Sleep 500ms Enter
Wait+Screen /Unstaged changes/
Hide
Type "S"
Sleep 1s
Show
Sleep 1s

# scene
Down@80ms 2 Sleep 500ms
Announce "s stages the file under the cursor"
Type "s" Sleep 2s

# scene
Down@80ms 4 Sleep 300ms Tab Sleep 1.5s
Announce "In an unfolded file, s stages the hunk under the cursor"
Ctrl+Down@300ms 2 Sleep 700ms Type "s" Sleep 2s

# scene
Ctrl+Up Sleep 300ms Down@100ms 8 Sleep 700ms
Announce "shift+down selects lines, and s stages them"
Shift+Down@200ms 8 Sleep 1s Type "s" Sleep 2s

# scene
Announce "shift+tab folds every file"
Shift+Tab Sleep 2.5s

# scene
Announce "c c commits what is staged"
Type "c" Sleep 500ms Type "c"
Sleep 2s
Type@60ms "Emit OSC-1717 records for hunk headers" Sleep 1s
Ctrl+S Sleep 300ms Ctrl+Q
Wait+Screen /Emit OSC-1717 records for hunk headers/
Sleep 2.5s

# scene
Type "s" Sleep 1.5s
Announce "c e adds what is staged to the last commit, and c a lets you edit its message too"
Type "c" Sleep 500ms Type "e"
Sleep 3s

# scene
Announce@3s "To use delta's TUI as shown in this video put the following in git config:"
Hide
Type "q" Sleep 1s
Type "clear; show_config \"# For the recent commits this video shows:\" \"[delta-tui]\" \"    recent-commits-limit = 3\"" Enter
Sleep 1s
Show
Sleep 8s
