# difftastic demo

`make difftastic-demo` builds delta, builds a git repo of examples at `/tmp/difftastic-demo`
(`DIFFTASTIC_DEMO_DIR` sets another place), and opens delta's TUI on its log. It needs `difft`
(difftastic) on `PATH`.

Each commit after the first changes one example, and its message says what difftastic shows for the
change and what a line diff shows. Press enter on a commit to open it, and `*` to switch between
git's view and difftastic's.

`examples/` holds each example's before and after versions and its commit message; `make-repo`
builds the repo from them. Most come from difftastic's README, homepage and manual, and their
messages link to where.
