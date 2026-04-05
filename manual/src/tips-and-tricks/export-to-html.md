# Save output with colors to HTML/PDF etc

Install [aha](https://github.com/theZiz/aha).

```sh
git show \
    | delta --no-gitconfig --file-decoration-style blue --hunk-header-decoration-style blue \
    | aha \
    > /tmp/diff.html
```

Now open `/tmp/diff.html` in a web browser, print to PDF, etc.

Remove the `--no-gitconfig` above to use your own delta style, but note that `aha` does not handle hyperlinks or decoration boxes etc.
