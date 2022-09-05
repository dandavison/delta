# Usage

The main way to use delta is to configure it as the pager for git: see [Configuration](./configuration.md).

Delta can also be used as a shorthand for diffing two files, even if they are not in a git repo: the following two commands do the same thing:

```
delta /somewhere/a.txt /somewhere/else/b.txt

git diff /somewhere/a.txt /somewhere/else/b.txt
```

You can also use [process substitution](https://en.wikipedia.org/wiki/Process_substitution) shell syntax with delta, e.g.

```
delta <(sort file1) <(sort file2)
```

In addition to git output, delta handles standard unified diff format, e.g. `diff -u a.txt b.txt | delta`.

For Mercurial, you can add delta, with its command line options, to the `[pager]` section of `.hgrc`.
