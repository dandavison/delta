use syntect::highlighting::Style as SyntectStyle;
use unicode_segmentation::UnicodeSegmentation;

use crate::config::Config;
use crate::delta::State;
use crate::features::line_numbers;
use crate::features::side_by_side::line_is_too_long;
use crate::features::side_by_side::LeftRight;
use crate::features::side_by_side::PanelSide::*;
use crate::style::Style;

use super::{line_numbers::SideBySideLineWidth, side_by_side::available_line_width};

#[derive(Clone)]
pub struct WrapConfig {
    pub wrap_symbol: String,
    pub wrap_right_symbol: String,
    pub right_align_symbol: String,
    pub use_wrap_right_permille: usize,
    pub max_lines: usize,
}

// Wrap the given `line` if it is longer than `line_width`. Wrap to at most
// `wrap_config.max_lines` lines, then truncate again. Place `wrap_config.wrap_symbol`
// at then end of wrapped lines. However if wrapping results in only one extra line
// and if the width of the wrapped line is less than `wrap_config.use_wrap_right_permille`
// right-align the second line and use `wrap_config.wrap_right_symbol`.
//
// The input `line` is expected to start with an (ultimately not printed) "+/-/ " prefix.
// A prefix ("_") is also added to the start of wrapped lines.
pub fn wrap_line<'a, I, S>(
    config: &'a Config,
    line: I,
    line_width: usize,
    fill_style: &S,
    inline_hint_style: &Option<S>,
) -> Vec<Vec<(S, &'a str)>>
where
    I: IntoIterator<Item = (S, &'a str)> + std::fmt::Debug,
    <I as IntoIterator>::IntoIter: DoubleEndedIterator,
    S: Copy + Default + std::fmt::Debug,
{
    let mut result = Vec::new();

    let wrap_config = &config.wrap_config;

    // Symbol which:
    //  - represents the additional "+/-/ " prefix on the unwrapped input line, its
    //    length is added to the line_width.
    //  - can be more prominent than a space because syntax highlighting has already
    //    been done.
    //  - is added to the beginning of wrapped lines so the wrapped lines also have
    //    a prefix (which is not printed).
    const LINEPREFIX: &str = "_";
    static_assertions::const_assert_eq!(LINEPREFIX.len(), 1); // must be a 1-byte char

    let max_len = line_width + LINEPREFIX.len();

    // Stay defensive just in case: guard against infinite loops.
    let mut n = max_len * wrap_config.max_lines * 2;

    let mut curr_line = Vec::new();
    let mut curr_len = 0;

    // Determine the background (diff) and color (syntax) of
    // an inserted symbol.
    let symbol_style = match inline_hint_style {
        Some(style) => *style,
        None => *fill_style,
    };

    let mut stack = line.into_iter().rev().collect::<Vec<_>>();

    while !stack.is_empty()
        && result.len() + 1 < wrap_config.max_lines
        && max_len > LINEPREFIX.len()
        && n > 0
    {
        n -= 1;

        let (style, text, graphemes) = stack
            .pop()
            .map(|(style, text)| (style, text, text.grapheme_indices(true).collect::<Vec<_>>()))
            .unwrap();
        let new_sum = curr_len + graphemes.len();

        let must_split = if new_sum < max_len {
            curr_line.push((style, text));
            curr_len = new_sum;
            false
        } else if new_sum == max_len {
            match stack.last() {
                // Perfect fit, no need to make space for a `wrap_symbol`.
                None => {
                    curr_line.push((style, text));
                    curr_len = new_sum;
                    false
                }
                // A single '\n' left on the stack can be pushed onto the current line.
                Some((next_style, nl)) if stack.len() == 1 && *nl == "\n" => {
                    curr_line.push((style, text));
                    curr_line.push((*next_style, *nl));
                    stack.pop();
                    curr_len = new_sum; // do not count the '\n'
                    false
                }
                _ => true,
            }
        } else if new_sum == max_len + 1 && stack.is_empty() {
            // If the one overhanging char is '\n' then keep it on the current line.
            if !text.is_empty() && *text.as_bytes().last().unwrap() == b'\n' {
                curr_line.push((style, text));
                curr_len = new_sum - 1; // do not count the '\n'
                false
            } else {
                true
            }
        } else {
            true
        };

        // Text must be split, one part (or just `wrap_symbol`) is added to the
        // current line, the other is pushed onto the stack.
        if must_split {
            let grapheme_split_pos = graphemes.len() - (new_sum - max_len) - 1;

            let next_line = if grapheme_split_pos == 0 {
                text
            } else {
                let byte_split_pos = graphemes[grapheme_split_pos].0;
                let this_line = &text[..byte_split_pos];
                curr_line.push((style, this_line));
                &text[byte_split_pos..]
            };
            stack.push((style, next_line));

            curr_line.push((symbol_style, &wrap_config.wrap_symbol));
            result.push(curr_line);

            curr_line = vec![(S::default(), LINEPREFIX)];
            curr_len = LINEPREFIX.len();
        }
    }

    // Right-align wrapped line:
    // Done if wrapping adds exactly one line and it is less than the
    // given permille of panel width wide. Also change the wrap symbol at the
    // end of the previous (first) line.
    if result.len() == 1 && !curr_line.is_empty() {
        let current_permille = (curr_len * 1000) / max_len;

        // &config.wrap_config.right_align_symbol length
        const RIGHT_ALIGN_SYMBOL_LEN: usize = 1;
        let pad_len = max_len.saturating_sub(curr_len - LINEPREFIX.len() + RIGHT_ALIGN_SYMBOL_LEN);

        if wrap_config.use_wrap_right_permille > current_permille
            && result.len() == 1
            && pad_len > RIGHT_ALIGN_SYMBOL_LEN
        {
            const SPACES: &str = "        ";

            match result.last_mut() {
                Some(ref mut vec) if !vec.is_empty() => {
                    vec.last_mut().unwrap().1 = &wrap_config.wrap_right_symbol
                }
                _ => unreachable!("wrap result must not be empty"),
            }

            let mut right_aligned_line = vec![(S::default(), LINEPREFIX)];

            for _ in 0..(pad_len / SPACES.len()) {
                right_aligned_line.push((*fill_style, SPACES));
            }

            match pad_len % SPACES.len() {
                0 => (),
                n => right_aligned_line.push((*fill_style, &SPACES[0..n])),
            }

            right_aligned_line.push((symbol_style, &wrap_config.right_align_symbol));

            // skip LINEPREFIX
            right_aligned_line.extend(curr_line.into_iter().skip(1));

            curr_line = right_aligned_line;
        }
    }

    if !curr_line.is_empty() {
        result.push(curr_line);
    }

    if !stack.is_empty() {
        if result.is_empty() {
            result.push(Vec::new());
        }
        result
            .last_mut()
            .map(|vec| vec.extend(stack.into_iter().rev()));
    }

    result
}

fn wrap_if_too_long<'a, S>(
    config: &'a Config,
    wrapped: &mut Vec<Vec<(S, &'a str)>>,
    input_vec: Vec<(S, &'a str)>,
    must_wrap: bool,
    line_width: usize,
    fill_style: &S,
    inline_hint_style: &Option<S>,
) -> (usize, usize)
where
    S: Copy + Default + std::fmt::Debug,
{
    let size_prev = wrapped.len();

    if must_wrap {
        wrapped.append(&mut wrap_line(
            &config,
            input_vec.into_iter(),
            line_width,
            fill_style,
            &inline_hint_style,
        ));
    } else {
        wrapped.push(input_vec.to_vec());
    }

    (size_prev, wrapped.len())
}

#[allow(clippy::comparison_chain, clippy::type_complexity)]
pub fn wrap_plusminus_block<'c: 'a, 'a>(
    config: &'c Config,
    syntax: LeftRight<Vec<Vec<(SyntectStyle, &'a str)>>>,
    diff: LeftRight<Vec<Vec<(Style, &'a str)>>>,
    alignment: &[(Option<usize>, Option<usize>)],
    line_width: &SideBySideLineWidth,
    wrapinfo: &'a LeftRight<Vec<bool>>,
) -> (
    Vec<(Option<usize>, Option<usize>)>,
    LeftRight<Vec<State>>,
    LeftRight<Vec<Vec<(SyntectStyle, &'a str)>>>,
    LeftRight<Vec<Vec<(Style, &'a str)>>>,
) {
    let mut new_alignment = Vec::new();
    let mut new_states = LeftRight::<Vec<State>>::default();
    let mut new_wrapped_syntax = LeftRight::default();
    let mut new_wrapped_diff = LeftRight::default();

    // Turn all these into iterators so they can be advanced according
    // to the alignment.
    let mut syntax = LeftRight::new(syntax.left.into_iter(), syntax.right.into_iter());
    let mut diff = LeftRight::new(diff.left.into_iter(), diff.right.into_iter());
    let mut wrapinfo = LeftRight::new(wrapinfo.left.iter(), wrapinfo.right.iter());

    let fill_style = LeftRight::new(&config.minus_style, &config.plus_style);

    // Internal helper function to perform wrapping for both the syntax and the
    // diff highlighting (SyntectStyle and Style).
    #[allow(clippy::too_many_arguments)]
    pub fn wrap_syntax_and_diff<'a, ItSyn, ItDiff, ItWrap>(
        config: &'a Config,
        wrapped_syntax: &mut Vec<Vec<(SyntectStyle, &'a str)>>,
        wrapped_diff: &mut Vec<Vec<(Style, &'a str)>>,
        syntax_iter: &mut ItSyn,
        diff_iter: &mut ItDiff,
        wrapinfo_iter: &mut ItWrap,
        line_width: usize,
        fill_style: &Style,
        errhint: &'a str,
    ) -> (usize, usize)
    where
        ItSyn: Iterator<Item = Vec<(SyntectStyle, &'a str)>>,
        ItDiff: Iterator<Item = Vec<(Style, &'a str)>>,
        ItWrap: Iterator<Item = &'a bool>,
    {
        let must_wrap = *wrapinfo_iter
            .next()
            .unwrap_or_else(|| panic!("bad wrap info {}", errhint));

        let (start, extended_to) = wrap_if_too_long(
            &config,
            wrapped_syntax,
            syntax_iter
                .next()
                .unwrap_or_else(|| panic!("bad syntax alignment {}", errhint)),
            must_wrap,
            line_width,
            &SyntectStyle::default(),
            &config.inline_hint_color,
        );

        let (start2, extended_to2) = wrap_if_too_long(
            &config,
            wrapped_diff,
            diff_iter
                .next()
                .unwrap_or_else(|| panic!("bad diff alignment {}", errhint)),
            must_wrap,
            line_width,
            &fill_style,
            &None,
        );

        // The underlying text is the same for the style and diff, so
        // the length of the wrapping should be identical:
        assert_eq!(
            (start, extended_to),
            (start2, extended_to2),
            "syntax and diff wrapping differs {}",
            errhint
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

                if plus_minus < 0 {
                    for n in plus_start + plus_minus.abs() as usize..p_extended_to {
                        new_alignment.push((None, Some(n)));
                    }
                } else if plus_minus > 0 {
                    for n in minus_start + plus_minus as usize..m_extended_to {
                        new_alignment.push((Some(n), None));
                    }
                }

                (minus_extended, plus_extended)
            }
            _ => panic!("unexpected None-None alignment"),
        };

        if minus_extended > 0 {
            new_states.left.push(State::HunkMinus(None));
            for _ in 1..minus_extended {
                new_states.left.push(State::HunkMinusWrapped);
            }
        }
        if plus_extended > 0 {
            new_states.right.push(State::HunkPlus(None));
            for _ in 1..plus_extended {
                new_states.right.push(State::HunkPlusWrapped);
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
    raw_line: &str,
    mut states: Vec<State>,
    syntax_style_sections: Vec<Vec<(SyntectStyle, &'a str)>>,
    diff_style_sections: Vec<Vec<(Style, &'a str)>>,
    line_numbers_data: &Option<&mut line_numbers::LineNumbersData>,
) -> (
    Vec<State>,
    Vec<Vec<(SyntectStyle, &'a str)>>,
    Vec<Vec<(Style, &'a str)>>,
) {
    // The width is the minimum of the left/right side. The panels should be equally sized,
    // but in rare cases the remaining panel width might differ due to the space the line
    // numbers take up.
    let line_width = if let Some(line_numbers_data) = line_numbers_data {
        let width = available_line_width(&config, &line_numbers_data);
        std::cmp::min(width.left, width.right)
    } else {
        std::cmp::min(
            config.side_by_side_data[Left].width,
            config.side_by_side_data[Right].width,
        )
    };

    // Called with a single line (based on raw_line), so no need to use the 1-sized bool vector,
    // if that changes the wrapping logic should be updated as well.
    assert_eq!(diff_style_sections.len(), 1);

    let should_wrap = line_is_too_long(&raw_line, line_width);

    if should_wrap {
        let syntax_style = wrap_line(
            &config,
            syntax_style_sections.into_iter().flatten(),
            line_width,
            &SyntectStyle::default(),
            &config.inline_hint_color,
        );
        let diff_style = wrap_line(
            &config,
            diff_style_sections.into_iter().flatten(),
            line_width,
            &config.null_style,
            &None,
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

    use super::wrap_line;
    use super::WrapConfig;
    use crate::ansi::strip_ansi_codes;
    use crate::cli::Opt;
    use crate::config::Config;
    use crate::style::Style;
    use crate::tests::integration_test_utils::integration_test_utils::{
        make_config_from_args, run_delta,
    };

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

    const W: &str = &"+";
    const WR: &str = &"<";
    const RA: &str = &">";
    lazy_static! {
        static ref TEST_WRAP_CFG: WrapConfig = WrapConfig {
            wrap_symbol: W.into(),
            wrap_right_symbol: WR.into(),
            right_align_symbol: RA.into(),
            use_wrap_right_permille: 370,
            max_lines: 5,
        };
    }

    fn mk_wrap_cfg(wrap_cfg: &WrapConfig) -> Config {
        let mut cfg: Config = Config::from(Opt::default());
        cfg.wrap_config = wrap_cfg.clone();
        cfg
    }

    fn wrap_test<'a, I, S>(cfg: &'a Config, line: I, line_width: usize) -> Vec<Vec<(S, &'a str)>>
    where
        I: IntoIterator<Item = (S, &'a str)> + std::fmt::Debug,
        <I as IntoIterator>::IntoIter: DoubleEndedIterator,
        S: Copy + Default + std::fmt::Debug,
    {
        wrap_line(&cfg, line, line_width, &S::default(), &None)
    }

    #[test]
    fn test_wrap_line_plain() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        {
            let line = vec![(*SY, "_0")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*SY, "_0")]]);
        }

        {
            let line = vec![(*S1, "")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "")]]);
        }

        {
            let line = vec![(*S1, "_")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "_")]]);
        }

        {
            let line = vec![(*S1, "_"), (*S2, "0")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "_"), (*S2, "0")]]);
        }

        {
            let line = vec![(*S1, "_012"), (*S2, "34")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "_012"), (*S2, "34")]]);
        }

        {
            let line = vec![(*S1, "_012"), (*S2, "345")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "_012"), (*S2, "345")]]);
        }

        {
            let line = vec![(*S1, "_012"), (*S2, "3456")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(
                lines,
                vec![
                    vec![(*S1, "_012"), (*S2, "34"), (*SD, "+")],
                    vec![(*SD, "_"), (*S2, "56")]
                ]
            );
        }
    }

    #[test]
    fn test_wrap_line_newlines() {
        fn mk_input(len: usize) -> Vec<(Style, &'static str)> {
            const IN: &str = "_0123456789abcdefZ";
            let v = &[*S1, *S2];
            let s1s2 = v.iter().cycle();
            let text: Vec<_> = IN.matches(|_| true).take(len + 1).collect();
            s1s2.zip(text.iter())
                .map(|(style, text)| (style.clone(), *text))
                .collect()
        }
        fn mk_input_nl(len: usize) -> Vec<(Style, &'static str)> {
            const NL: &str = "\n";
            let mut line = mk_input(len);
            line.push((*S2, NL));
            line
        }
        fn mk_expected<'a>(
            prepend: Option<(Style, &'a str)>,
            vec: &Vec<(Style, &'a str)>,
            from: usize,
            to: usize,
            append: Option<(Style, &'a str)>,
        ) -> Vec<(Style, &'a str)> {
            let mut result: Vec<_> = vec[from..to].iter().cloned().collect();
            if let Some(val) = append {
                result.push(val);
            }
            if let Some(val) = prepend {
                result.insert(0, val);
            }
            result
        }

        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        {
            let line = vec![(*S1, "_012"), (*S2, "345\n")];
            let lines = wrap_test(&cfg, line, 6);
            assert_eq!(lines, vec![vec![(*S1, "_012"), (*S2, "345\n")]]);
        }

        {
            for i in 0..=6 {
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
            let line1 = mk_expected(None, &expected, 0, 3, Some((*SD, &W)));
            let line2 = mk_expected(Some((*SD, "_")), &expected, 3, 5, Some((*SD, &W)));
            let line3 = mk_expected(Some((*SD, "_")), &expected, 5, 7, Some((*SD, &W)));
            let line4 = mk_expected(Some((*SD, "_")), &expected, 7, 11, None);
            assert_eq!(lines, vec![line1, line2, line3, line4]);
        }

        {
            let line = mk_input_nl(10);
            let lines = wrap_test(&cfg, line, 3);
            let expected = mk_input_nl(10);
            let line1 = mk_expected(None, &expected, 0, 3, Some((*SD, &W)));
            let line2 = mk_expected(Some((*SD, "_")), &expected, 3, 5, Some((*SD, &W)));
            let line3 = mk_expected(Some((*SD, "_")), &expected, 5, 7, Some((*SD, &W)));
            let line4 = mk_expected(Some((*SD, "_")), &expected, 7, 9, Some((*SD, &W)));
            let line5 = mk_expected(Some((*SD, "_")), &expected, 9, 11, Some((*S2, "\n")));
            assert_eq!(lines, vec![line1, line2, line3, line4, line5]);
        }

        {
            let line = vec![(*S1, "_abc"), (*S2, "01230123012301230123"), (*S1, "ZZZZZ")];

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

            let lines = wrap_line(&wcfg1, line.clone(), 4, &Style::default(), &None);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
            let lines = wrap_line(&wcfg2, line.clone(), 4, &Style::default(), &None);
            assert_eq!(lines.len(), 2);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
            let lines = wrap_line(&wcfg3, line.clone(), 4, &Style::default(), &None);
            assert_eq!(lines.len(), 3);
            assert_eq!(lines.last().unwrap().last().unwrap().1, "ZZZZZ");
        }
    }

    #[test]
    fn test_wrap_line_right() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        let line = vec![(*S1, "_0123456789ab")];
        let lines = wrap_test(&cfg, line, 11);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].last().unwrap().1, WR);
        assert_eq!(
            lines[1],
            vec![
                (*SD, "_"),
                (*SD, "        "),
                (*SD, " "),
                (*SD, ">"),
                (*S1, "ab")
            ]
        );
    }

    #[test]
    fn test_wrap_line_unicode() {
        let cfg = mk_wrap_cfg(&TEST_WRAP_CFG);

        // from UnicodeSegmentation documentation and the linked
        // Unicode Standard Annex #29
        let line = vec![(*S1, "_abc"), (*S2, "mnö̲"), (*S1, "xyz")];
        let lines = wrap_test(&cfg, line, 4);
        assert_eq!(
            lines,
            vec![
                vec![(*S1, "_abc"), (*SD, &W)],
                vec![(*SD, "_"), (*S2, "mnö̲"), (*SD, &W)],
                vec![(*SD, "_"), (*S1, "xyz")]
            ]
        );

        // Not working: Tailored grapheme clusters: क्षि  = क् + षि
        let line = vec![(*S1, "_abc"), (*S2, "deநி"), (*S1, "ghij")];
        let lines = wrap_test(&cfg, line, 4);
        assert_eq!(
            lines,
            vec![
                vec![(*S1, "_abc"), (*SD, &W)],
                vec![(*SD, "_"), (*S2, "deநி"), (*SD, &W)],
                vec![(*SD, "_"), (*S1, "ghij")]
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

    #[test]
    fn test_wrap_with_linefmt1() {
        let mut config = make_config_from_args(&[
            "--side-by-side",
            "--line-numbers-left-format",
            "│L│",
            "--line-numbers-right-format",
            "│RRRR│",
            "--width",
            "40",
        ]);
        config.truncation_symbol = ">".into();

        config.wrap_config = TEST_WRAP_CFG.clone();

        let output = run_delta(HUNK_ZERO_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let lines: Vec<_> = output.lines().skip(7).collect();
        let expected = vec![
            "│L│abcdefghijklm+   │RRRR│abcdefghijklm+",
            "│L│nopqrstuvwxzy+   │RRRR│nopqrstuvwxzy+",
            "│L│ 0123456789 0+   │RRRR│ 0123456789 0+",
            "│L│123456789 012+   │RRRR│123456789 012+",
            "│L│3456789 01234567>│RRRR│3456789 01234>",
            "│L│a = 1            │RRRR│a = 2",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn test_wrap_with_linefmt2() {
        let mut config = make_config_from_args(&[
            "--side-by-side",
            "--line-numbers-left-format",
            "│LLL│",
            "--line-numbers-right-format",
            "│WW {nm} +- {np:2} WW│",
            "--width",
            "60",
        ]);
        config.wrap_config = TEST_WRAP_CFG.clone();

        config.truncation_symbol = ">".into();
        let output = run_delta(HUNK_ZERO_LARGE_LINENUMBERS_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let lines: Vec<_> = output.lines().skip(7).collect();
        let expected = vec![
            "│LLL│abcde+                   │WW   10   +- 101999 WW│abcde+",
            "│LLL│fghij+                   │WW        +-        WW│fghij+",
            "│LLL│klmno+                   │WW        +-        WW│klmno+",
            "│LLL│pqrst+                   │WW        +-        WW│pqrst+",
            "│LLL│uvwxzy 0123456789 012345>│WW        +-        WW│uvwxz>",
            "│LLL│a = 1                    │WW        +- 102000 WW│a = 2",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn test_wrap_with_keep_markers() {
        let mut config = make_config_from_args(&[
            "--side-by-side",
            "--keep-plus-minus-markers",
            "--width",
            "45",
        ]);
        config.wrap_config = TEST_WRAP_CFG.clone();
        config.truncation_symbol = ">".into();
        let output = run_delta(HUNK_MP_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let lines: Vec<_> = output.lines().skip(7).collect();
        let expected = vec![
            "│ 4  │ abcdefghijklmn+│ 15 │ abcdefghijklmn+",
            "│    │ opqrstuvwxzy 0+│    │ opqrstuvwxzy 0+",
            "│    │ 123456789 0123+│    │ 123456789 0123+",
            "│    │ 456789 0123456+│    │ 456789 0123456+",
            "│    │ 789 0123456789>│    │ 789 0123456789>",
            "│ 5  │-a = 0123456789+│ 16 │+b = 0123456789+",
            "│    │  0123456789 01+│    │  0123456789 01+",
            "│    │ 23456789 01234+│    │ 23456789 01234+",
            "│    │ 56789 01234567+│    │ 56789 01234567+",
            "│    │ 89             │    │ 89",
        ];
        assert_eq!(lines, expected);
    }
}
