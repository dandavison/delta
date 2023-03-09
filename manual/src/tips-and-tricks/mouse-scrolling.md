# Mouse scrolling

If mouse scrolling isn't working correctly, ensure that you have the most recent version of `less`.

- For Windows you can download from https://github.com/jftuga/less-Windows/releases/latest
- For Mac you can install `brew install less; brew link less`

Alternatively try setting your `DELTA_PAGER` environment variable to (at least) `less -R`. See [issue #58](https://github.com/dandavison/delta/issues/58). See also [bat README / "Using a different pager"](https://github.com/sharkdp/bat#using-a-different-pager), since the `DELTA_PAGER` environment variable functions very similarly for delta.
