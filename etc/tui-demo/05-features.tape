Hide
# The diff of a commit that rewrites part of the blame handler, with the default feature keys in
# this clone's own git config, so that a key set from the feature list is written there and not to
# demo.gitconfig.
Type "git checkout -q -b blame 76905c8e && git branch -q -f main 76905c8e~" Enter
Type "git config delta-tui.general.diff-renderer.feature-keys.side-by-side S" Enter
Type "git config delta-tui.general.diff-renderer.feature-keys.line-numbers L" Enter
Type "git config delta-tui.general.diff-renderer.feature-keys.plus-minus-markers + && clear" Enter
Sleep 500ms
Show

Announce@4s "How to use delta's TUI: more features" "delta's TUI is based on https://github.com/altsem/gitu"
Type "git diff main" Sleep 500ms Enter
Wait+Screen /4 files changed/
# Side-by-side is where a session starts, without saying so.
Hide
Type "S"
Sleep 1s
Show
Sleep 1s

# scene
# To the hunk of blame.rs that replaces an if/else with a call to format::pad.
Down Sleep 300ms Tab Sleep 1s Ctrl+Down@80ms 7 Sleep 1s
Announce "? lists what you can do, here with the hunk under the cursor"
Type "?" Sleep 4s
Type "q" Sleep 1s

# scene
# In the unified view, where a copy of the screen reads as a patch, and back to side-by-side after,
# which the scenes after this one count lines in.
Type "S" Sleep 1.5s
Announce "+ toggles +/- markers, for copying a patch for an AI agent"
Type "+" Sleep 2.5s Type "+" Sleep 1.5s
Type "S" Sleep 1s

# scene
Announce "- lists delta features, your own too, to turn on and off"
Type "-" Sleep 2s
Down@300ms 2 Sleep 700ms
Announce "A letter makes it the feature's key"
Type "p" Sleep 1.5s
Escape Sleep 700ms
Type "p" Sleep 2s Type "p" Sleep 1.5s

# scene
# To the first added line of the hunk.
Down@150ms 4 Sleep 700ms
Announce "B on a diff line shows the blame of its file"
Type "B" Sleep 3s
Type "q" Sleep 1s

# scene
Announce "& shows only the files you name"
Type "&" Sleep 1s Type@80ms "src/delta.rs" Sleep 500ms Enter
Sleep 2.5s

# scene
Announce "! leaves files out"
Type "&" Sleep 700ms Ctrl+U Sleep 300ms Type@80ms "src !src/handlers" Sleep 500ms Enter
Sleep 2.5s

# scene
Announce "up and down step through what you typed before, here and in :"
Type "&" Sleep 700ms Up Sleep 1.5s Enter
Sleep 2.5s

# scene
Announce@3s "To use delta's TUI as shown in this video put the following in git config:"
Hide
Type "q" Sleep 1s
Type "clear; show_config" Enter
Sleep 1s
Show
Sleep 8s
