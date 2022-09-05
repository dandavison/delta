# How delta works

If you configure delta in gitconfig as above, then git will automatically send its output to delta.
Delta in turn passes its own output on to a "real" pager.
Note that git will only send its output to delta if git believes that its output is going to a terminal (a "tty") for a human to read.
In other words, if you do something like `git diff | grep ...` then you don't have to worry about delta changing the output from git, because delta will never be invoked at all.
If you need to force delta to be invoked when git itself would not invoke it, then you can always pipe to delta explicitly.
For example, `git diff | delta | something-that-expects-delta-output-with-colors` (in this example, git's output is being sent to a pipe, so git itself will not invoke delta).
In general however, delta's output is intended for humans, not machines.

If you are interested in the implementation of delta, please see [ARCHITECTURE.md](https://github.com/dandavison/delta/blob/master/ARCHITECTURE.md).
