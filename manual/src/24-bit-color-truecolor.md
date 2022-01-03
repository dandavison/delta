# 24 bit color (truecolor)

Delta looks best if your terminal application supports 24 bit colors. See https://github.com/termstandard/colors#readme. For example, on MacOS, iTerm2 supports 24-bit colors but Terminal.app does not.

If your terminal application does not support 24-bit color, delta will still work, by automatically choosing the closest color from those available. See the `Colors` section of the help output below.

If you're using tmux, it's worth checking that 24 bit color is working correctly. For example, run a color test script like [this one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh), or one of the others listed [here](https://gist.github.com/XVilka/8346728). If you do not see smooth color gradients, see the discussion at [tmux#696](https://github.com/tmux/tmux/issues/696). The short version is you need something like this in your `~/.tmux.conf`:

```Shell
set -ga terminal-overrides ",xterm-256color:Tc"
```

and you may then need to quit tmux completely for it to take effect.

True color output in GNU Screen is currently only possible when using a development build, as support for it is not yet implemented in the (v4) release versions. A snapshot of the latest Git trunk can be obtained via https://git.savannah.gnu.org/cgit/screen.git/snapshot/screen-master.tar.gz - the required build steps are described in the `src/INSTALL` file. After installing the program, 24-bit color support can be activated by including `truecolor on` in either the system's or the user's `screenrc` file.
