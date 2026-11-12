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

To use one theme on a dark background and another on a light one, list them in `dark-features` and `light-features`:

```gitconfig
[delta]
    dark-features = collared-trogon
    light-features = hoopoe
```

Delta decides whether the background is dark or light as it does without these: from `dark` or `light` in your config or on the command line, else from your terminal's background, else from your `syntax-theme`, else dark. A feature in `dark-features` or `light-features` cannot change that decision, even if it sets `dark`, `light` or a `syntax-theme` for the other mode. These features rank above those in `features`, and below those on the command line. `--features`, or `DELTA_FEATURES` without a leading `+`, replaces them, as it replaces `features`. Any feature can be listed, not only a theme.

Note that this terminology differs from [bat](https://github.com/sharkdp/bat): bat does not apply background colors, and uses the term "theme" to refer to what delta calls `syntax-theme`. Delta does not have a setting named "theme": a theme is a "feature", so one uses `features` to select a theme.
