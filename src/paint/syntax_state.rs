use std::ops::Range;

use lazy_static::lazy_static;
use syntect::easy::ScopeRangeIterator;
use syntect::highlighting::{
    HighlightIterator, HighlightState, Highlighter, Style as SyntectStyle,
};
use syntect::parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxReference};

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
    // Tracked apart from `highlight`, which only exists with a syntax theme.
    scopes: ScopeStack,
    highlight: Option<HighlightState>,
}

/// What kind of prose a line holds, so that only lines of the same kind form a paragraph.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProseKind {
    /// A comment, identified by its innermost comment scope.
    Comment(Scope),
    /// Text in a document format such as Markdown.
    Text,
}

/// A line consisting of prose, apart from comment markers and surrounding whitespace.
pub struct ProseLine {
    pub kind: ProseKind,
    pub body: Range<usize>,
    // Whether the line starts a new paragraph, as a list item does.
    pub starts_paragraph: bool,
}

impl<'p> SyntaxState<'p> {
    /// Start parsing from the beginning of a hunk; without a highlighter, only for analysis.
    pub fn new(syntax: &SyntaxReference, highlighter: Option<Highlighter<'p>>) -> Self {
        let side = SideState {
            parser: ParseState::new(syntax),
            scopes: ScopeStack::new(),
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
        self.side_lines(Plus, lines, config).0
    }

    /// Parse consecutive minus or plus lines.
    pub fn side_lines<'a>(
        &mut self,
        side: MinusPlusIndex,
        lines: &'a [(String, State)],
        config: &Config,
    ) -> (Vec<LineSections<'a, SyntectStyle>>, Vec<Option<ProseLine>>) {
        let highlight = lines
            .iter()
            .any(|(_, state)| Painter::should_compute_syntax_highlighting(state, config));
        if lines.is_empty() || !self.should_parse(highlight, config) {
            return lines
                .iter()
                .map(|(line, _)| (unstyled(line, config), None))
                .unzip();
        }
        self.in_step = false;
        lines
            .iter()
            .map(|(line, _)| {
                self.sides[side].parse_line(line, self.highlighter.as_ref(), highlight, config)
            })
            .unzip()
    }

    // Skipping lines leaves the state behind, which only matters for analysis.
    fn should_parse(&self, highlight: bool, config: &Config) -> bool {
        config.prose_reflow || (highlight && self.highlighter.is_some())
    }

    /// Parse a context line, which both sides share.
    pub fn context_line<'a>(
        &mut self,
        line: &'a str,
        state: &State,
        config: &Config,
    ) -> LineSections<'a, SyntectStyle> {
        let highlight = Painter::should_compute_syntax_highlighting(state, config);
        if !self.should_parse(highlight, config) {
            return unstyled(line, config);
        }
        let highlighter = self.highlighter.as_ref();
        let (sections, _) = self
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
    ) -> (LineSections<'a, SyntectStyle>, Option<ProseLine>) {
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
            return (unstyled(line, config), None);
        };
        let prose = prose_line(&mut self.scopes, &ops, with_syntax).filter(|_| plain.is_empty());
        let mut sections = match (highlighter, &mut self.highlight) {
            // Highlight even when the result is discarded, to keep the state up to date.
            (Some(highlighter), Some(state)) => {
                HighlightIterator::new(state, &ops, with_syntax, highlighter).collect()
            }
            _ => Vec::new(),
        };
        if !highlight || sections.is_empty() {
            return (unstyled(line, config), prose);
        }
        if !plain.is_empty() {
            sections.push((config.null_syntect_style, plain));
        }
        (sections, prose)
    }
}

fn unstyled<'a>(line: &'a str, config: &Config) -> LineSections<'a, SyntectStyle> {
    vec![(config.null_syntect_style, line)]
}

lazy_static! {
    static ref COMMENT: Scope = Scope::new("comment").unwrap();
    static ref BLOCK_COMMENT: Scope = Scope::new("comment.block").unwrap();
    static ref TEXT: Scope = Scope::new("text").unwrap();
    static ref COMMENT_MARKER: Scope = Scope::new("punctuation.definition.comment").unwrap();
    static ref LIST_MARKER: Scope = Scope::new("punctuation.definition.list_item").unwrap();
    // Parts of text documents that aren't prose.
    static ref NOT_TEXT: Vec<Scope> = [
        "source",
        "meta.tag",
        "meta.code-fence",
        "markup.raw.block",
        "markup.raw.code-fence",
        "markup.heading",
    ]
    .iter()
    .map(|scope| Scope::new(scope).unwrap())
    .collect();
}

/// Apply a line's parse to `scopes`, and find whether the line is prose: either only comment, or
/// text in a document format such as Markdown.
fn prose_line(
    scopes: &mut ScopeStack,
    ops: &[(usize, ScopeStackOp)],
    line: &str,
) -> Option<ProseLine> {
    let mut is_prose = true;
    let mut kind = None;
    let mut body: Option<Range<usize>> = None;
    let mut starts_paragraph = false;
    for (range, op) in ScopeRangeIterator::new(ops, line) {
        // Keep applying the parse after the line is found not to be prose.
        if scopes.apply(op).is_err() {
            return None;
        }
        let text = &line[range.clone()];
        if !is_prose || text.trim().is_empty() {
            continue;
        }
        let Some((region_kind, marker)) = classify_region(scopes.as_slice()) else {
            is_prose = false;
            continue;
        };
        if *kind.get_or_insert(region_kind) != region_kind {
            is_prose = false;
        } else if !marker {
            let start = range.start + text.len() - text.trim_start().len();
            let end = range.start + text.trim_end().len();
            body = Some(body.map_or(start, |body| body.start)..end);
        } else if region_kind == ProseKind::Text {
            // A list item.
            starts_paragraph = true;
        }
    }
    let (kind, mut body) = (kind.filter(|_| is_prose)?, body?);
    // The leading `*` that decorates the lines of a block comment isn't always a marker scope.
    if matches!(kind, ProseKind::Comment(scope) if BLOCK_COMMENT.is_prefix_of(scope)) {
        if let Some(rest) = line[body.clone()].strip_prefix('*') {
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                body.start = body.end - rest.trim_start().len();
            }
        }
    }
    (!body.is_empty()).then_some(ProseLine {
        kind,
        body,
        starts_paragraph,
    })
}

/// The kind of prose a region of text with the given scopes is, if any, and whether it is a
/// marker that delimits the prose rather than being part of it.
fn classify_region(stack: &[Scope]) -> Option<(ProseKind, bool)> {
    let has = |scope: &Scope| stack.iter().any(|s| scope.is_prefix_of(*s));
    if let Some(&comment) = stack.iter().rev().find(|s| COMMENT.is_prefix_of(**s)) {
        Some((ProseKind::Comment(comment), has(&COMMENT_MARKER)))
    } else if stack.first().is_some_and(|root| TEXT.is_prefix_of(*root))
        && !NOT_TEXT.iter().any(has)
    {
        Some((ProseKind::Text, has(&LIST_MARKER)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::DiffType;
    use crate::tests::integration_test_utils::make_config_from_args;

    /// Each line's prose, and whether it continues the previous line's kind of prose.
    fn prose(extension: &str, text: &str) -> Vec<Option<(String, bool)>> {
        let config = make_config_from_args(&[]);
        let syntax = config
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap();
        let lines: Vec<_> = text
            .split_inclusive('\n')
            .map(|line| (line.to_string(), State::HunkPlus(DiffType::Unified, None)))
            .collect();
        let (_, prose) = SyntaxState::new(syntax, None).side_lines(Plus, &lines, &config);
        let mut previous_kind = None;
        lines
            .iter()
            .zip(prose)
            .map(|((line, _), prose)| {
                let kind = prose.as_ref().map(|prose| prose.kind);
                let continues = kind.is_some()
                    && kind == previous_kind
                    && !prose.as_ref().is_some_and(|prose| prose.starts_paragraph);
                previous_kind = kind;
                prose.map(|prose| (line[prose.body].to_string(), continues))
            })
            .collect()
    }

    fn some(body: &str, continues: bool) -> Option<(String, bool)> {
        Some((body.to_string(), continues))
    }

    #[test]
    fn test_comment_markers_are_not_prose() {
        assert_eq!(
            prose("c", "/* a b\n * c\n */\n// d\n"),
            [some("a b", false), some("c", true), None, some("d", false)]
        );
        assert_eq!(
            prose("rs", "/// a\n/// b\n// c\n/** d\n * e */\n"),
            [
                some("a", false),
                some("b", true),
                some("c", false),
                some("d", false),
                some("e", true)
            ]
        );
        assert_eq!(
            prose("hs", "{- a\n   b -}\n-- c\n"),
            [some("a", false), some("b", true), some("c", false)]
        );
    }

    #[test]
    fn test_code_is_not_prose() {
        assert_eq!(
            prose("py", "x = 1  # a\n# b\ns = \"\"\"c\nd\"\"\"\n"),
            [None, some("b", false), None, None]
        );
    }

    #[test]
    fn test_docstrings_are_prose() {
        assert_eq!(
            prose("py", "def f():\n    \"\"\"a\n    b\n    \"\"\"\n"),
            [None, some("a", false), some("b", true), None]
        );
    }

    #[test]
    fn test_markdown_text_is_prose() {
        assert_eq!(
            prose("md", "# a\nb *c*\nd `e`\n- f\n  g\n\n```\nh\n```\n"),
            [
                None,
                some("b *c*", false),
                some("d `e`", true),
                some("f", false),
                some("g", true),
                None,
                None,
                None,
                None
            ]
        );
    }
}
