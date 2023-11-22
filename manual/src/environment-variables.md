# Environment variables

## Git environment variables

The `GIT_PAGER` environment variable must either not be set at all, or set to the value `delta` (you can add argument here if you want; this env var plays the same role as the `core.pager` config entry).

## Pager environment variables

A pager is a program that accepts many lines of text as input, and displays them one screenful at a time.
The standard pager is [less](https://linux.die.net/man/1/less), and this is what delta uses by default (it's also what [`bat`](https://github.com/sharkdp/bat) uses).
Therefore:

1. It is very important that you are using a recent version of less. In particular, on Windows, the installed version of less is often broken and it is usually necessary to install it yourself or use the version of less that is installed with git on Windows.

2. The command line flags passed to `less` are important, and there are some environment variables that affect these (see below). By default, delta will try to ensure that they are sensible.

3. When delta is displaying lengthy output, anything you do with the keyboard or mouse is actually received by less, and it is worth looking at less documentation (`less --help` or `man less` or [online](https://linux.die.net/man/1/less)) to discover what you can do.

The exact command that `delta` uses to start its pager is taken from one of the following environment variables (in this order):

- `DELTA_PAGER`
- `BAT_PAGER`
- `PAGER`

Delta does not use `bat` when it is running, and delta users do not need to
install bat. (Delta does use the `bat` Rust library for its syntax highlighting
themes and language definitions, and for launching the pager, which is why the
`BAT_PAGER` environment variable is honored).

If none of these is set, delta uses `less -R`, and you should always include `-R` if you are setting these environment variables yourself.

In addition to those `*PAGER` environment variables, the behavior of `less` is also affected by the `LESS` environment variable (see `man less` or [online documentation](https://linux.die.net/man/1/less)). This env var can contain command line options and/or interactive less-commands (prefixed by a leading `+` sign; these are executed every time right after less is launched).

## Delta-specific environment variables

To temporarily activate and inactivate delta features, you can use `DELTA_FEATURES`, e.g.

```sh
export DELTA_FEATURES='+side-by-side my-feature'
```

(The `+` means "add these features to those configured in git config".)

The `DELTA_PAGER` env var is described above.
