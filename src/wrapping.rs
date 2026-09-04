use syntect::highlighting::Style as SyntectStyle;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::cli;
use crate::config::INLINE_SYMBOL_WIDTH_1;
use crate::fatal;

use crate::config::Config;
use crate::delta::DiffType;
use crate::delta::State;
use crate::features::line_numbers::{self, SideBySideLineWidth};
use crate::features::side_by_side::{available_line_width, line_is_too_long, Left, Right};
use crate::minusplus::*;
use crate::paint::LineSections;
use crate::style::Style;
use crate::utils::syntect::FromDeltaStyle;

/// See [`wrap_line`] for documentation.
#[derive(Clone, Debug)]
pub struct WrapConfig {
    pub left_symbol: String,
    pub right_symbol: String,
    pub right_prefix_symbol: String,
    // In fractions of 1000 so that a >100 wide panel can
    // still be configured down to a single character.
    pub use_wrap_right_permille: usize,
    // This value is --wrap-max-lines + 1, and unlimited is 0, see
    // adapt_wrap_max_lines_argument()
    pub max_lines: usize,
    // Maximum number of columns to scan backwards from the column-based split
    // looking for a word-friendly break point. 0 disables the lookback.
    pub word_lookback: usize,
    pub inline_hint_syntect_style: SyntectStyle,
}

impl WrapConfig {
    pub fn from_opt(opt: &cli::Opt, inline_hint_style: Style) -> Self {
        Self {
            left_symbol: ensure_display_width_1("wrap-left-symbol", opt.wrap_left_symbol.clone()),
            right_symbol: ensure_display_width_1(
                "wrap-right-symbol",
                opt.wrap_right_symbol.clone(),
            ),
            right_prefix_symbol: ensure_display_width_1(
                "wrap-right-prefix-symbol",
                opt.wrap_right_prefix_symbol.clone(),
            ),
            use_wrap_right_permille: {
                let arg = &opt.wrap_right_percent;
                let percent = remove_percent_suffix(arg)
                    .parse::<f64>()
                    .unwrap_or_else(|err| {
                        fatal(format!(
                            "Could not parse wrap-right-percent argument {}: {}.",
                            arg, err
                        ))
                    });
                if percent.is_finite() && percent > 0.0 && percent < 100.0 {
                    (percent * 10.0).round() as usize
                } else {
                    fatal("Invalid value for wrap-right-percent, not between 0 and 100.")
                }
            },
            max_lines: adapt_wrap_max_lines_argument(opt.wrap_max_lines.clone()),
            word_lookback: opt.wrap_word_lookback,
            inline_hint_syntect_style: SyntectStyle::from_delta_style(inline_hint_style),
        }
    }

    // Compute value of `max_line_length` field in the main `Config` struct.
    pub fn config_max_line_length(
        &self,
        max_line_length: usize,
        available_terminal_width: usize,
    ) -> usize {
        match self.max_lines {
            1 => max_line_length,
            // Ensure there is enough text to wrap, either don't truncate the input at all (0)
            // or ensure there is enough for the requested number of lines.
            // The input can contain ANSI sequences, so round up a bit. This is enough for
            // normal `git diff`, but might not be with ANSI heavy input.
            0 => 0,
            wrap_max_lines => {
                let single_pane_width = available_terminal_width / 2;
                let add_25_percent_or_term_width =
                    |x| x + std::cmp::max((x * 250) / 1000, single_pane_width);
                std::cmp::max(
                    max_line_length,
                    add_25_percent_or_term_width(single_pane_width * wrap_max_lines),
                )
            }
        }
    }
}

fn remove_percent_suffix(arg: &str) -> &str {
    match &arg.strip_suffix('%') {
        Some(s) => s,
        None => arg,
    }
}

fn ensure_display_width_1(what: &str, arg: String) -> String {
    match arg.grapheme_indices(true).count() {
        INLINE_SYMBOL_WIDTH_1 => arg,
        width => fatal(format!(
            "Invalid value for {what}, display width of \"{arg}\" must be {INLINE_SYMBOL_WIDTH_1} but is {width}",
        )),
    }
}

fn is_word_grapheme(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}

fn is_whitespace_grapheme(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_whitespace())
}

fn adapt_wrap_max_lines_argument(arg: String) -> usize {
    if arg == "∞" || arg == "unlimited" || arg.starts_with("inf") {
        0
    } else {
        arg.parse::<usize>()
            .unwrap_or_else(|err| fatal(format!("Invalid wrap-max-lines argument: {err}")))
            + 1
    }
}

#[derive(PartialEq)]
enum Stop {
    StackEmpty,
    LineLimit,
}

/// How many wrapped rows to allow. If the panel is so narrow that only the wrap
/// symbol itself would fit, wrapping is impossible, so force a single (truncated)
/// row. Both `find_cuts` and `wrap_line`'s loop use this, so they must agree.
fn effective_max_lines(line_width: usize, max_lines: usize) -> usize {
    if line_width <= INLINE_SYMBOL_WIDTH_1 {
        1
    } else {
        max_lines
    }
}

/// Grapheme indices at which to cut `line` into wrapped rows.
/// Assumes a style boundary never splits a grapheme cluster.
fn find_cuts(config: &Config, line: &str, line_width: usize) -> Vec<usize> {
    let wrap_symbol_width = config.wrap_config.left_symbol.width();
    let word_lookback = config.wrap_config.word_lookback;
    let max_lines = effective_max_lines(line_width, config.wrap_config.max_lines);

    if line_width < wrap_symbol_width {
        return Vec::new();
    }
    let word_lookback_start = (line_width - wrap_symbol_width).saturating_sub(word_lookback) + 1;

    let mut graphemes = line.graphemes(true);
    let mut best_break_iter = graphemes.clone();

    let mut cuts: Vec<usize> = Vec::new();
    let (mut current_row_width, mut current_row_start) = (0usize, 0usize);
    let mut best_break = (0usize, 0usize); // (rank, absolute grapheme index)
    let mut current_pos = 0usize;

    loop {
        // Deal with the remainder first: on overflow, cut at the best break.
        // Re-read the next row from the cut.
        if current_row_width > line_width {
            let (_, cut) = best_break;
            if max_lines > 0 && cuts.len() + 1 >= max_lines {
                break;
            }
            // Best cut is the row start — a leading grapheme too wide to fit.
            // Unbounded, so stop and leave it as the overflowing last row.
            if cut == current_row_start && max_lines == 0 {
                break;
            }
            cuts.push(cut);
            graphemes = best_break_iter.clone();
            current_pos = cut;
            current_row_width = 0;
            current_row_start = cut;
            best_break = (0, cut);
            continue;
        }

        // The iterator we rewind to if the cut lands here.
        let mut break_iter = graphemes.clone();
        let g = match graphemes.next() {
            Some(g) => g,
            None => break,
        };
        // Prefer a whitespace break over a non-word one over a word one.
        let break_rank = if is_whitespace_grapheme(g) {
            2
        } else if !is_word_grapheme(g) {
            1
        } else {
            0
        };
        current_row_width += g.width();
        // Break after `g` if room remains for the wrap symbol, else before it.
        let (pos, cut_width) = if current_row_width <= line_width - wrap_symbol_width {
            break_iter.next();
            (current_pos + 1, current_row_width)
        } else {
            (current_pos, current_row_width - g.width())
        };
        if cut_width <= line_width - wrap_symbol_width {
            let rank = if word_lookback > 0 && cut_width >= word_lookback_start {
                break_rank
            } else {
                0
            };
            if rank >= best_break.0 {
                best_break = (rank, pos);
                best_break_iter = break_iter;
            }
        }
        current_pos += 1;
    }
    cuts
}

/// Wrap the given `line` if it is longer than `line_width`. Wrap to at most
/// [Config::WrapConfig::max_lines](WrapConfig::max_lines) lines,
/// then truncate again - but never truncate if it is `0`. Place
/// [left_symbol](WrapConfig::left_symbol) at the end of wrapped lines.
/// If wrapping results in only *one* extra line and if the width of the wrapped
/// line is less than [use_wrap_right_permille](WrapConfig::use_wrap_right_permille)
/// then right-align the second line and use the symbols
/// [right_symbol](WrapConfig::right_symbol) and
/// on the next line [right_prefix_symbol](WrapConfig::right_prefix_symbol).
/// The inserted characters will follow the
/// [inline_hint_syntect_style](WrapConfig::inline_hint_syntect_style).
pub fn wrap_line<'a, I, S>(
    config: &'a Config,
    line: I,
    line_width: usize,
    fill_style: &S,
    inline_hint_style: &Option<S>,
    cuts: &[usize],
) -> Vec<LineSections<'a, S>>
where
    I: IntoIterator<Item = (S, &'a str)> + std::fmt::Debug,
    <I as IntoIterator>::IntoIter: DoubleEndedIterator,
    S: Copy + Default + std::fmt::Debug,
{
    let mut result = Vec::new();

    let wrap_config = &config.wrap_config;

    // The current line being assembled from the input to fit exactly into the given width.
    // A somewhat leaky abstraction as the fields are also accessed directly.
    struct CurrLine<'a, S: Default> {
        line_segments: LineSections<'a, S>,
        len: usize,
    }
    impl<'a, S: Default> CurrLine<'a, S> {
        fn reset() -> Self {
            CurrLine {
                line_segments: Vec::new(),
                len: 0,
            }
        }
        fn push_and_set_len(&mut self, text: (S, &'a str), len: usize) {
            self.line_segments.push(text);
            self.len = len;
        }
        fn has_text(&self) -> bool {
            self.len > 0
        }
        fn text_len(&self) -> usize {
            self.len
        }
    }

    let mut curr_line = CurrLine::reset();

    // Determine the background (diff) and color (syntax) of an inserted symbol.
    let symbol_style = match inline_hint_style {
        Some(style) => *style,
        None => *fill_style,
    };

    let mut stack = line.into_iter().rev().collect::<Vec<_>>();

    let max_lines = effective_max_lines(line_width, wrap_config.max_lines);

    let line_limit_reached = |result: &Vec<_>| max_lines > 0 && result.len() + 1 >= max_lines;

    // `cuts` (absolute grapheme indices, shared by both streams) are peeked
    // before consuming each chunk, so a chunk straddling a cut is split there.
    let mut cuts_iter = cuts.iter().peekable();
    let mut outer_pos: usize = 0;

    let stop = loop {
        if stack.is_empty() {
            break Stop::StackEmpty;
        } else if line_limit_reached(&result) {
            break Stop::LineLimit;
        }

        let (style, text, graphemes) = stack
            .pop()
            .map(|(style, text)| {
                (
                    style,
                    text,
                    text.graphemes(true)
                        .map(|item| (item.len(), item.width()))
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap();

        let graphemes_width: usize = graphemes.iter().map(|(_, w)| w).sum();
        let new_len = curr_line.len + graphemes_width;
        let outer_chunk_end = outer_pos + graphemes.len();

        let must_split = matches!(cuts_iter.peek(), Some(&&outer_cut) if outer_cut <= outer_chunk_end);

        if !must_split {
            curr_line.push_and_set_len((style, text), new_len);
            outer_pos = outer_chunk_end;
        } else {
            let outer_cut = *cuts_iter.next().unwrap();
            let inner_cut = outer_cut - outer_pos;
            let byte_split_pos: usize = graphemes[..inner_cut].iter().map(|(l, _)| *l).sum();
            let mut line_segments = curr_line.line_segments;
            if byte_split_pos > 0 {
                line_segments.push((style, &text[..byte_split_pos]));
            }
            if byte_split_pos < text.len() {
                stack.push((style, &text[byte_split_pos..]));
            }
            line_segments.push((symbol_style, &wrap_config.left_symbol));
            result.push(line_segments);
            curr_line = CurrLine::reset();
            outer_pos = outer_cut;
        }
    };

    // Right-align wrapped line:
    // Done if wrapping adds exactly one line and this line is less than the given
    // permille wide. Also change the wrap symbol at the end of the previous (first) line.
    if result.len() == 1 && curr_line.has_text() {
        let current_permille = (curr_line.text_len() * 1000) / line_width;

        // pad line will add a wrap_config.right_prefix_symbol
        let pad_len = line_width
            .saturating_sub(curr_line.text_len() + wrap_config.right_prefix_symbol.width());

        if wrap_config.use_wrap_right_permille > current_permille && pad_len > 0 {
            // The inserted spaces, which align a line to the right, point into this string.
            const SPACES: &str = "                                                                ";

            match result.last_mut() {
                Some(ref mut vec) if !vec.is_empty() => {
                    vec.last_mut().unwrap().1 = &wrap_config.right_symbol
                }
                _ => unreachable!("wrap result must not be empty"),
            }

            let mut right_aligned_line = Vec::new();

            for _ in 0..(pad_len / SPACES.len()) {
                right_aligned_line.push((*fill_style, SPACES));
            }

            match pad_len % SPACES.len() {
                0 => (),
                n => right_aligned_line.push((*fill_style, &SPACES[0..n])),
            }

            right_aligned_line.push((symbol_style, &wrap_config.right_prefix_symbol));

            right_aligned_line.extend(curr_line.line_segments);

            curr_line.line_segments = right_aligned_line;

            // curr_line.len not updated, as only 0 / >0 for `has_text()` is required.
        }
    }

    if curr_line.has_text() {
        result.push(curr_line.line_segments);
    }

    if stop == Stop::LineLimit && result.len() != max_lines {
        result.push(Vec::new());
    }

    // Anything that is left will be added to the (last) line. If this is too long it will
    // be truncated later.
    if !stack.is_empty() {
        if result.is_empty() {
            result.push(Vec::new());
        }

        // unwrap: previous `if` ensures result can not be empty
        result.last_mut().unwrap().extend(stack.into_iter().rev());
    }

    result
}

#[allow(clippy::too_many_arguments)]
fn wrap_if_too_long<'a, S>(
    config: &'a Config,
    wrapped: &mut Vec<LineSections<'a, S>>,
    input_vec: LineSections<'a, S>,
    must_wrap: bool,
    line_width: usize,
    fill_style: &S,
    inline_hint_style: &Option<S>,
    cuts: &[usize],
) -> (usize, usize)
where
    S: Copy + Default + std::fmt::Debug,
{
    let size_prev = wrapped.len();

    if must_wrap {
        wrapped.append(&mut wrap_line(
            config,
            input_vec,
            line_width,
            fill_style,
            inline_hint_style,
            cuts,
        ));
    } else {
        wrapped.push(input_vec.to_vec());
    }

    (size_prev, wrapped.len())
}

/// Call [`wrap_line`] for the `syntax` and the `diff` lines if `wrapinfo` says
/// a specific line was longer than `line_width`. Return an adjusted `alignment`
/// with regard to the added wrapped lines.
#[allow(clippy::comparison_chain, clippy::type_complexity)]
pub fn wrap_minusplus_block<'c: 'a, 'a>(
    config: &'c Config,
    lines: MinusPlus<&Vec<(String, State)>>,
    syntax: MinusPlus<Vec<LineSections<'a, SyntectStyle>>>,
    diff: MinusPlus<Vec<LineSections<'a, Style>>>,
    alignment: &[(Option<usize>, Option<usize>)],
    line_width: &SideBySideLineWidth,
    wrapinfo: &'a MinusPlus<Vec<bool>>,
) -> (
    Vec<(Option<usize>, Option<usize>)>,
    MinusPlus<Vec<State>>,
    MinusPlus<Vec<LineSections<'a, SyntectStyle>>>,
    MinusPlus<Vec<LineSections<'a, Style>>>,
) {
    let mut new_alignment = Vec::new();
    let mut new_states = MinusPlus::<Vec<State>>::default();
    let mut new_wrapped_syntax = MinusPlus::default();
    let mut new_wrapped_diff = MinusPlus::default();

    // Turn all these into pairs of iterators so they can be advanced according
    // to the alignment and independently.
    let mut syntax = MinusPlus::new(syntax.minus.into_iter(), syntax.plus.into_iter());
    let mut diff = MinusPlus::new(diff.minus.into_iter(), diff.plus.into_iter());
    let mut wrapinfo = MinusPlus::new(wrapinfo[Left].iter(), wrapinfo[Right].iter());

    let fill_style = MinusPlus::new(&config.minus_style, &config.plus_style);

    // Internal helper function to perform wrapping for both the syntax and the
    // diff highlighting (SyntectStyle and Style).
    #[allow(clippy::too_many_arguments)]
    pub fn wrap_syntax_and_diff<'a, ItSyn, ItDiff, ItWrap>(
        config: &'a Config,
        wrapped_syntax: &mut Vec<LineSections<'a, SyntectStyle>>,
        wrapped_diff: &mut Vec<LineSections<'a, Style>>,
        syntax_iter: &mut ItSyn,
        diff_iter: &mut ItDiff,
        wrapinfo_iter: &mut ItWrap,
        line: &str,
        line_width: usize,
        fill_style: &Style,
        errhint: &'a str,
    ) -> (usize, usize)
    where
        ItSyn: Iterator<Item = LineSections<'a, SyntectStyle>>,
        ItDiff: Iterator<Item = LineSections<'a, Style>>,
        ItWrap: Iterator<Item = &'a bool>,
    {
        let must_wrap = *wrapinfo_iter
            .next()
            .unwrap_or_else(|| panic!("bad wrap info {}", errhint));

        let cuts = if must_wrap {
            find_cuts(config, line, line_width)
        } else {
            Vec::new()
        };

        let (start, extended_to) = wrap_if_too_long(
            config,
            wrapped_syntax,
            syntax_iter
                .next()
                .unwrap_or_else(|| panic!("bad syntax alignment {}", errhint)),
            must_wrap,
            line_width,
            &config.null_syntect_style,
            &Some(config.wrap_config.inline_hint_syntect_style),
            &cuts,
        );

        // TODO: Why is the background color set to white when
        // ansi_term_style.background is None?
        let inline_hint_style = if config
            .inline_hint_style
            .ansi_term_style
            .background
            .is_some()
        {
            Some(config.inline_hint_style)
        } else {
            None
        };

        let (start2, extended_to2) = wrap_if_too_long(
            config,
            wrapped_diff,
            diff_iter
                .next()
                .unwrap_or_else(|| panic!("bad diff alignment {}", errhint)),
            must_wrap,
            line_width,
            fill_style,
            &inline_hint_style,
            &cuts,
        );

        // Both wraps consume the same `cuts`, so they add the same number of
        // rows by construction; this just guards that invariant.
        assert_eq!(
            (start, extended_to),
            (start2, extended_to2),
            "syntax and diff wrapping differs {errhint}",
        );

        (start, extended_to)
    }

    // This macro avoids having the same code block 4x in the alignment processing
    macro_rules! wrap_and_assert {
        ($side:tt, $errhint:tt, $have:tt, $expected:tt) => {{
            assert_eq!(*$have, $expected, "bad alignment index {}", $errhint);
            $expected += 1;

            wrap_syntax_and_diff(
                &config,
                &mut new_wrapped_syntax[$side],
                &mut new_wrapped_diff[$side],
                &mut syntax[$side],
                &mut diff[$side],
                &mut wrapinfo[$side],
                lines[$side][*$have].0.as_str(),
                line_width[$side],
                &fill_style[$side],
                $errhint,
            )
        }};
    }

    let mut m_expected = 0;
    let mut p_expected = 0;

    // Process blocks according to the alignment and build a new alignment.
    // If lines get added via wrapping these are assigned the state HunkMinusWrapped/HunkPlusWrapped.
    for (minus, plus) in alignment {
        let (minus_extended, plus_extended) = match (minus, plus) {
            (Some(m), None) => {
                let (minus_start, extended_to) = wrap_and_assert!(Left, "[*l*] (-)", m, m_expected);

                for i in minus_start..extended_to {
                    new_alignment.push((Some(i), None));
                }

                (extended_to - minus_start, 0)
            }
            (None, Some(p)) => {
                let (plus_start, extended_to) = wrap_and_assert!(Right, "(-) [*r*]", p, p_expected);

                for i in plus_start..extended_to {
                    new_alignment.push((None, Some(i)));
                }

                (0, extended_to - plus_start)
            }
            (Some(m), Some(p)) => {
                let (minus_start, m_extended_to) =
                    wrap_and_assert!(Left, "[*l*] (r)", m, m_expected);
                let (plus_start, p_extended_to) =
                    wrap_and_assert!(Right, "(l) [*r*]", p, p_expected);

                for (new_m, new_p) in (minus_start..m_extended_to).zip(plus_start..p_extended_to) {
                    new_alignment.push((Some(new_m), Some(new_p)));
                }

                // This Some(m):Some(p) alignment might have become uneven, so fill
                // up the shorter side with None.

                let minus_extended = m_extended_to - minus_start;
                let plus_extended = p_extended_to - plus_start;

                let plus_minus = (minus_extended as isize) - (plus_extended as isize);

                if plus_minus > 0 {
                    for m in (m_extended_to as isize - plus_minus) as usize..m_extended_to {
                        new_alignment.push((Some(m), None));
                    }
                } else if plus_minus < 0 {
                    for p in (p_extended_to as isize + plus_minus) as usize..p_extended_to {
                        new_alignment.push((None, Some(p)));
                    }
                }

                (minus_extended, plus_extended)
            }
            _ => unreachable!("None-None alignment"),
        };

        if minus_extended > 0 {
            new_states[Left].push(State::HunkMinus(DiffType::Unified, None));
            for _ in 1..minus_extended {
                new_states[Left].push(State::HunkMinusWrapped);
            }
        }
        if plus_extended > 0 {
            new_states[Right].push(State::HunkPlus(DiffType::Unified, None));
            for _ in 1..plus_extended {
                new_states[Right].push(State::HunkPlusWrapped);
            }
        }
    }

    (
        new_alignment,
        new_states,
        new_wrapped_syntax,
        new_wrapped_diff,
    )
}

#[allow(clippy::comparison_chain, clippy::type_complexity)]
pub fn wrap_zero_block<'c: 'a, 'a>(
    config: &'c Config,
    line: &str,
    mut states: Vec<State>,
    syntax_style_sections: Vec<LineSections<'a, SyntectStyle>>,
    diff_style_sections: Vec<LineSections<'a, Style>>,
    line_numbers_data: &Option<&mut line_numbers::LineNumbersData>,
) -> (
    Vec<State>,
    Vec<LineSections<'a, SyntectStyle>>,
    Vec<LineSections<'a, Style>>,
) {
    // The width is the minimum of the left/right side. The panels should be equally sized,
    // but in rare cases the remaining panel width might differ due to the space the line
    // numbers take up.
    let line_width = if let Some(line_numbers_data) = line_numbers_data {
        let width = available_line_width(config, line_numbers_data);
        std::cmp::min(width[Left], width[Right])
    } else {
        std::cmp::min(
            config.side_by_side_data[Left].width,
            config.side_by_side_data[Right].width,
        )
    };

    // Called with a single line, so no need to use the 1-sized bool vector.
    // If that changes the wrapping logic should be updated as well.
    debug_assert_eq!(diff_style_sections.len(), 1);

    let should_wrap = line_is_too_long(line, line_width);

    if should_wrap {
        let cuts = find_cuts(config, line, line_width);
        let syntax_style = wrap_line(
            config,
            syntax_style_sections.into_iter().flatten(),
            line_width,
            &SyntectStyle::default(),
            &Some(config.wrap_config.inline_hint_syntect_style),
            &cuts,
        );

        // TODO: Why is the background color set to white when
        // ansi_term_style.background is None?
        let inline_hint_style = if config
            .inline_hint_style
            .ansi_term_style
            .background
            .is_some()
        {
            Some(config.inline_hint_style)
        } else {
            None
        };
        let diff_style = wrap_line(
            config,
            diff_style_sections.into_iter().flatten(),
            line_width,
            // To actually highlight inline hint characters:
            &Style {
                is_syntax_highlighted: true,
                ..config.null_style
            },
            &inline_hint_style,
            &cuts,
        );

        states.resize_with(syntax_style.len(), || State::HunkZeroWrapped);

        (states, syntax_style, diff_style)
    } else {
        (states, syntax_style_sections, diff_style_sections)
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;
    use syntect::highlighting::Style as SyntectStyle;

    use super::find_cuts;
    use super::wrap_line;
    use super::WrapConfig;
    use crate::config::Config;
    use crate::paint::LineSections;
    use crate::style::Style;
    use crate::tests::integration_test_utils::{make_config_from_args, DeltaTest};

    lazy_static! {
        static ref S1: Style = Style {
            is_syntax_highlighted: true,
            ..Default::default()
        };
    }
    lazy_static! {
        static ref S2: Style = Style {
            is_emph: true,
            ..Default::default()
        };
    }
    lazy_static! {
        static ref SY: SyntectStyle = SyntectStyle::default();
    }
    lazy_static! {
        static ref SD: Style = Style::default();
    }

    const W: &str = "+"; // wrap
    const WR: &str = "<"; // wrap-right
    const RA: &str = ">"; // right-align

    lazy_static! {
        static ref WRAP_DEFAULT_ARGS: Vec<&'static str> = vec![
            "--wrap-left-symbol",
            W,
            "--wrap-right-symbol",
            WR,
            "--wrap-right-prefix-symbol",
            RA,
            "--wrap-max-lines",
            "4",
            "--wrap-right-percent",
            "37.0%",
        ];
    }

    lazy_static! {
        static ref TEST_WRAP_CFG: WrapConfig =
            make_config_from_args(&WRAP_DEFAULT_ARGS).wrap_config;
    }

    fn default_wrap_cfg_plus<'a>(args: &[&'a str]) -> Vec<&'a str> {
        let mut result = WRAP_DEFAULT_ARGS.clone();
        result.extend_from_slice(args);
        result
    }

    fn mk_wrap_cfg(wrap_cfg: &WrapConfig) -> Config {
        let mut cfg: Config = make_config_from_args(&[]);
        cfg.wrap_config = wrap_cfg.clone();
        cfg
    }

    fn wrap_test<'a, I, S>(cfg: &'a Config, line: I, line_width: usize) -> Vec<LineSections<'a, S>>
    where
        I: IntoIterator<Item = (S, &'a str)> + std::fmt::Debug,
        <I as IntoIterator>::IntoIter: DoubleEndedIterator,
        S: Copy + Default + std::fmt::Debug,
    {
        let line: Vec<(S, &'a str)> = line.into_iter().collect();
        let line_text: String = line.iter().map(|(_, t)| *t).collect();
        let cuts = find_cuts(cfg, &line_text, line_width);
        wrap_line(cfg, line, line_width, &S::default(), &None, &cuts)
    }

    #[test]
    fn test_wrap_line_single() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        {
            let line = vec![(*SY, "0")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*SY, "0")]]);
        }
        {
            let line = vec![(*S1, "012"), (*S2, "34")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "012"), (*S2, "34")]]);
        }
        {
            let line = vec![(*S1, "012"), (*S2, "345")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "012"), (*S2, "345")]]);
        }
        {
            // Empty input usually does not happen
            let line = vec![(*S1, "")];
            let lines = wrap_test(&cfg, line, 6);
            assert!(lines.is_empty());
        }
        {
            // Partially empty should not happen either
            let line = vec![(*S1, ""), (*S2, "0")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, ""), (*S2, "0")]]);
        }
        {
            let line = vec![(*S1, "0"), (*S2, "")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "0"), (*S2, "")]]);
        }
        {
            let line = vec![
                (*S1, "0"),
                (*S2, ""),
                (*S1, ""),
                (*S2, ""),
                (*S1, ""),
                (*S2, ""),
                (*S1, ""),
                (*S2, ""),
                (*S1, ""),
                (*S2, ""),
            ];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(
                lines,
                vec![vec![
                    (*S1, "0"),
                    (*S2, ""),
                    (*S1, ""),
                    (*S2, ""),
                    (*S1, ""),
                    (*S2, ""),
                    (*S1, ""),
                    (*S2, ""),
                    (*S1, ""),
                    (*S2, "")
                ]]
            );
        }
    }

    #[test]
    fn test_wrap_line_align_right_1() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        let line = vec![(*S1, "0123456789ab")];
        let lines = wrap_test(&cfg, line, 11);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].last().unwrap().1, WR);
        assert_eq!(lines[1], [(*SD, "        "), (*SD, ">"), (*S1, "ab")]);
    }

    #[test]
    fn test_wrap_line_align_right_2() {
        let line = vec![(*S1, "012"), (*S2, "3456")];

        {
            // Right align lines on the second line
            let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

            let lines = wrap_test(&cfg, line.clone(), 6);
            assert_eq!(
                lines,
                vec![
                    vec![(*S1, "012"), (*S2, "34"), (*SD, WR)],
                    vec![(*SD, "   "), (*SD, RA), (*S2, "56")]
                ]
            );
        }

        {
            // Set right align percentage lower, normal wrapping
            let mut no_align_right = TEST_WRAP_CFG.clone();
            no_align_right.use_wrap_right_permille = 1; // 0.1%
            let cfg_no_align_right = mk_wrap_cfg(&no_align_right);

            let lines = wrap_test(&cfg_no_align_right, line, 6);
            assert_eq!(
                lines,
                vec![vec![(*S1, "012"), (*S2, "34"), (*SD, W)], vec![(*S2, "56")]]
            );
        }
    }

    #[test]
    fn test_wrap_line_word_lookback() {
        // At width 17 the column-based split lands inside "important_file_xy"
        // (mid-word). With word_lookback=10 the split should back up to right
        // after the "/" that follows "very".
        let line = vec![(*S1, "src/very/important_file_xy")];

        {
            // word_lookback=0 (disabled): split exactly at column 16.
            let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);
            let lines = wrap_test(&cfg, line.clone(), 17);
            assert_eq!(lines[0], [(*S1, "src/very/importa"), (*SD, W)]);
        }

        {
            let mut wc = TEST_WRAP_CFG.clone();
            wc.word_lookback = 10;
            let cfg = mk_wrap_cfg(&wc);
            let lines = wrap_test(&cfg, line, 17);
            // Lookback finds the "/" after "very" and splits there.
            assert_eq!(lines[0], [(*S1, "src/very/"), (*SD, W)]);
        }
    }

    #[test]
    fn test_wrap_line_word_lookback_prefers_whitespace() {
        // Window contains both a non-word grapheme ("/") closer to the split
        // and a whitespace (" ") farther back. The space should win.
        let line = vec![(*S1, "alpha beta/gammadelta")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 10;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 16);
        // Column-based split sits inside "gammadelta". The "/" at index 10 is
        // closer, but the space at index 5 is preferred when within the window.
        assert_eq!(lines[0], [(*S1, "alpha "), (*SD, W)]);
    }

    #[test]
    fn test_wrap_line_word_lookback_no_break_in_window() {
        // No non-word grapheme within lookback window — fall back to column split.
        let line = vec![(*S1, "abcdefghijklmnopqrstuvwxyz")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 5;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 11);
        assert_eq!(lines[0], [(*S1, "abcdefghij"), (*SD, W)]);
    }

    #[test]
    fn test_wrap_line_word_lookback_break_before_space() {
        // The cut sits *before* the space after "cdef" (whose own column is just
        // past the limit), filling row 1 — rather than backing up to the earlier
        // space, which would waste the row and wrap worse than no lookback at all.
        let line = vec![(*S1, "ab cdef ghij")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 8;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 8);
        assert_eq!(
            lines,
            vec![vec![(*S1, "ab cdef"), (*SD, W)], vec![(*S1, " ghij")]]
        );
    }

    #[test]
    fn test_wrap_line_word_lookback_break_before_nonword() {
        // A non-word (rank 1) before-break: "/" overflows the row, so the cut
        // lands before it and "/" leads the next row.
        let line = vec![(*S1, "abcdefg/hij")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 8;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 8);
        assert_eq!(
            lines,
            vec![vec![(*S1, "abcdefg"), (*SD, W)], vec![(*S1, "/hij")]]
        );
    }

    #[test]
    fn test_wrap_line_segmentation_independent() {
        // Cuts come from the line text, not the style-segment boundaries.
        // So the syntax and diff streams wrap the same, however they split it.
        fn row_texts(rows: &[LineSections<'_, Style>]) -> Vec<String> {
            rows.iter()
                .map(|row| row.iter().map(|&(_, t)| t).collect::<String>())
                .collect()
        }
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);
        let width = 6;
        // Segment boundaries (at 3 and 7) deliberately straddle the wrap cut at 5.
        let whole = vec![(*S1, "abcdefghij")];
        let split = vec![(*S1, "abc"), (*S2, "defg"), (*S1, "hij")];
        assert_eq!(
            row_texts(&wrap_test(&cfg, whole, width)),
            row_texts(&wrap_test(&cfg, split, width)),
            "wrapping must not depend on how the line is split into style segments",
        );
    }

    #[test]
    fn test_wrap_line_cluster_survives_segmentation() {
        // A width-1 grapheme cluster of 7 code points: base 'e' + 6 combining marks.
        const G: &str = "e\u{0301}\u{0300}\u{0302}\u{0303}\u{0308}\u{0304}";
        assert_eq!(G.chars().count(), 7, "G should be a 7-code-point cluster");
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);
        // "aGbcd" at width 3 cuts after the cluster, so "aG" (two columns) fills row
        // 1 — proving G counts as one column and is never split — and it lands there
        // identically however the surrounding segment boundaries fall.
        let seg_a = vec![(*S1, "a"), (*S2, G), (*S1, "bcd")];
        let seg_b = vec![(*S1, "a"), (*S2, G), (*S2, "b"), (*S1, "cd")];
        assert_eq!(
            wrap_test(&cfg, seg_a, 3),
            vec![vec![(*S1, "a"), (*S2, G), (*SD, W)], vec![(*S1, "bcd")]]
        );
        assert_eq!(
            wrap_test(&cfg, seg_b, 3),
            vec![vec![(*S1, "a"), (*S2, G), (*SD, W)], vec![(*S2, "b"), (*S1, "cd")]]
        );
    }

    #[test]
    fn test_wrap_line_word_lookback_max_lines() {
        // Word-lookback and --wrap-max-lines compose: this would wrap to 3
        // word-broken rows, but max_lines = 2 truncates, leaving the remainder
        // as the (overflowing) last row.
        let line = vec![(*S1, "ab cdef ghij klmn")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 8;
        wc.max_lines = 2;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 8);
        assert_eq!(
            lines,
            vec![
                vec![(*S1, "ab cdef"), (*SD, W)],
                vec![(*S1, " ghij klmn")],
            ]
        );
    }

    #[test]
    fn test_wrap_line_word_lookback_wide_glyph_window() {
        // The lookback window is measured in columns, not graphemes. At width 9 the
        // symbol-leaving width is 8 and the floor is 8 - 4 = 4 columns; the "/" sits
        // exactly at column 4, just outside the window, while the two double-width
        // glyphs after it fill the window with word graphemes. So the split falls
        // back to the column boundary, not the "/". (A grapheme-count window would
        // have wrongly pulled the "/" in.)
        let line = vec![(*S1, "abc/一二三四五")];
        let mut wc = TEST_WRAP_CFG.clone();
        wc.word_lookback = 4;
        let cfg = mk_wrap_cfg(&wc);
        let lines = wrap_test(&cfg, line, 9);
        assert_eq!(
            lines,
            vec![vec![(*S1, "abc/一二"), (*SD, W)], vec![(*S1, "三四五")]]
        );
    }

    #[test]
    fn test_wrap_line_newlines() {
        fn mk_input(len: usize) -> LineSections<'static, Style> {
            const IN: &str = "0123456789abcdefZ";
            let v = &[*S1, *S2];
            let s1s2 = v.iter().cycle();
            let text: Vec<_> = IN.matches(|_| true).take(len + 1).collect();
            s1s2.zip(text.iter())
                .map(|(style, text)| (*style, *text))
                .collect()
        }
        fn mk_input_nl(len: usize) -> LineSections<'static, Style> {
            const NL: &str = "\n";
            let mut line = mk_input(len);
            line.push((*S2, NL));
            line
        }
        fn mk_expected<'a>(
            vec: &LineSections<'a, Style>,
            from: usize,
            to: usize,
            append: Option<(Style, &'a str)>,
        ) -> LineSections<'a, Style> {
            let mut result: Vec<_> = vec[from..to].to_vec();
            if let Some(val) = append {
                result.push(val);
            }
            result
        }

        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        {
            let line = vec![(*S1, "012"), (*S2, "345\n")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "012"), (*S2, "345\n")]]);
        }

        {
            for i in 0..=5 {
                let line = mk_input(i);
                let lines = wrap_test(&cfg, line, 6);
                assert_eq!(lines, vec![mk_input(i)]);

                let line = mk_input_nl(i);
                let lines = wrap_test(&cfg, line, 6);
                assert_eq!(lines, vec![mk_input_nl(i)]);
            }
        }

        {
            let line = mk_input_nl(9);
            let lines = wrap_test(&cfg, line, 3);
            let expected = mk_input_nl(9);
            let line1 = mk_expected(&expected, 0, 2, Some((*SD, W)));
            let line2 = mk_expected(&expected, 2, 4, Some((*SD, W)));
            let line3 = mk_expected(&expected, 4, 6, Some((*SD, W)));
            let line4 = mk_expected(&expected, 6, 8, Some((*SD, W)));
            let line5 = mk_expected(&expected, 8, 11, None);
            assert_eq!(lines, vec![line1, line2, line3, line4, line5]);
        }

        {
            let line = mk_input_nl(10);
            let lines = wrap_test(&cfg, line, 3);
            let expected = mk_input_nl(10);
            let line1 = mk_expected(&expected, 0, 2, Some((*SD, W)));
            let line2 = mk_expected(&expected, 2, 4, Some((*SD, W)));
            let line3 = mk_expected(&expected, 4, 6, Some((*SD, W)));
            let line4 = mk_expected(&expected, 6, 8, Some((*SD, W)));
            let line5 = mk_expected(&expected, 8, 11, Some((*S2, "\n")));
            assert_eq!(lines, vec![line1, line2, line3, line4, line5]);
        }

        {
            let line = vec![(*S1, "abc"), (*S2, "01230123012301230123"), (*S1, "ZZZZZ")];

            let wcfg1 = mk_wrap_cfg(&WrapConfig {
                max_lines: 1,
                ..TEST_WRAP_CFG.clone()
            });
            let wcfg2 = mk_wrap_cfg(&WrapConfig {
                max_lines: 2,
                ..TEST_WRAP_CFG.clone()
            });
            let wcfg3 = mk_wrap_cfg(&WrapConfig {
                max_lines: 3,
                ..TEST_WRAP_CFG.clone()
            });

            let lines = wrap_test(&wcfg1, line.clone(), 4);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
            let lines = wrap_test(&wcfg2, line.clone(), 4);
            assert_eq!(lines.len(), 2);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
            let lines = wrap_test(&wcfg3, line.clone(), 4);
            assert_eq!(lines.len(), 3);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
        }
    }

    #[test]
    fn test_wrap_line_unicode() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        // from UnicodeSegmentation documentation and the linked
        // Unicode Standard Annex #29
        let line = vec![(*S1, "abc"), (*S2, "mnö̲"), (*S1, "xyz")];
        let lines = wrap_test(&cfg, line, 4);
        assert_eq!(
            lines,
            vec![
                vec![(*S1, "abc"), (*SD, W)],
                vec![(*S2, "mnö̲"), (*SD, W)],
                vec![(*S1, "xyz")]
            ]
        );

        // Not working: Tailored grapheme clusters: क्षि  = क् + षि
        //
        // Difference compare to previous example (even they may look like the
        // same width in text editor.) :
        //
        // width நி: 2
        // width ö̲: 1
        let line = vec![(*S1, "abc"), (*S2, "dநி"), (*S1, "ghij")];
        let lines = wrap_test(&cfg, line, 4);
        assert_eq!(
            lines,
            vec![
                vec![(*S1, "abc"), (*SD, W)],
                vec![(*S2, "dநி"), (*SD, W)],
                vec![(*S1, "ghij")]
            ]
        );
    }

    const HUNK_ZERO_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -4,3 +15,3 @@
 abcdefghijklmnopqrstuvwxzy 0123456789 0123456789 0123456789 0123456789 0123456789
-a = 1
+a = 2
";

    const HUNK_ZERO_LARGE_LINENUMBERS_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -10,3 +101999,3 @@
 abcdefghijklmnopqrstuvwxzy 0123456789 0123456789 0123456789 0123456789 0123456789
-a = 1
+a = 2
";

    const HUNK_MP_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -4,3 +15,3 @@
 abcdefghijklmnopqrstuvwxzy 0123456789 0123456789 0123456789 0123456789 0123456789
-a = 0123456789 0123456789 0123456789 0123456789 0123456789
+b = 0123456789 0123456789 0123456789 0123456789 0123456789
";

    const HUNK_ALIGN_DIFF_HEADER: &str = "--- a\n+++ b\n@@ -1,1 +1,1 @@\n";
    const HUNK_ALIGN_DIFF_SHORT: &str = ".........1.........2....\n";
    const HUNK_ALIGN_DIFF_LONG: &str =
        ".........1.........2.........3.........4.........5.........6\n";

    #[test]
    fn test_wrap_with_unequal_hunk_zero_width() {
        DeltaTest::with_args(&default_wrap_cfg_plus(&[
            "--side-by-side",
            "--line-numbers-left-format",
            "│L│",
            "--line-numbers-right-format",
            "│RRRR│",
            "--width",
            "40",
            "--line-fill-method",
            "spaces",
        ]))
        .set_config(|cfg| cfg.truncation_symbol = ">".into())
        .with_input(HUNK_ZERO_DIFF)
        .expect_after_header(
            r#"
            │L│abcdefghijklm+   │RRRR│abcdefghijklm+
            │L│nopqrstuvwxzy+   │RRRR│nopqrstuvwxzy+
            │L│ 0123456789 0+   │RRRR│ 0123456789 0+
            │L│123456789 012+   │RRRR│123456789 012+
            │L│3456789 01234567>│RRRR│3456789 01234>
            │L│a = 1            │RRRR│a = 2         "#,
        );
    }

    #[test]
    fn test_wrap_with_large_hunk_zero_line_numbers() {
        DeltaTest::with_args(&default_wrap_cfg_plus(&[
            "--side-by-side",
            "--line-numbers-left-format",
            "│LLL│",
            "--line-numbers-right-format",
            "│WW {nm} +- {np:2} WW│",
            "--width",
            "60",
            "--line-fill-method",
            "ansi",
        ]))
        .set_config(|cfg| cfg.truncation_symbol = ">".into())
        .with_input(HUNK_ZERO_LARGE_LINENUMBERS_DIFF)
        .expect_after_header(
            r#"
            │LLL│abcde+                   │WW   10   +- 101999 WW│abcde+
            │LLL│fghij+                   │WW        +-        WW│fghij+
            │LLL│klmno+                   │WW        +-        WW│klmno+
            │LLL│pqrst+                   │WW        +-        WW│pqrst+
            │LLL│uvwxzy 0123456789 012345>│WW        +-        WW│uvwxz>
            │LLL│a = 1                    │WW        +- 102000 WW│a = 2"#,
        );
    }

    #[test]
    fn test_wrap_with_keep_markers() {
        use crate::features::side_by_side::ansifill::ODD_PAD_CHAR;
        let t = DeltaTest::with_args(&default_wrap_cfg_plus(&[
            "--side-by-side",
            "--keep-plus-minus-markers",
            "--width",
            "45",
        ]))
        .set_config(|cfg| cfg.truncation_symbol = ">".into())
        .with_input(HUNK_MP_DIFF)
        .expect_after_header(
            r#"
            │  4 │ abcdefghijklmn+ │ 15 │ abcdefghijklmn+
            │    │ opqrstuvwxzy 0+ │    │ opqrstuvwxzy 0+
            │    │ 123456789 0123+ │    │ 123456789 0123+
            │    │ 456789 0123456+ │    │ 456789 0123456+
            │    │ 789 0123456789> │    │ 789 0123456789>
            │  5 │-a = 0123456789+ │ 16 │+b = 0123456789+
            │    │  0123456789 01+ │    │  0123456789 01+
            │    │ 23456789 01234+ │    │ 23456789 01234+
            │    │ 56789 01234567+ │    │ 56789 01234567+
            │    │ 89              │    │ 89"#,
            // this column here is^ where ODD_PAD_CHAR is inserted due to the odd 45 width
        );

        assert!(!t.output.is_empty());

        for line in t.output.lines().skip(crate::config::HEADER_LEN) {
            assert_eq!(line.chars().nth(22), Some(ODD_PAD_CHAR));
        }
    }

    #[test]
    fn test_alignment_2_lines_vs_3_lines() {
        let config =
            make_config_from_args(&default_wrap_cfg_plus(&["--side-by-side", "--width", "55"]));

        {
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_SHORT}+{HUNK_ALIGN_DIFF_LONG}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2< │  1 │.........1.........2+
                    │    │                >.... │    │.........3.........4+
                    │    │                      │    │.........5.........6"#,
                );
            // the place where ODD_PAD_CHAR^ is inserted due to the odd 55 width
        }

        {
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_LONG}+{HUNK_ALIGN_DIFF_SHORT}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2+ │  1 │.........1.........2<
                    │    │.........3.........4+ │    │                >....
                    │    │.........5.........6  │    │"#,
                );
        }
    }

    #[test]
    fn test_alignment_1_line_vs_3_lines() {
        let config = make_config_from_args(&default_wrap_cfg_plus(&[
            "--side-by-side",
            "--width",
            "61",
            "--line-fill-method",
            "spaces",
        ]));

        {
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_SHORT}+{HUNK_ALIGN_DIFF_LONG}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2....│  1 │.........1.........2...+
                    │    │                        │    │......3.........4......+
                    │    │                        │    │...5.........6          "#,
                );
        }

        {
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_LONG}+{HUNK_ALIGN_DIFF_SHORT}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2...+│  1 │.........1.........2....
                    │    │......3.........4......+│    │
                    │    │...5.........6          │    │"#,
                );
        }
    }

    #[test]
    fn test_wrap_max_lines_2() {
        // TODO overriding is not possible, need to change config directly
        let mut config = make_config_from_args(&default_wrap_cfg_plus(&[
            // "--wrap-max-lines",
            // "2",
            "--side-by-side",
            "--width",
            "72",
            "--line-fill-method",
            "spaces",
        ]));
        config.truncation_symbol = ">".into();

        {
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_SHORT}+{HUNK_ALIGN_DIFF_LONG}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2....      │  1 │.........1.........2.........+
                    │    │                              │    │3.........4.........5........+
                    │    │                              │    │.6                            "#,
                );
        }

        {
            config.wrap_config.max_lines = 2;
            DeltaTest::with_config(&config)
                .with_input(&format!(
                    "{HUNK_ALIGN_DIFF_HEADER}-{HUNK_ALIGN_DIFF_SHORT}+{HUNK_ALIGN_DIFF_LONG}",
                ))
                .expect_after_header(
                    r#"
                    │  1 │.........1.........2....      │  1 │.........1.........2.........+
                    │    │                              │    │3.........4.........5........>"#,
                );
        }
    }
}
