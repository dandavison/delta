If you're using tmux, it's worth checking that 24 bit color is working correctly. For example, run a color test script like [this one](https://gist.githubusercontent.com/lifepillar/09a44b8cf0f9397465614e622979107f/raw/24-bit-color.sh), or one of the others listed [here](https://gist.github.com/XVilka/8346728). If you do not see smooth color gradients, see the discussion at [tmux#696](https://github.com/tmux/tmux/issues/696). The short version is you need something like this in your `~/.tmux.conf`:

```Shell
set -ga terminal-overrides ",xterm-256color:Tc"
```

and you may then need to quit tmux completely for it to take effect.
