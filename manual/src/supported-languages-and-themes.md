# Supported languages and themes

To list the supported languages and color themes, use `delta --list-languages` and `delta --list-syntax-themes`. To see a demo of the color themes, use `delta --show-syntax-themes`:

To add your own custom color theme, or language, please follow the instructions in the Customization section of the [bat documentation](https://github.com/sharkdp/bat/#customization):

- [Adding a custom language](https://github.com/sharkdp/bat/#adding-new-syntaxes--language-definitions)
- [Adding a custom theme](https://github.com/sharkdp/bat/#adding-new-themes)

Delta automatically recognizes custom themes and languages added to bat. You will need to install bat in order to run the `bat cache --build` command. Ideally, the version of bat you install should match the version specified in delta's [`Cargo.toml`](https://github.com/dandavison/delta/blob/main/Cargo.toml) for the delta release you're using. There are [known problems](https://github.com/dandavison/delta/issues/1712) with bat v0.18.3 and below.

The languages and color themes that ship with delta are those that ship with bat. So, to propose a new language or color theme for inclusion in delta, it would need to be a helpful addition to bat, in which case please open a PR against bat.

## Mapping filename patterns to syntaxes

Delta accepts user-defined glob → syntax-name mappings via the `--map-syntax` option. The format is `<glob-pattern>:<syntax-name>`, where `<syntax-name>` is one of the language names listed by `delta --list-languages`. The option has the same semantics as [`bat --map-syntax`](https://github.com/sharkdp/bat).

The glob uses [`globset`](https://docs.rs/globset) syntax with literal path separators and is matched case-insensitively against both the full path and the file-name component. Note that a pattern starting with `.` (e.g. `.vimrc`) matches a file exactly named `.vimrc` anywhere in the tree via the file-name check, but does not match `foo.vimrc`. To match files by extension prefix with `*`. On the command line the option may be repeated; later entries override earlier ones. The same option may also be set via the `[delta]` section of a git config file by repeating the `map-syntax` key.

Example:

```ini
[delta]
    map-syntax = *.gitconfig.local:Git Config
    map-syntax = *.zsh*:Bourne Again Shell (bash)
    map-syntax = *.vimrc.local:VimL
```

The same mappings on the command line:

```sh
delta --map-syntax '*.gitconfig.local:Git Config' \
      --map-syntax '*.zsh*:Bourne Again Shell (bash)' \
      --map-syntax '*.vimrc.local:VimL' \
```

