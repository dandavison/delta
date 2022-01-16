# How delta works

If you configure delta in gitconfig as above, then git will automatically send its output to delta.
Delta in turn passes its own output on to a "real" pager.
Note that git will only send its output to delta if git believes that its output is going to a terminal (a "tty") for a human to read.
In other words, if you do something like `git diff | grep ...` then you don't have to worry about delta changing the output from git, because delta will never be invoked at all.
If you need to force delta to be invoked when git itself would not invoke it, then you can always pipe to delta explicitly.
For example, `git diff | delta | something-that-expects-delta-output-with-colors` (in this example, git's output is being sent to a pipe, so git itself will not invoke delta).
In general however, delta's output is intended for humans, not machines.

The pager that delta uses is determined by consulting the following environment variables (in this order):

- `DELTA_PAGER`
- `PAGER`

If neither is set, delta's fallback is `less -R`.

The behavior of delta's default pager, `less`, can be controlled using the `LESS` environment variable.
It may contain any of the `less` command line options and/or interactive less-commands (prefixed by a leading `+` sign; these are executed every time right after less is launched).
For full documentation of `less` configuration options, please see the `less(1)` [manual](https://jlk.fjfi.cvut.cz/arch/manpages/man/core/less/less.1.en).

In addition to `DELTA_PAGER`, and `PAGER`, delta currently also consults `$BAT_PAGER` (with priority between the two).
However, this is deprecated: please use `DELTA_PAGER` instead.
No other [`bat`](https://github.com/sharkdp/bat) environment variables are used by delta.

If you are interested in the implementation of delta, please see [ARCHITECTURE.md](https://github.com/dandavison/delta/blob/master/ARCHITECTURE.md).
