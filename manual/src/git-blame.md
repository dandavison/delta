# Git blame

If you do not have `pager=delta` in `[core]` and only want to use delta for blame, add `blame=delta` to the `[pager]` section of your gitconfig: see the [example gitconfig](./get-started.md).
If `hyperlinks` is enabled in the `[delta]` section then each blame commit will link to the commit on GitHub/GitLab/Bitbucket/etc.
 See [hyperlinks](./hyperlinks.md).

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/141891376-1fdb87dc-1d9c-4ad6-9d72-eeb19a8aeb0b.png" alt="image" /></td></tr></table>
