# Custom themes

A "theme" in delta is just a collection of settings grouped together in a named [feature](./features-named-groups-of-settings.md). One of the available settings is `syntax-theme`: this dictates the colors and styles that are applied to foreground text by the syntax highlighter. Thus the concept of "theme" in delta encompasses not just the foreground syntax-highlighting color theme, but also background colors, decorations such as boxes and under/overlines, etc.

The delta git repo contains a [collection of themes](https://github.com/dandavison/delta/blob/main/themes.gitconfig) created by users. These focus on the visual appearance: colors etc. If you want features like `side-by-side` or `navigate`, you would set that yourself, after selecting the color theme.

To browse themes, use `delta --show-themes`, or browse the list of theme PRs: <https://github.com/dandavison/delta/commits/main/themes.gitconfig>. (The PRs nearly always have screenshots in them.)

To use the delta themes, clone the delta repo (or [download](https://raw.githubusercontent.com/dandavison/delta/main/themes.gitconfig) the raw `themes.gitconfig` file) and add the following entry in your gitconfig:

```gitconfig
[include]
    path = /PATH/TO/delta/themes.gitconfig
```

Then, add your chosen color theme to your features list, e.g.

```gitconfig
[delta]
    features = collared-trogon
    side-by-side = true
    ...
```

Note that this terminology differs from [bat](https://github.com/sharkdp/bat): bat does not apply background colors, and uses the term "theme" to refer to what delta calls `syntax-theme`. Delta does not have a setting named "theme": a theme is a "feature", so one uses `features` to select a theme.

## Automatic light/dark themes

Delta detects whether your terminal has a light or dark background (controlled by the
`--detect-dark-light` option). Use the `dark-features` and `light-features` settings to
activate different features depending on the detected mode. Each
takes a space-separated feature list, just like `features`, and is applied only when delta is
in the corresponding mode:

```gitconfig
[delta]
    features       = side-by-side   # always applied
    dark-features  = collared-trogon
    light-features = woolly-mammoth
```

With auto-detection, delta now activates `collared-trogon` on a dark background and
`woolly-mammoth` on a light one, while `side-by-side` is applied in both. This also works when
the mode is set explicitly with `--dark` / `--light` (or `delta.dark` / `delta.light`).

Because the values are ordinary feature lists, they can reference any feature — not only color
themes. For example you can enable `side-by-side` only in dark mode, or use a different
`syntax-theme` per mode. The per-mode lists take priority over a plain `features` list. Like
`features`, they are part of the git-config feature list, so passing `--features` on the
command line replaces them. To add a feature for a single invocation while keeping the
configured features (including the per-mode lists), use the additive `DELTA_FEATURES`
environment variable, e.g. `DELTA_FEATURES=+side-by-side`.
