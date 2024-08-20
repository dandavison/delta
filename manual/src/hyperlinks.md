# Hyperlinks

Delta uses [terminal hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda) to turn line numbers, file paths, commit hashes, etc into clickable links, as long as your terminal emulator supports it. Enable the feature with

```gitconfig
[delta]
    hyperlinks = true
```

Commit hashes link to GitHub/GitLab/Bitbucket (use `hyperlinks-commit-link-format` for full control).

The links on line numbers (in grep output, as well as diffs) are particularly interesting: with a little bit of effort, they can be made to open your editor or IDE at the correct line.
Use `hyperlinks-file-link-format` to construct the correct URL for your system.
For VSCode and JetBrains IDEs this is easy, since they support their own special URL protocols. Here are examples:

```gitconfig
[delta]
    hyperlinks = true
    hyperlinks-file-link-format = "vscode://file/{path}:{line}"
    # hyperlinks-file-link-format = "idea://open?file={path}&line={line}"
    # hyperlinks-file-link-format = "pycharm://open?file={path}&line={line}"
```

Zed also supports its own URL protocol, and probably others.

If your editor does not have its own URL protocol, then there are still many possibilities, although they may be more work.

- The easiest is probably to write a toy HTTP server (e.g. in [Python](https://docs.python.org/3/library/http.server.html)) that opens the links in the way that you need. Then your delta config would look something like
    ```gitconfig
    [delta]
    hyperlinks = true
    hyperlinks-file-link-format = "http://localhost:8000/open-in-editor?path={path}&line={line}"
    # Now write an HTTP server that handles those requests by opening your editor at the file and line
    ```

    Here's some Python code that could be used as a starting point:
    ```python
    from http.server import BaseHTTPRequestHandler, HTTPServer
    from pathlib import Path
    from subprocess import call
    from urllib.parse import parse_qs, urlparse


    class OpenInEditor(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path.startswith("/open-in-editor"):
                query = parse_qs(urlparse(self.path).query)
                [path], [line] = query["path"], query["line"]
                # TODO: You might need to change this to construct the correct root directory for the
                # project that the file is in, so that your IDE opens in the project workspace.
                cwd = Path(path).parent
                # TODO: Replace with the appropriate command for your editor
                call(["code", "-g", f"{path}:{line}"], cwd=cwd)
                self.send_response(200)
            else:
                self.send_response(404)
            self.end_headers()


    print("Starting open-in-editor server on port 8000...")
    HTTPServer(("", 8000), OpenInEditor).serve_forever()
    ```

- Another possibility is to register a custom protocol with your OS (like VSCode does) that invokes a script to open the file. [dandavison/open-in-editor](https://github.com/dandavison/open-in-editor) is a project that aimed to do that and may be helpful. However, registering the protocol with your OS can be frustrating, depending on your appetite for such things. If you go this route, your delta configuration would look like
    ```gitconfig
    [delta]
    hyperlinks = true
    hyperlinks-file-link-format = "my-file-line-protocol://{path}:{line}"
    # Now configure your OS to handle "my-file-line-protocol" URLs!
    ```
- Finally, you can just use traditional `file://` links (making sure your OS is configured to use the correct editor). But then your editor won't open the file at the correct line, which would be missing out on something very useful.