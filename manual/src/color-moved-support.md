# `--color-moved` support

Recent versions of Git (≥ v2.17, April 2018) are able to detect moved blocks of code and style them differently from the usual removed/added lines. If you have activated this feature in Git, then Delta will automatically detect such differently-styled lines, and display them unchanged, i.e. with the raw colors it receives from Git.

To activate the Git feature, use

```gitconfig
[diff]
    colorMoved = default
```

and see the [Git documentation](https://git-scm.com/docs/git-diff#Documentation/git-diff.txt---color-movedltmodegt) for the other possible values and associated color configuration.

The `map-styles` option allows us to transform the styles that git emits for color-moved sections into delta styles.
Here's an example of using `map-styles` to assign delta styles to the raw color-moved styles output by git.
This feature allows all of git's color-moved options to be rendered using delta styles, including with syntax highlighting.

```gitconfig
[delta]
    map-styles = bold purple => syntax magenta, bold cyan => syntax blue
```

There is a pair of features provided in [themes.config](https://github.com/dandavison/delta/blob/master/themes.gitconfig) called `zebra-dark` and `zebra-light` which utilise the moved colors by displaying them as a faint background color on the affected lines while keeping syntax highlighting as the foreground color. You can enable one of these features by stacking it upon the theme you are using, like as follows

```gitconfig
[delta]
    features = my-dark-theme zebra-dark
```

<table><tr>
  <td><img width=600px src="https://user-images.githubusercontent.com/1030961/149756321-d9e885fe-206c-4e08-8c40-e0ed5969b04a.png" alt="image" /></td>
  <td><img width=600px src="https://user-images.githubusercontent.com/1030961/149756332-ce8a4e2a-7487-4880-a151-761872657a28.png" alt="image" /></td>
</tr></table>

It is also possible to reference other styles.

```gitconfig
[delta]
    features = my-color-moved-theme

[delta "my-color-moved-theme"]
    git-moved-from-style = bold purple     # An ad-hoc named style (must end in "-style")

    map-styles = "my-color-moved-theme.git-moved-from-style => red #cccccc, \
                  bold cyan => syntax #cccccc"

    # we could also have defined git-moved-to-style = bold cyan
```

To make use of that, you need to know that git is emitting "bold cyan" and "bold purple".
But that's not always obvious.
To help with that, delta now has a `--parse-ansi` mode. E.g. `git show --color=always | delta --parse-ansi` outputs something like this:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/143238872-58a40754-ae50-4a9e-ba72-07e330e520e6.png" alt="image" /></td></tr></table>

As you see above, we can now define named styles in gitconfig and refer to them in places where a style string is expected.
We can also define custom named colors in git config, and styles can reference other styles; see the [hoopoe theme](https://github.com/dandavison/delta/blob/master/themes.gitconfig#L76-L91) for an example:

```gitconfig
[delta "hoopoe"]
    green = "#d0ffd0"  # ad-hoc named color
    plus-style = syntax hoopoe.green  # refer to named color
    plus-non-emph-style = plus-style  # styles can reference other styles
```

Additionally, we can now use the 140 color names that are standard in CSS. Use `delta --show-colors` to get a demo of the available colors, as background colors to see how they look with syntax highlighting:

<table><tr><td><img width=300px src="https://user-images.githubusercontent.com/52205/143237384-246db199-ef65-4ad2-ad4e-03d07d1ea41d.png" alt="image" /></td></tr></table>
