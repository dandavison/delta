# diff-highlight and diff-so-fancy emulation

Use `--diff-highlight` or `--diff-so-fancy` to activate the respective emulation mode.

You may want to know which delta configuration values the emulation mode has selected, so that you can adjust them. To do that, use e.g. `delta --diff-so-fancy --show-config`:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/86271121-5abe4c80-bb9a-11ea-950a-7c79502267d5.png" alt="image" /></td></tr></table>

[diff-highlight](https://github.com/git/git/tree/master/contrib/diff-highlight) is a perl script distributed with git that allows within-line edits to be identified and highlighted according to colors specified in git config. [diff-so-fancy](https://github.com/so-fancy/diff-so-fancy) builds on diff-highlight, making various additional improvements to the default git diff output. Both tools provide very helpful ways of viewing diffs, and so delta provides emulation modes for both of them.

The within-line highlighting rules employed by diff-highlight (and therefore by diff-so-fancy) are deliberately simpler than Delta's Levenshtein-type edit inference algorithm (see discussion in the [diff-highlight README](https://github.com/git/git/tree/master/contrib/diff-highlight)). diff-highlight's rules could be added to delta as an alternative highlighting algorithm, but that hasn't been done yet.
