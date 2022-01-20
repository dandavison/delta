# Custom themes

A "theme" in delta is just a collection of settings grouped together in a named [feature](./features-named-groups-of-settings.md). One of the available settings is `syntax-theme`: this dictates the colors and styles that are applied to foreground text by the syntax highlighter. Thus the concept of "theme" in delta encompasses not just the foreground syntax-highlighting color theme, but also background colors, decorations such as boxes and under/overlines, etc.

The delta git repo contains a [collection of themes](https://github.com/dandavison/delta/blob/master/themes.gitconfig) created by users. These focus on the visual appearance: colors etc. If you want features like `side-by-side` or `navigate`, you would set that yourself, after selecting the color theme. To use the delta themes, clone the delta repo (or [download](https://raw.githubusercontent.com/dandavison/delta/master/themes.gitconfig) the raw `themes.gitconfig` file) and add the following entry in your gitconfig:

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
