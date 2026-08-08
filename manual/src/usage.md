# Usage

The main way to use delta is to configure it as the pager for git: see [Configuration](./configuration.md).

Delta can also be used as a shorthand for diffing two files, even if they are not in a git repo: the following two commands do the same thing:

```sh
delta /somewhere/a.txt /somewhere/else/b.txt

git diff /somewhere/a.txt /somewhere/else/b.txt
```

You can also use [process substitution](https://en.wikipedia.org/wiki/Process_substitution) shell syntax with delta, e.g.

```sh
delta <(sort file1) <(sort file2)
```

In addition to git output, delta handles standard unified diffs when you pipe them in:

```sh
diff -u a.txt b.txt | delta
git diff | delta
git show HEAD | delta --side-by-side
```

That works without changing `~/.gitconfig`. Use `--no-gitconfig` if you want built-in defaults only (ignore any `[delta]` settings already in git config):

```sh
git diff | delta --no-gitconfig
```

For permanent setup (pager, interactive diffFilter, etc.), see [Configuration](./configuration.md).

For Mercurial, you can add delta, with its command line options, to the `[pager]` section of `.hgrc`.
