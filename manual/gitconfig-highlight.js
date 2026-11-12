// mdbook's highlight.js has no gitconfig language, but gitconfig's syntax is
// ini's. book.js, which runs first, leaves the gitconfig blocks plain, so they
// are highlighted again here.
hljs.registerAliases("gitconfig", { languageName: "ini" });
document
  .querySelectorAll("code.language-gitconfig")
  .forEach((block) => hljs.highlightBlock(block));
