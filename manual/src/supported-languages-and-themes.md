# Supported languages and themes

To list the supported languages and color themes, use `delta --list-languages` and `delta --list-syntax-themes`. To see a demo of the color themes, use `delta --show-syntax-themes`:

To add your own custom color theme, or language, please follow the instructions in the Customization section of the [bat documentation](https://github.com/sharkdp/bat/#customization):

- [Adding a custom language](https://github.com/sharkdp/bat/#adding-new-syntaxes--language-definitions)
- [Adding a custom theme](https://github.com/sharkdp/bat/#adding-new-themes)

Delta automatically recognizes custom themes and languages added to bat. You will need to install bat in order to run the `bat cache --build` command.

The languages and color themes that ship with delta are those that ship with bat. So, to propose a new language or color theme for inclusion in delta, it would need to be a helpful addition to bat, in which case please open a PR against bat.
