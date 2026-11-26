//! Highlight wording changes in prose that was rewrapped onto different lines.
//!
//! Per-line pairing emphasizes every word that moves to another line when a paragraph is
//! rewrapped. Instead, align the words of each changed paragraph as a whole, and emphasize only
//! those that changed.

use std::ops::Range;

use unicode_width::UnicodeWidthStr;

use super::syntax_state::{ProseKind, ProseLine};
use super::LineSections;
use crate::align::{Alignment, Operation};
use crate::config::Config;
use crate::delta::State;
use crate::edits;
use crate::minusplus::{Minus, MinusPlus, Plus};
use crate::style::Style;

/// Consecutive prose lines of the same kind.
struct Paragraph<'a> {
    kind: ProseKind,
    // Index of the first line.
    start: usize,
    // Each line's prose, with its offset within the line.
    bodies: Vec<(&'a str, usize)>,
}

/// Override the per-line diff styles of rewrapped paragraphs.
pub fn highlight<'a>(
    lines: &MinusPlus<&'a Vec<(String, State)>>,
    prose: &MinusPlus<Vec<Option<ProseLine>>>,
    sections: &mut MinusPlus<Vec<LineSections<'a, Style>>>,
    homologs: &mut MinusPlus<Vec<bool>>,
    config: &Config,
) {
    let paragraphs = [Minus, Plus].map(|side| paragraphs_in(lines[side], &prose[side]));
    let [minus_paragraphs, plus_paragraphs] = &paragraphs;
    // Require the same sequence of paragraphs on each side before pairing them.
    if minus_paragraphs.len() != plus_paragraphs.len() {
        return;
    }
    for (minus, plus) in minus_paragraphs.iter().zip(plus_paragraphs) {
        // Single lines can't have moved words; skip building their alignment table.
        if minus.kind != plus.kind || (minus.bodies.len() == 1 && plus.bodies.len() == 1) {
            continue;
        }
        let Some(changes) = align_paragraphs(minus, plus, config) else {
            continue;
        };
        for (side, paragraph, emph_style) in [
            (Minus, minus, config.minus_emph_style),
            (Plus, plus, config.plus_emph_style),
        ] {
            for (i, ranges) in changes[side].iter().enumerate() {
                let index = paragraph.start + i;
                let (line, state) = &lines[side][index];
                sections[side][index] =
                    style_line(line, ranges, *config.get_style(state), emph_style);
                homologs[side][index] = true;
            }
        }
    }
}

fn paragraphs_in<'a>(
    lines: &'a [(String, State)],
    prose: &[Option<ProseLine>],
) -> Vec<Paragraph<'a>> {
    let mut paragraphs: Vec<Paragraph> = Vec::new();
    for (i, ((line, state), prose)) in lines.iter().zip(prose).enumerate() {
        // Lines with their own ANSI colors are painted as they are.
        if !matches!(state, State::HunkMinus(_, None) | State::HunkPlus(_, None)) {
            continue;
        }
        let Some(ProseLine {
            kind,
            body,
            starts_paragraph,
        }) = prose
        else {
            continue;
        };
        let body = (&line[body.clone()], body.start);
        match paragraphs.last_mut() {
            Some(previous)
                if !starts_paragraph
                    && previous.kind == *kind
                    && previous.start + previous.bodies.len() == i =>
            {
                previous.bodies.push(body);
            }
            _ => paragraphs.push(Paragraph {
                kind: *kind,
                start: i,
                bodies: vec![body],
            }),
        }
    }
    paragraphs
}

// A token's line index, byte range within that line, and display width.
type TokenPosition = (usize, Range<usize>, usize);
type ChangedRanges = MinusPlus<Vec<Vec<Range<usize>>>>;

fn paragraph_tokens<'a>(
    paragraph: &Paragraph<'a>,
    regex: &regex::Regex,
) -> (Vec<&'a str>, Vec<TokenPosition>) {
    // The alignment starts with a common empty token.
    let mut tokens = vec![""];
    let mut positions = vec![(0, 0..0, 0)];
    for (index, &(body, mut start)) in paragraph.bodies.iter().enumerate() {
        for token in edits::tokenize(body, regex) {
            let end = start + token.len();
            if !token.trim().is_empty() {
                tokens.push(token);
                // Weigh by display width, as `edits::annotate` does for per-line pairing.
                positions.push((index, start..end, token.width()));
            }
            start = end;
        }
    }
    (tokens, positions)
}

fn align_paragraphs(minus: &Paragraph, plus: &Paragraph, config: &Config) -> Option<ChangedRanges> {
    let (minus_tokens, minus_positions) = paragraph_tokens(minus, &config.tokenization_regex);
    let (plus_tokens, plus_positions) = paragraph_tokens(plus, &config.tokenization_regex);
    // Bound the quadratic alignment table for large replacement blocks.
    if minus_tokens.len().saturating_mul(plus_tokens.len()) > 1_000_000 {
        return None;
    }
    let mut positions = MinusPlus::new(minus_positions.into_iter(), plus_positions.into_iter());
    let mut changes = MinusPlus::new(
        vec![Vec::new(); minus.bodies.len()],
        vec![Vec::new(); plus.bodies.len()],
    );
    let mut moved = false;
    let mut matched = 0;
    let mut edited = 0;
    for operation in Alignment::new(minus_tokens, plus_tokens).operations() {
        let side = match operation {
            Operation::NoOp => {
                let (old_line, _, old_width) = positions.minus.next().unwrap();
                let (new_line, _, new_width) = positions.plus.next().unwrap();
                moved |= old_line != new_line;
                matched += old_width + new_width;
                continue;
            }
            Operation::Deletion => Minus,
            Operation::Insertion => Plus,
        };
        let (index, range, width) = positions[side].next().unwrap();
        edited += width;
        changes[side][index].push(range);
    }
    let distance = edits::compute_distance(edited as f64, (matched + edited) as f64);
    (moved && distance <= config.max_line_distance).then_some(changes)
}

fn style_line<'a>(
    line: &'a str,
    ranges: &[Range<usize>],
    base: Style,
    emph: Style,
) -> LineSections<'a, Style> {
    let mut offset = 0;
    let mut sections = Vec::new();
    for &Range { start, end } in ranges {
        if start > offset {
            sections.push((base, &line[offset..start]));
        }
        sections.push((emph, &line[start..end]));
        offset = end;
    }
    // Split off trailing whitespace so it can still be flagged as a whitespace error.
    let content_end =
        edits::get_contents_before_trailing_whitespace(line).map_or(line.len(), str::len);
    if offset < content_end {
        sections.push((base, &line[offset..content_end]));
    }
    if content_end < line.len() {
        sections.push((base, &line[content_end..]));
    }
    sections
}
