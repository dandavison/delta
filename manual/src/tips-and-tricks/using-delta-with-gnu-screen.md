# Using Delta with GNU Screen

True color output in GNU Screen is currently only possible when using a development build, as support for it is not yet implemented in the (v4) release versions. A snapshot of the latest Git trunk can be obtained via https://git.savannah.gnu.org/cgit/screen.git/snapshot/screen-master.tar.gz - the required build steps are described in the `src/INSTALL` file. After installing the program, 24-bit color support can be activated by including `truecolor on` in either the system's or the user's `screenrc` file.

When working in Screen without true color output, it might be that colors supposed to be different look the same in XTerm compatible terminals. If that is the case, make sure the following settings are included in your `screenrc` file:

```Shell
term screen-256color
termcapinfo xterm 'Co#256:AB=\E[48;5;%dm:AF=\E[38;5;%dm'  # ANSI (256-color) patterns - AB: background, AF: foreground
attrcolor b ".I"                                          # use bright colors for bold text
```

If despite having those settings you still only get a limited set of colors, your build of Screen might have been configured without the `--enable-colors256` flag. If this is the case, you have two options :

- If available for your OS, get a different package of Screen. Otherwise
- Build your own binary :
  - Download and extract a release tarball from https://ftp.gnu.org/gnu/screen/
  - `cd` into the newly extracted folder
  - Follow the instructions in the `INSTALL` file, and when running the `./configure` command apply the `--enable-colors256` flag.
