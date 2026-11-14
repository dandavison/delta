Hide
# A squash's combined message is taken as git writes it.
Type "export GIT_EDITOR=true && clear" Enter
Sleep 300ms
Show
Announce@4s "How to use delta's TUI: git log" "delta's TUI is based on https://github.com/altsem/gitu"
Type "gl" Sleep 500ms Enter
Wait+Screen /▸ [0-9a-f]{7}/
# Side-by-side is where a session starts, without saying so.
Hide
Type "S"
Sleep 1s
Show
Sleep 1s
Ctrl+Down@150ms 2 Ctrl+Up Sleep 1s

# scene
Announce "/ searches"
Type "/" Sleep 500ms Type "#2181" Sleep 500ms Enter
Sleep 1.5s

Enter
Wait+Screen /14 files changed/
Sleep 2.5s

# scene
# A hunk of the commit, unannounced, as tab and side-by-side are shown in the diff chapter.
Ctrl+Down@80ms 7 Sleep 500ms Tab Sleep 2.5s

# scene
Type "q" Sleep 2s

# scene
Hide
Type "gg"
Sleep 300ms
Show
Ctrl+Down@150ms 4 Sleep 700ms
Announce "r i starts an interactive rebase"
Type "r" Sleep 500ms Type "i" Sleep 2.5s
Announce "alt+up and alt+down move a commit"
Down@150ms 3 Sleep 500ms Alt+Up Sleep 1s Alt+Up Sleep 1.5s
Announce "s and f squash and fixup commits"
Up@150ms 2 Sleep 500ms Type "f" Sleep 1s Down@150ms 2 Sleep 500ms Type "s" Sleep 1.5s
Announce "enter runs the rebase"
Enter
Sleep 3s

Announce ": edits the git command, here to add -p"
Type ":" Sleep 1s
Type@120ms " -p" Sleep 1s
Enter
# With -p, two blank rows come between a commit's subject and its stat; without, one.
Wait+Screen /\n[ ▌]*\n[ ▌]*\n[ ▌]+\S+ +\| +[0-9]+ [+-]/
Sleep 2.5s

# scene
Announce "shift+tab folds files, then commits"
Shift+Tab Sleep 1.5s Shift+Tab Sleep 1.5s Shift+Tab Sleep 1.5s

# scene
Announce@3s "To use delta's TUI as shown in this video put the following in git config:"
Hide
Type "q" Sleep 1s
Type "clear; show_config \"# For the log, also in your shell:\" \"# alias gl='git log --date relative --color=always --pretty=commit-format --stat'\"" Enter
Sleep 1s
Show
Sleep 8s
