use syntect::highlighting::{
    HighlightIterator, HighlightState, Highlighter, Style as SyntectStyle,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference};

use super::{LineSections, Painter};
use crate::ansi;
use crate::config::Config;
use crate::delta::State;
use crate::minusplus::{MinusPlus, MinusPlusIndex, Plus};

/// Syntax parsing state for each side of a hunk.
///
/// Minus and plus lines continue from different states: a string or comment opened by a removed
/// line doesn't affect the added lines that follow it. Context lines advance both sides.
pub struct SyntaxState<'p> {
    highlighter: Option<Highlighter<'p>>,
    sides: MinusPlus<SideState>,
    // Whether both sides are in the same state, so that context lines need only be parsed once.
    in_step: bool,
}

/// A point to return to, for painting several pairs of sides from the same starting point.
pub struct SavedSyntaxState {
    sides: MinusPlus<SideState>,
    in_step: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct SideState {
    parser: ParseState,
    highlight: Option<HighlightState>,
}

impl<'p> SyntaxState<'p> {
    /// Start parsing from the beginning of a hunk; without a highlighter, only for analysis.
    pub fn new(syntax: &SyntaxReference, highlighter: Option<Highlighter<'p>>) -> Self {
        let side = SideState {
            parser: ParseState::new(syntax),
            highlight: highlighter
                .as_ref()
                .map(|highlighter| HighlightState::new(highlighter, ScopeStack::new())),
        };
        Self {
            highlighter,
            sides: MinusPlus::new(side.clone(), side),
            in_step: true,
        }
    }

    pub fn save(&self) -> SavedSyntaxState {
        SavedSyntaxState {
            sides: self.sides.clone(),
            in_step: self.in_step,
        }
    }

    pub fn restore(&mut self, saved: &SavedSyntaxState) {
        self.sides.clone_from(&saved.sides);
        self.in_step = saved.in_step;
    }

    /// Syntax-highlight lines that aren't part of a diff, such as grep results or a hunk header's
    /// code fragment.
    pub fn highlight_lines<'a>(
        &mut self,
        lines: &'a [(String, State)],
        config: &Config,
    ) -> Vec<LineSections<'a, SyntectStyle>> {
        if self.highlighter.is_none() {
            return lines
                .iter()
                .map(|(line, _)| unstyled(line, config))
                .collect();
        }
        self.side_lines(Plus, lines, config)
    }

    /// Parse consecutive minus or plus lines.
    pub fn side_lines<'a>(
        &mut self,
        side: MinusPlusIndex,
        lines: &'a [(String, State)],
        config: &Config,
    ) -> Vec<LineSections<'a, SyntectStyle>> {
        if lines.is_empty() {
            return Vec::new();
        }
        self.in_step = false;
        let highlight = lines
            .iter()
            .any(|(_, state)| Painter::should_compute_syntax_highlighting(state, config));
        lines
            .iter()
            .map(|(line, _)| {
                self.sides[side].parse_line(line, self.highlighter.as_ref(), highlight, config)
            })
            .collect()
    }

    /// Parse a context line, which both sides share.
    pub fn context_line<'a>(
        &mut self,
        line: &'a str,
        state: &State,
        config: &Config,
    ) -> LineSections<'a, SyntectStyle> {
        let highlight = Painter::should_compute_syntax_highlighting(state, config);
        let highlighter = self.highlighter.as_ref();
        let sections = self
            .sides
            .plus
            .parse_line(line, highlighter, highlight, config);
        if self.in_step {
            self.sides.minus.clone_from(&self.sides.plus);
        } else {
            self.sides
                .minus
                .parse_line(line, highlighter, highlight, config);
            self.in_step = self.sides.minus == self.sides.plus;
        }
        sections
    }
}

impl SideState {
    fn parse_line<'a>(
        &mut self,
        line: &'a str,
        highlighter: Option<&Highlighter>,
        highlight: bool,
        config: &Config,
    ) -> LineSections<'a, SyntectStyle> {
        // Fast but simple length comparison. Overcounts non-printable ansi characters or wider
        // UTF-8, but `truncate_str_short` corrects that.
        let (with_syntax, plain) =
            if line.len() < config.max_syntax_length || config.max_syntax_length == 0 {
                (line, "")
            } else {
                // Re-split to get references into `line` with correct lifetimes.
                // SAFETY: slicing the string is safe because `truncate_str_short` always returns a
                // prefix of the input and only cuts at grapheme borders.
                //
                // Note: only parsing part of a line may result in wrong highlighting until the
                // state is reset in the next hunk. Also, as lines are no longer newline
                // terminated they might not be highlighted correctly.
                line.split_at(ansi::truncate_str_short(line, config.max_syntax_length).len())
            };
        let Ok(ops) = self.parser.parse_line(with_syntax, &config.syntax_set) else {
            return unstyled(line, config);
        };
        let mut sections = match (highlighter, &mut self.highlight) {
            // Highlight even when the result is discarded, to keep the state up to date.
            (Some(highlighter), Some(state)) => {
                HighlightIterator::new(state, &ops, with_syntax, highlighter).collect()
            }
            _ => Vec::new(),
        };
        if !highlight || sections.is_empty() {
            return unstyled(line, config);
        }
        if !plain.is_empty() {
            sections.push((config.null_syntect_style, plain));
        }
        sections
    }
}

fn unstyled<'a>(line: &'a str, config: &Config) -> LineSections<'a, SyntectStyle> {
    vec![(config.null_syntect_style, line)]
}
