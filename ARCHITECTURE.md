The purpose of delta is to transform input received from git, diff, git blame, grep, etc to produce visually appealing output, including by syntax highlighting code.

### Initialization

Delta [reads](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/main.rs#L83) user options from `[delta]` sections in [`.gitconfig`](https://git-scm.com/docs/git-config), and from the command line.

### Input

Delta [reads from stdin](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/main.rs#L138), hence one can do e.g. `git diff | delta`.
Note that when git's stdout is sent to a pipe, (a) git does not emit [ANSI color escape sequences](https://en.wikipedia.org/wiki/ANSI_escape_code) unless `--color=always`, and (b) git does not start its own pager process.

Users typically configure git to use delta as its pager.
In that case, git sends its stdout to delta behind the scenes (_with_ ANSI color escape sequences), without the user needing to pipe it explicitly.

### Parsing the input

Delta [parses input](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/delta.rs#L81) using a state machine in which
the states correspond to semantically distinct sections of the input (e.g. `HunkMinus` means that we are in a removed line in a diff hunk).
The core dispatching loop is [here](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/delta.rs#L115-L129).

### Output

Delta [creates](https://github.com/dandavison/delta/blob/114ae670223520657208501a3245a3b4261c1093/src/main.rs#L125) a child pager process (`less`) and writes its output to the stdin of the pager process.
Delta's `navigate` feature is implemented by constructing an appropriate regex and passing it as an argument to `less`.

## Core utility data structures

- [`config::Config`](https://github.com/dandavison/delta/blob/5dc0d6ef7e37a565b06d794b50fcc763079f9ed7/src/config.rs#L59-L143)
  This is a struct with many fields corresponding to all user options and miscellaneous other useful things.
  It might be possible to store it globally, but currently the code passes references to it around the call stack.

- [`paint::Painter`](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/paint.rs#L24-L36)
  This struct holds the syntax highlighter, and a writable output stream (connected to the stdin of the child `less` process).
  It also holds two line buffers: one to store all the removed ("minus") lines encountered in a single diff hunk, and one to hold the added ("plus") lines.

## Handling diff hunk lines

Here we will follow one code path in detail: [handling diff hunk lines](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/handlers/hunk.rs#L27) (removed/unchanged/added).
This is the most important, and most complex, code path.

Recall that git diff output contains multiple diff "hunks".
A hunk is a sequence of diff lines describing the changes among some lines of code that are close together in the same file.
A git diff may have many hunks, from multiple files (and therefore multiple languages).
Within a hunk, there are sequences of consecutive removed and/or added lines ("subhunks"), separated by unchanged lines.
(The term "hunk" is standard; the term "subhunk" is specific to delta.)

The handler function that is called when delta process a hunk line is [`handle_hunk_line`](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/handlers/hunk.rs#L27).
This function stores the line in a buffer (one buffer for minus lines and one for plus lines): the processing work is not done until we get to the end of the subhunk.

Now, we are at the end of a subhunk, and we have a sequence of minus lines, and a sequence of plus lines.

<table><tr><td><img width=1000px src="https://user-images.githubusercontent.com/52205/143171872-64f41fe1-9968-48c7-86e8-dba9303a54e2.png" alt="image" /></td></tr></table>

Delta [processes a subhunk](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/paint.rs#L163) as follows:

1. **Compute syntax styles for the subhunk**

   We [call](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/paint.rs#L164-L173) the [syntect](https://github.com/trishume/syntect) library to compute syntax highlighting styles for each of the minus lines, and each of the plus lines, if the minus/plus styles specify syntax highlighting.
   The language used for syntax-highlighting is determined by the filename in the diff.
   For a single line, the result is an array of `(style, substring)` pairs. Each pair specifies the foreground (text) color to be applied to a substring of the line (for example, a language keyword, or a string literal).

2. **Compute diff styles for the subhunk**

   Again, the [call](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/paint.rs#L174-L175) yields, for each line, an array of `(style, substring)` pairs.
   Each pair represents foreground and background colors to be applied to a substring of the line, as specified by delta's `*-style` options.

   In order to compute the array of style sections, the call has to (1) infer the correct alignment of minus and plus lines, and (2) for each such "homologous pair", infer the edit operations that transformed the minus line into the plus line (see [within-line-diff-algorithm](#within-line-diff-algorithm)).

   For example, for a minus line, we may have inferred that the line has a homologous plus line, and that a word has been deleted.
   By default, delta applies a bright red background color to such a word and lets the foreground color be determined by the terminal emulator default foreground color (`minus-emph-style = normal "#901011"`).
   On the other hand, for an added word, delta by default applies a bright green background color, and specifies that the foreground color should come from the syntax highlighting styles (`plus-emph-style = syntax "#006000"`).

3. **Process subhunk lines for side-by-side or unified output**

   At this point we have a collection of lines corresponding to a subhunk and, for each line, a specification of how syntax styles and diff styles are applied to substrings of the line. These data structures are [processed differently](https://github.com/dandavison/delta/blob/3e21f00765794f7a4e955826a1612b49f1723bfd/src/paint.rs#L177-L230) according to whether unified or side-by-side diff display has been requested.

4. **Superimpose syntax and diff styles for a line**

   Before we can output a line of code we need to take the two arrays of `(style, substring)` pairs and compute a single output array of `(style, substring)` pairs, such that the output array represents the diff styles, but with foreground colors taken from the syntax highlighting, where appropriate.
   The call is [here](https://github.com/dandavison/delta/blob/1e1bd6b6b96a3515fd7c70d6b252a25eb9807dc7/src/paint.rs#L490-L495).

5. **Output a line with styles converted to ANSI color escape sequences**

   The `style` structs that delta uses are implemented by the [`ansi_term`](https://github.com/ogham/rust-ansi-term) library.
   Individual substrings are [painted](https://github.com/dandavison/delta/blob/3e21f00765794f7a4e955826a1612b49f1723bfd/src/paint.rs#L507) with their assigned style, and [concatenated](https://github.com/dandavison/delta/blob/3e21f00765794f7a4e955826a1612b49f1723bfd/src/paint.rs#L514) to form a utf-8 string containing ANSI color escape sequences.

## Within-line diff algorithm

There is currently only one within-line diff algorithm implemented.
This [considers](https://github.com/dandavison/delta/blob/3e21f00765794f7a4e955826a1612b49f1723bfd/src/edits.rs#L41-L43) all possible pairings for a given line and for each one, [computes](https://github.com/dandavison/delta/blob/3e21f00765794f7a4e955826a1612b49f1723bfd/src/edits.rs#L48-L56) the minimum number of edit operations between the candidate pair.
The inferred pairing is the one with the smallest edit distance.
(The number of comparisons is constrained by the possible interleavings, and furthermore a greedy heuristic is used, so that the number of comparisons is not quadratic).

## Features

Delta features such as `line-numbers`, `side-by-side`, `diff-so-fancy`, etc can be considered to consist of (a) some feature-specific implementation code, and (b) a collection of key-value pairs specifying the values that certain delta options should take if that feature is enabled.
Accordingly, each such "feature" is implemented by a separate module under [`src/features/`](https://github.com/dandavison/delta/tree/master/src/features).
Each of these modules must export a function named `make_feature` whose job is to return key-value pairs for updating the user options.

## Common terms used in the code

|                  |                                                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `minus`          | a removed line in a diff hunk (i.e. the lines starting with `-`)                                                             |
| `zero`           | an unchanged line in a diff hunk                                                                                             |
| `plus`           | an added line in a diff hunk (i.e. the lines starting with `+`)                                                              |
| `style`          | a struct specifying foreground colors, background colors, and other attributes such as boldness, derived from ANSI sequences |
| `style_sections` | an array of `(style, section)` tuples                                                                                        |
| `paint`          | to take a string without ANSI color sequences and return a new one with ANSI color sequences                                 |
| `hunk`           | a [diff hunk](https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html)                                 |
| `subhunk`        | a consecutive sequence of minus and/or plus lines, without any zero line                                                     |
