// Emit per-line diff metadata as an OSC sequence, so that a host application
// (e.g. lazygit) can map a rendered diff row back to its patch-space identity
// (file, type, new-file line, old-file line) without re-parsing the rendered
// output. delta restructures the diff (drops the +/- markers, conveys the side
// by color, adds gutters), so that identity cannot be recovered from the
// painted text alone -- the pager, which still has it at render time, states it.
//
// This is gated on the OSC1717_METADATA environment variable: the host
// advertises the protocol versions it understands and delta emits the highest
// mutually-understood one. When the variable is unset (i.e. delta is not running
// under such a host) nothing is emitted, so a raw terminal / less / tmux see a
// normal diff.
//
// The OSC number 1717 was chosen after auditing the OSC allocations of the major
// terminal emulators (xterm, VTE, kitty, foot, WezTerm, iTerm2, Windows Terminal,
// Ghostty, VS Code, ConEmu, urxvt); none of them interpret it, so a terminal that
// is not a participating host skips it harmlessly.

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
/// in `OSC1717_METADATA` (e.g. "V1" or "V1,V2"), or `None` when no host is
/// asking (the variable is unset) so delta stays silent.
pub fn negotiated_version() -> Option<u32> {
    static VERSION: OnceLock<Option<u32>> = OnceLock::new();
    *VERSION.get_or_init(|| pick_version(&std::env::var("OSC1717_METADATA").ok()?))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DiffType, State};

    fn emitter() -> DiffLineMetadata {
        let mut md = DiffLineMetadata {
            version: 1,
            old_line: 0,
            new_line: 0,
            file: String::new(),
            last_record: None,
        };
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
