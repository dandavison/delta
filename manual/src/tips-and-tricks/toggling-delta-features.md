To toggle features such as `side-by-side` on and off, you need to *not* turn on `line-numbers` or `side-by-side` etc in your main delta config (`~/.gitconfig`). Then, one approach is to use the [`DELTA_FEATURES](../features-named-groups-of-settings.md)` environment variable:

```sh
export DELTA_FEATURES=+side-by-side
```

and to undo that:

```sh
export DELTA_FEATURES=+
```

To make that convenient, you could use this shell function:

```sh
delta-toggle() {
    eval "export DELTA_FEATURES='$(-delta-features-toggle $1 | tee /dev/stderr)'"
}
```
where `-delta-features-toggle` is this Python script:
[https://github.com/dandavison/tools/blob/main/python/-delta-features-toggle](https://github.com/dandavison/tools/blob/main/python/-delta-features-toggle).


Then

```
delta-toggle    # shows current features
delta-toggle s  # toggles side-by-side
delta-toggle l  # toggles line-numbers
```

(It might make sense to add something like this Python script to `delta` itself.)

Another approach is to use git aliases, e.g.

```gitconfig
[alias]
    diff-side-by-side = -c delta.features=side-by-side diff
```
