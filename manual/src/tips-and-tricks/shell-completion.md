# Generating completion files for various shells

Delta can generate completion files for various shells.
Use the `--generate-completion` subcommand to print the completion script to stdout:

```sh
delta --generate-completion <SHELL>
```
<SHELL> should be replaced with the lowercase name of the shell for which the script is to be generated.
Currently bash, elvish, fish, powershell and zsh are supported.

The completion files in `etc/completion` were also generated with this function and may not be up-to-date.
