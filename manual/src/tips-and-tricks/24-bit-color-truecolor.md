# 24 bit color (truecolor)

Delta looks best if your terminal application supports 24 bit colors. See <https://github.com/termstandard/colors#readme>. For example, on MacOS, iTerm2 supports 24-bit colors but Terminal.app does not.

If your terminal application does not support 24-bit color, delta will still work, by automatically choosing the closest color from those available. See the `Colors` section of the help output below.

If 24-bit color is supported by your terminal emulator, then it should have set the `COLORTERM` env var to the value `truecolor` (or `24bit`). If necessary, you can explicitly enable true color, either by using `--true-color=always` or by adding the following to your configuration file:

```gitconfig
[delta]
    true-color = always
```
