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

/// Tracks the current old-/new-file line numbers within a hunk and formats the
/// per-line OSC sequence. One instance is held by the `Painter` (when emission
/// is negotiated) and re-seeded at every hunk header.
pub struct DiffLineMetadata {
    version: u32,
    old_line: usize,
    new_line: usize,
    file: String,
}

impl DiffLineMetadata {
    /// Returns an emitter iff the host negotiated a protocol version.
    pub fn from_env() -> Option<Self> {
        negotiated_version().map(|version| Self {
            version,
            old_line: 0,
            new_line: 0,
            file: String::new(),
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
    /// metadata (headers, wrapped continuations, and -- in this prototype --
    /// combined/merge diffs). The counter arithmetic mirrors
    /// `line_numbers::linenumbers_and_styles`.
    pub fn osc_for_line(&mut self, state: &State) -> String {
        use State::*;
        let (type_char, new_line, old_line) = match state {
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
            _ => return String::new(),
        };
        let old_field = old_line.map_or(String::new(), |n| n.to_string());
        format!(
            "{OSC};{version};{type_char};{new_line};{old_field};{file}{ST}",
            version = self.version,
            file = self.file,
        )
    }
}
