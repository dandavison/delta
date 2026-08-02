// Emit per-line diff metadata as an OSC sequence, so that a host application
// (e.g. lazygit) can map a rendered diff row back to its patch-space identity
// (file, type, new-file line, old-file line) without re-parsing the rendered
// output. delta restructures the diff (drops the +/- markers, conveys the side
// by color, adds gutters), so that identity cannot be recovered from the
// painted text alone -- the pager, which still has it at render time, states it.
//
// This is gated on the OSC1717 environment variable: the host advertises the
// protocol versions it understands and delta emits the highest mutually-
// understood one. When the variable is unset (i.e. delta is not running under
// such a host) nothing is emitted, so a raw terminal / less / tmux see a normal
// diff.
//
// The OSC number 1717 was chosen after auditing the OSC allocations of the major
// terminal emulators (xterm, VTE, kitty, foot, WezTerm, iTerm2, Windows Terminal,
// Ghostty, VS Code, ConEmu, urxvt); none of them interpret it, so a terminal that
// is not a participating host skips it harmlessly.

use std::io::{self, Write};
use std::sync::OnceLock;

use crate::delta::{DiffType, State};

/// Highest protocol version this build of delta knows how to emit.
const SUPPORTED_VERSION: u32 = 1;

const OSC: &str = "\x1b]1717";
const ST: &str = "\x1b\\";

/// The highest version in the host's advertised list (e.g. "V1" or "V1,V2")
/// that this build also understands, or `None` if the lists are disjoint.
fn pick_version(advertised: &str) -> Option<u32> {
    advertised
        .split(',')
        .filter_map(|v| v.trim().strip_prefix('V')?.parse::<u32>().ok())
        .filter(|v| *v <= SUPPORTED_VERSION)
        .max()
}

/// The protocol version to emit, negotiated against the host's advertised list
/// in `OSC1717` (e.g. "V1" or "V1,V2"), or `None` when no host is asking (the
/// variable is unset) so delta stays silent.
pub fn negotiated_version() -> Option<u32> {
    static VERSION: OnceLock<Option<u32>> = OnceLock::new();
    *VERSION.get_or_init(|| pick_version(&std::env::var("OSC1717").ok()?))
}

/// The handshake record: a version-only OSC 1717 (no further fields) that a
/// conforming pager emits once, as its first output, to announce it speaks the
/// protocol (see the spec, §4.4). It lets a host probe delta on an empty diff --
/// which emits no per-line records -- and tell "speaks the protocol" apart from
/// "unsupported pager". Empty when no host negotiated a version.
pub fn handshake() -> String {
    negotiated_version().map_or(String::new(), handshake_for_version)
}

fn handshake_for_version(version: u32) -> String {
    format!("{OSC};{version}{ST}")
}

/// Tracks the current old-/new-file line numbers within a hunk and formats the
/// per-line OSC sequence. One instance is held by the `Painter` (when emission
/// is negotiated) and re-seeded at every hunk header.
pub struct DiffLineMetadata {
    version: u32,
    old_line: usize,
    new_line: usize,
    file: String,
    /// The record (`type_char`, `new_line`, `old_line`) most recently emitted
    /// for a primary content line, so a wrapped continuation row can re-emit it
    /// without advancing the counters. Set by every primary line; read by the
    /// wrapped row(s) that immediately follow it.
    last_record: Option<(char, usize, Option<usize>)>,
}

impl DiffLineMetadata {
    /// Returns an emitter iff the host negotiated a protocol version.
    pub fn from_env() -> Option<Self> {
        negotiated_version().map(|version| Self {
            version,
            old_line: 0,
            new_line: 0,
            file: String::new(),
            last_record: None,
        })
    }

    /// A version-1 emitter regardless of environment. Tests cannot go through
    /// `from_env`: the negotiated version is cached in a process-wide OnceLock,
    /// so an env var set by one test would leak into all others.
    #[cfg(test)]
    pub fn v1_for_tests() -> Self {
        Self {
            version: 1,
            old_line: 0,
            new_line: 0,
            file: String::new(),
            last_record: None,
        }
    }

    /// Seed the line-number counters and file path at a hunk header. Mirrors
    /// `LineNumbersData::initialize_hunk`: the first entry is the old-file start,
    /// the last is the new-file start.
    pub fn initialize_hunk(&mut self, line_numbers: &[(usize, usize)], file: String) {
        self.old_line = line_numbers[0].0;
        self.new_line = line_numbers[line_numbers.len() - 1].0;
        self.file = file;
    }

    /// Advance the counters for `state` and return the OSC sequence to prepend
    /// to that content line, or an empty string for states that carry no
    /// metadata (headers and -- in this prototype -- combined/merge diffs). The
    /// counter arithmetic mirrors `line_numbers::linenumbers_and_styles`.
    ///
    /// A wrapped continuation row (`Hunk*Wrapped`, produced when side-by-side
    /// wraps a long line into several output rows) re-emits the record of the
    /// primary line it continues, *without* advancing the counters. delta emits
    /// each wrapped row as a distinct output line, so a host sees them as
    /// distinct lines and needs the identity on each -- otherwise acting on a
    /// continuation row, or treating the wrapped line as one block when
    /// navigating, breaks.
    pub fn osc_for_line(&mut self, state: &State) -> String {
        use State::*;
        let record = match state {
            HunkZero(DiffType::Unified, _) => {
                let record = ('c', self.new_line, None);
                self.old_line += 1;
                self.new_line += 1;
                record
            }
            HunkPlus(DiffType::Unified, _) => {
                let record = ('a', self.new_line, None);
                self.new_line += 1;
                record
            }
            HunkMinus(DiffType::Unified, _) => {
                // A deletion sits at the new-file position the new-file counter
                // currently holds (it has not advanced past the preceding
                // context/added lines); it carries both numbers.
                let record = ('d', self.new_line, Some(self.old_line));
                self.old_line += 1;
                record
            }
            HunkZeroWrapped | HunkMinusWrapped | HunkPlusWrapped => match self.last_record {
                Some(record) => record,
                None => return String::new(),
            },
            _ => return String::new(),
        };
        self.last_record = Some(record);
        let (type_char, new_line, old_line) = record;
        let old_field = old_line.map_or(String::new(), |n| n.to_string());
        format!(
            "{OSC};{version};{type_char};{new_line};{old_field};{file}{ST}",
            version = self.version,
            file = self.file,
        )
    }

    /// The `h` (hunk-header) record for the current hunk. `initialize_hunk` has
    /// just seeded `new_line` with the hunk's new-file start -- which is the
    /// hunk's first line (spec §5.2) -- so it is in hand exactly when the hunk
    /// header is rendered. `old-line` is empty on a header.
    pub fn osc_for_hunk_header(&self) -> String {
        self.header_osc('h', Some(self.new_line), &self.file)
    }

    /// The `f` (file-header) record. It carries no line numbers (spec §5.5):
    /// delta renders the file header (the boxed file name) as soon as it parses
    /// the `+++` line, *before* it has seen the first `@@`, so the file's first
    /// hunk line is not known here. The path is supplied by the caller because
    /// the emitter only learns it at the first hunk header.
    pub fn osc_for_file_header(&self, file: &str) -> String {
        self.header_osc('f', None, file)
    }

    fn header_osc(&self, type_char: char, new_line: Option<usize>, file: &str) -> String {
        let new_field = new_line.map_or(String::new(), |n| n.to_string());
        format!(
            "{OSC};{version};{type_char};{new_field};;{file}{ST}",
            version = self.version,
        )
    }
}

/// A `Write` adapter that injects `prefix` at the start of the stream and after
/// every newline. delta draws a file/hunk header as a *multi-row* decoration --
/// an underline under the file name, a box around the hunk header -- written
/// straight to the output by the `draw` functions. To tag the whole block we
/// wrap that writer so every row the decoration emits is preceded by the
/// header's record, the same "every output row carries the record" rule that
/// wrapped content rows follow (spec §6.3/§6.4).
struct OscLinePrefixer<'a> {
    inner: &'a mut dyn Write,
    prefix: &'a str,
    at_line_start: bool,
}

impl Write for OscLinePrefixer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for chunk in buf.split_inclusive(|&b| b == b'\n') {
            if self.at_line_start {
                self.inner.write_all(self.prefix.as_bytes())?;
            }
            self.inner.write_all(chunk)?;
            self.at_line_start = chunk.ends_with(b"\n");
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Run `draw`, prefixing every output row it writes with `osc` (a header
/// record). With `osc` empty the closure writes to `writer` untouched, so the
/// output stays byte-for-byte identical to stock delta.
pub fn write_with_header_osc(
    writer: &mut dyn Write,
    osc: &str,
    draw: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<()> {
    if osc.is_empty() {
        draw(writer)
    } else {
        let mut prefixer = OscLinePrefixer {
            inner: writer,
            prefix: osc,
            at_line_start: true,
        };
        draw(&mut prefixer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DiffType, State};

    fn emitter() -> DiffLineMetadata {
        let mut md = DiffLineMetadata::v1_for_tests();
        // @@ -1,3 +1,4 @@ : old start 1, new start 1.
        md.initialize_hunk(&[(1, 3), (1, 4)], "f.txt".to_owned());
        md
    }

    #[test]
    fn test_handshake_is_version_only() {
        // The handshake carries only the version (no further fields), so a host tells
        // it apart from a per-line record by field count.
        assert_eq!(handshake_for_version(1), "\x1b]1717;1\x1b\\");
    }

    #[test]
    fn test_hunk_header_record_carries_the_hunks_first_line() {
        // `h` carries the hunk's first new-file line (just seeded by
        // initialize_hunk) and no old-line. Emitting it does not advance the
        // counters: the hunk's first content line reports the same line.
        let mut md = emitter();
        assert_eq!(md.osc_for_hunk_header(), "\x1b]1717;1;h;1;;f.txt\x1b\\");
        assert_eq!(
            md.osc_for_line(&State::HunkZero(DiffType::Unified, None)),
            "\x1b]1717;1;c;1;;f.txt\x1b\\"
        );
    }

    #[test]
    fn test_file_header_record_has_no_line_numbers() {
        // `f` never carries line numbers (spec §5.5): delta draws the file
        // header before it has seen the first `@@`. The path comes from the
        // caller, because the emitter's own state is only seeded at the first
        // hunk header.
        let md = emitter();
        assert_eq!(
            md.osc_for_file_header("src/foo.rs"),
            "\x1b]1717;1;f;;;src/foo.rs\x1b\\"
        );
    }

    #[test]
    fn test_header_record_prefixes_every_row_of_a_multi_row_block() {
        // A file/hunk header is a multi-row decoration (an underlined name, a
        // box); every row of the block carries the record (spec §6.4). With no
        // record the drawn bytes pass through untouched.
        let draw = |w: &mut dyn io::Write| -> io::Result<()> {
            write!(w, "──┐\n1:│\n──┘\n")
        };

        let mut tagged = Vec::new();
        write_with_header_osc(&mut tagged, "[osc]", draw).unwrap();
        assert_eq!(
            String::from_utf8(tagged).unwrap(),
            "[osc]──┐\n[osc]1:│\n[osc]──┘\n"
        );

        let mut untouched = Vec::new();
        write_with_header_osc(&mut untouched, "", draw).unwrap();
        assert_eq!(String::from_utf8(untouched).unwrap(), "──┐\n1:│\n──┘\n");
    }

    #[test]
    fn test_wrapped_rows_reemit_the_primary_record_without_advancing() {
        let mut md = emitter();
        let zero = md.osc_for_line(&State::HunkZero(DiffType::Unified, None));
        let plus = md.osc_for_line(&State::HunkPlus(DiffType::Unified, None));
        let plus_w1 = md.osc_for_line(&State::HunkPlusWrapped);
        let plus_w2 = md.osc_for_line(&State::HunkPlusWrapped);
        let zero_after = md.osc_for_line(&State::HunkZero(DiffType::Unified, None));

        assert_eq!(zero, "\x1b]1717;1;c;1;;f.txt\x1b\\");
        assert_eq!(plus, "\x1b]1717;1;a;2;;f.txt\x1b\\");
        // Continuation rows re-emit the addition's record verbatim.
        assert_eq!(plus_w1, plus);
        assert_eq!(plus_w2, plus);
        // The wrapped rows must not advance the counters: the next content line
        // is new-line 3, not 5.
        assert_eq!(zero_after, "\x1b]1717;1;c;3;;f.txt\x1b\\");
    }

    #[test]
    fn test_wrapped_deletion_reemits_both_line_numbers() {
        let mut md = emitter();
        let minus = md.osc_for_line(&State::HunkMinus(DiffType::Unified, None));
        let minus_w = md.osc_for_line(&State::HunkMinusWrapped);
        assert_eq!(minus, "\x1b]1717;1;d;1;1;f.txt\x1b\\");
        assert_eq!(minus_w, minus);
    }

    #[test]
    fn test_wrapped_row_with_no_preceding_primary_emits_nothing() {
        let mut md = emitter();
        assert_eq!(md.osc_for_line(&State::HunkZeroWrapped), "");
    }

    #[test]
    fn test_pick_version() {
        assert_eq!(pick_version("V1"), Some(1));
        assert_eq!(pick_version("V1,V2"), Some(1)); // we cap at SUPPORTED_VERSION
        assert_eq!(pick_version("V2,V3"), None); // disjoint with what we emit
        assert_eq!(pick_version(""), None);
        assert_eq!(pick_version("garbage"), None);
    }

    #[test]
    fn test_unified_context_addition_deletion_sequence() {
        // A modification hunk in normal (unified) mode: one context line, two
        // deletions, two additions, then a trailing context line. Two consecutive
        // deletions share a new-line and are told apart only by old-line; a
        // deletion sits at the new-file position the counter currently holds,
        // while context and additions advance it.
        let mut md = emitter();
        assert_eq!(
            md.osc_for_line(&State::HunkZero(DiffType::Unified, None)),
            "\x1b]1717;1;c;1;;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkMinus(DiffType::Unified, None)),
            "\x1b]1717;1;d;2;2;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkMinus(DiffType::Unified, None)),
            "\x1b]1717;1;d;2;3;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkPlus(DiffType::Unified, None)),
            "\x1b]1717;1;a;2;;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkPlus(DiffType::Unified, None)),
            "\x1b]1717;1;a;3;;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkZero(DiffType::Unified, None)),
            "\x1b]1717;1;c;4;;f.txt\x1b\\"
        );
    }

    #[test]
    fn test_initialize_hunk_reseeds_the_counters() {
        // A second hunk re-seeds the old/new starts, so the counters jump to the
        // new hunk rather than continuing from the previous one.
        let mut md = emitter();
        md.initialize_hunk(&[(20, 3), (25, 4)], "f.txt".to_owned());
        assert_eq!(
            md.osc_for_line(&State::HunkZero(DiffType::Unified, None)),
            "\x1b]1717;1;c;25;;f.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkMinus(DiffType::Unified, None)),
            "\x1b]1717;1;d;26;21;f.txt\x1b\\"
        );
    }

    #[test]
    fn test_whole_file_deletion_sits_at_new_line_zero() {
        // A deleted file's hunk header is `@@ -1,N +0,0 @@`, so the new-file
        // start is 0 and every deletion reports new-line 0.
        let mut md = emitter();
        md.initialize_hunk(&[(1, 3), (0, 0)], "gone.txt".to_owned());
        assert_eq!(
            md.osc_for_line(&State::HunkMinus(DiffType::Unified, None)),
            "\x1b]1717;1;d;0;1;gone.txt\x1b\\"
        );
        assert_eq!(
            md.osc_for_line(&State::HunkMinus(DiffType::Unified, None)),
            "\x1b]1717;1;d;0;2;gone.txt\x1b\\"
        );
    }

    #[test]
    fn test_path_may_contain_semicolons() {
        // The path is the last field, so it may itself contain ';': a host splits
        // into at most five fields and takes the remainder as the path.
        let mut md = emitter();
        md.initialize_hunk(&[(1, 1), (1, 1)], "weird;name.txt".to_owned());
        assert_eq!(
            md.osc_for_line(&State::HunkPlus(DiffType::Unified, None)),
            "\x1b]1717;1;a;1;;weird;name.txt\x1b\\"
        );
    }
}
