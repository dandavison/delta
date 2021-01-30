use std::ops::{Index, IndexMut};

use itertools::Itertools;
use syntect::highlighting::Style as SyntectStyle;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::cli;
use crate::config::Config;
use crate::delta::State;
use crate::features::line_numbers;
use crate::features::OptionValueFunction;
use crate::paint::BgFillMethod;
use crate::paint::BgFillWidth;
use crate::paint::Painter;
use crate::style::Style;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "side-by-side",
            bool,
            None,
            _opt => true
        ),
        ("features", bool, None, _opt => "line-numbers"),
        ("line-numbers-left-format", String, None, _opt => "│{nm:^4}│".to_string()),
        ("line-numbers-right-format", String, None, _opt => "│{np:^4}│".to_string())
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelSide {
    Left,
    Right,
}

use PanelSide::*;

#[derive(Debug, PartialEq, Eq)]
pub struct LeftRight<T> {
    pub left: T,
    pub right: T,
}

impl<T> Index<PanelSide> for LeftRight<T> {
    type Output = T;
    fn index(&self, side: PanelSide) -> &Self::Output {
        match side {
            PanelSide::Left => &self.left,
            PanelSide::Right => &self.right,
        }
    }
}

impl<T> IndexMut<PanelSide> for LeftRight<T> {
    fn index_mut(&mut self, side: PanelSide) -> &mut Self::Output {
        match side {
            PanelSide::Left => &mut self.left,
            PanelSide::Right => &mut self.right,
        }
    }
}

impl<T> LeftRight<T> {
    pub fn new(left: T, right: T) -> Self {
        LeftRight { left, right }
    }
}

impl<T: Default> Default for LeftRight<T> {
    fn default() -> Self {
        Self {
            left: T::default(),
            right: T::default(),
        }
    }
}

pub struct Panel {
    pub width: usize,
    pub offset: usize,
}

pub type SideBySideData = LeftRight<Panel>;

impl SideBySideData {
    pub fn new_sbs(decorations_width: &cli::Width, available_terminal_width: &usize) -> Self {
        let panel_width = match decorations_width {
            cli::Width::Fixed(w) => w / 2,
            _ => available_terminal_width / 2,
        };
        SideBySideData::new(
            Panel {
                width: panel_width,
                offset: 0,
            },
            Panel {
                width: panel_width,
                offset: 0,
            },
        )
    }
}

pub fn available_line_width(
    config: &Config,
    data: &line_numbers::LineNumbersData,
) -> line_numbers::SideBySideLineWidth {
    let linennumbers_width = data.formatted_width();

    // The width can be reduced by the line numbers and/or a possibly kept 1-wide "+/-/ " prefix.
    let line_width = |side| {
        config.side_by_side_data[side]
            .width
            .saturating_sub(linennumbers_width[side])
            .saturating_sub(config.keep_plus_minus_markers as usize)
    };

    LeftRight::new(line_width(PanelSide::Left), line_width(PanelSide::Right))
}

pub fn line_is_too_long(line: &str, line_width: usize) -> bool {
    let line_sum = line.graphemes(true).count();

    // `line_sum` is too large, because both a leading "+/-/ " and a trailing
    // newline are present, counted, but are never printed. So allow two more
    // characters.
    line_sum > line_width + 2
}

/// Emit a sequence of minus and plus lines in side-by-side mode.
#[allow(clippy::too_many_arguments)]
pub fn paint_minus_and_plus_lines_side_by_side<'a>(
    syntax_left_right: LeftRight<Vec<Vec<(SyntectStyle, &str)>>>,
    diff_left_right: LeftRight<Vec<Vec<(Style, &str)>>>,
    states_left_right: LeftRight<Vec<&'a State>>,
    line_alignment: Vec<(Option<usize>, Option<usize>)>,
    output_buffer: &mut String,
    config: &Config,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    background_color_extends_to_terminal_width: LeftRight<BgFillWidth>,
) {
    for (minus_line_index, plus_line_index) in line_alignment {
        output_buffer.push_str(&paint_left_panel_minus_line(
            minus_line_index,
            &syntax_left_right[Left],
            &diff_left_right[Left],
            match minus_line_index {
                Some(i) => states_left_right[Left][i],
                None => &State::HunkMinus(None),
            },
            line_numbers_data,
            if config.keep_plus_minus_markers {
                Some(config.minus_style.paint("-"))
            } else {
                None
            },
            background_color_extends_to_terminal_width[Left],
            config,
        ));
        output_buffer.push_str(&paint_right_panel_plus_line(
            plus_line_index,
            &syntax_left_right[Right],
            &diff_left_right[Right],
            match plus_line_index {
                Some(i) => states_left_right[Right][i],
                None => &State::HunkPlus(None),
            },
            line_numbers_data,
            if config.keep_plus_minus_markers {
                Some(config.plus_style.paint("+"))
            } else {
                None
            },
            background_color_extends_to_terminal_width[Right],
            config,
        ));
        output_buffer.push('\n');
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_zero_lines_side_by_side(
    syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
    diff_style_sections: Vec<Vec<(Style, &str)>>,
    output_buffer: &mut String,
    config: &Config,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    painted_prefix: Option<ansi_term::ANSIString>,
    background_color_extends_to_terminal_width: BgFillWidth,
) {
    let state = State::HunkZero;

    for (line_index, (syntax_sections, diff_sections)) in syntax_style_sections
        .iter()
        .zip_eq(diff_style_sections.iter())
        .enumerate()
    {
        for panel_side in &[PanelSide::Left, PanelSide::Right] {
            let (mut panel_line, panel_line_is_empty) = Painter::paint_line(
                syntax_sections,
                diff_sections,
                &state,
                line_numbers_data,
                Some(*panel_side),
                painted_prefix.clone(),
                config,
            );
            pad_panel_line_to_width(
                &mut panel_line,
                panel_line_is_empty,
                Some(line_index),
                &diff_style_sections,
                &state,
                *panel_side,
                background_color_extends_to_terminal_width,
                config,
            );
            output_buffer.push_str(&panel_line);

            if panel_side == &PanelSide::Left {
                // TODO: Avoid doing the superimpose_style_sections work twice.
                // HACK: These are getting incremented twice, so knock them back down once.
                if let Some(d) = line_numbers_data.as_mut() {
                    d.line_number[Left] -= 1;
                    d.line_number[Right] -= 1;
                }
            }
        }
        output_buffer.push('\n');
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_left_panel_minus_line<'a>(
    line_index: Option<usize>,
    syntax_style_sections: &[Vec<(SyntectStyle, &str)>],
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &'a State,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    painted_prefix: Option<ansi_term::ANSIString>,
    background_color_extends_to_terminal_width: BgFillWidth,
    config: &Config,
) -> String {
    let (mut panel_line, panel_line_is_empty) = paint_minus_or_plus_panel_line(
        line_index,
        &syntax_style_sections,
        &diff_style_sections,
        state,
        line_numbers_data,
        PanelSide::Left,
        painted_prefix,
        config,
    );
    pad_panel_line_to_width(
        &mut panel_line,
        panel_line_is_empty,
        line_index,
        diff_style_sections,
        state,
        PanelSide::Left,
        background_color_extends_to_terminal_width,
        config,
    );

    panel_line
}

#[allow(clippy::too_many_arguments)]
fn paint_right_panel_plus_line<'a>(
    line_index: Option<usize>,
    syntax_style_sections: &[Vec<(SyntectStyle, &str)>],
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &'a State,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    painted_prefix: Option<ansi_term::ANSIString>,
    background_color_extends_to_terminal_width: BgFillWidth,
    config: &Config,
) -> String {
    let (mut panel_line, panel_line_is_empty) = paint_minus_or_plus_panel_line(
        line_index,
        &syntax_style_sections,
        &diff_style_sections,
        state,
        line_numbers_data,
        PanelSide::Right,
        painted_prefix,
        config,
    );

    pad_panel_line_to_width(
        &mut panel_line,
        panel_line_is_empty,
        line_index,
        diff_style_sections,
        state,
        PanelSide::Right,
        background_color_extends_to_terminal_width,
        config,
    );

    panel_line
}

fn get_right_fill_style_for_panel(
    line_is_empty: bool,
    line_index: Option<usize>,
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    panel_side: PanelSide,
    background_color_extends_to_terminal_width: BgFillWidth,
    config: &Config,
) -> (Option<BgFillMethod>, Style) {
    // If in the the left panel then it must be filled with spaces.
    let none_or_override = if panel_side == PanelSide::Left {
        Some(BgFillMethod::Spaces)
    } else {
        None
    };

    match (line_is_empty, line_index) {
        (true, _) => (none_or_override, config.null_style),
        (false, None) => (none_or_override, config.null_style),
        (false, Some(index)) => {
            let (bg_fill_mode, fill_style) =
                Painter::get_should_right_fill_background_color_and_fill_style(
                    &diff_style_sections[index],
                    state,
                    background_color_extends_to_terminal_width,
                    config,
                );

            match bg_fill_mode {
                None => (none_or_override, config.null_style),
                _ if panel_side == PanelSide::Left => (Some(BgFillMethod::Spaces), fill_style),
                _ => (bg_fill_mode, fill_style),
            }
        }
    }
}

/// Construct half of a minus or plus line under side-by-side mode, i.e. the half line that
/// goes in one or other panel. Return a tuple `(painted_half_line, is_empty)`.
// Suppose the line being displayed is a minus line with a paired plus line. Then both times
// this function is called, `line_index` will be `Some`. This case proceeds as one would
// expect: on the first call, we are constructing the left panel line, and we are passed
// `(Some(index), HunkMinus, Left)`. We pass `(HunkMinus, Left)` to
// `paint_line`. This has two consequences:
// 1. `format_and_paint_line_numbers` will increment the minus line number.
// 2. `format_and_paint_line_numbers` will emit the left line number field, and not the right.
//
// The second call does the analogous thing for the plus line to be displayed in the right panel:
// we are passed `(Some(index), HunkPlus, Right)` and we pass `(HunkPlus, Right)` to `paint_line`,
// causing it to increment the plus line number and emit the right line number field.
//
// Now consider the case where the line being displayed is a minus line with no paired plus line.
// The first call is as before. On the second call, we are passed `(None, HunkPlus, Right)` and we
// wish to display the right panel, with its line number container, but without any line number
// (and without any line contents). We do this by passing (HunkMinus, Right) to `paint_line`, since
// what this will do is set the line number pair in that function to `(Some(minus_number), None)`,
// and then only emit the right field (which has a None number, i.e. blank). However, it will also
// increment the minus line number, so we need to knock that back down.
#[allow(clippy::too_many_arguments)]
fn paint_minus_or_plus_panel_line(
    line_index: Option<usize>,
    syntax_style_sections: &[Vec<(SyntectStyle, &str)>],
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    panel_side: PanelSide,
    painted_prefix: Option<ansi_term::ANSIString>,
    config: &Config,
) -> (String, bool) {
    let (empty_line_syntax_sections, empty_line_diff_sections) = (Vec::new(), Vec::new());

    let (line_syntax_sections, line_diff_sections, state_for_line_numbers_field) =
        if let Some(index) = line_index {
            (
                &syntax_style_sections[index],
                &diff_style_sections[index],
                state.clone(),
            )
        } else {
            let opposite_state = match state {
                State::HunkMinus(x) => State::HunkPlus(x.clone()),
                State::HunkPlus(x) => State::HunkMinus(x.clone()),
                _ => unreachable!(),
            };
            (
                &empty_line_syntax_sections,
                &empty_line_diff_sections,
                opposite_state,
            )
        };

    let (line, line_is_empty) = Painter::paint_line(
        line_syntax_sections,
        line_diff_sections,
        &state_for_line_numbers_field,
        line_numbers_data,
        Some(panel_side),
        painted_prefix,
        config,
    );

    // Knock back down spuriously incremented line numbers. See comment above.
    match (state, &state_for_line_numbers_field) {
        (s, t) if s == t => {}
        (State::HunkPlus(_), State::HunkMinus(_)) => {
            if let Some(d) = line_numbers_data.as_mut() {
                d.line_number[Left] -= 1;
            }
        }
        (State::HunkMinus(_), State::HunkPlus(_)) => {
            if let Some(d) = line_numbers_data.as_mut() {
                d.line_number[Right] -= 1;
            }
        }
        _ => unreachable!(),
    }
    (line, line_is_empty)
}

/// Right-fill the background color of a line in a panel. If in the left panel this is always
/// done with spaces. The right panel can be filled with spaces or using ANSI sequences
/// instructing the terminal emulator to fill the background color rightwards.
#[allow(clippy::too_many_arguments, clippy::comparison_chain)]
fn pad_panel_line_to_width(
    panel_line: &mut String,
    panel_line_is_empty: bool,
    line_index: Option<usize>,
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    panel_side: PanelSide,
    background_color_extends_to_terminal_width: BgFillWidth,
    config: &Config,
) {
    // Emit empty line marker if the panel line is empty but not empty-by-construction. IOW if the
    // other panel contains a real line, and we are currently emitting an empty counterpart panel
    // to form the other half of the line, then don't emit the empty line marker.
    if panel_line_is_empty && line_index.is_some() {
        match state {
            State::HunkMinus(_) => Painter::mark_empty_line(
                &config.minus_empty_line_marker_style,
                panel_line,
                Some(" "),
            ),
            State::HunkPlus(_) => Painter::mark_empty_line(
                &config.plus_empty_line_marker_style,
                panel_line,
                Some(" "),
            ),
            State::HunkZero => {}
            _ => unreachable!(),
        };
    };

    let text_width = ansi::measure_text_width(&panel_line);
    let panel_width = config.side_by_side_data[panel_side].width;

    if text_width > panel_width {
        *panel_line =
            ansi::truncate_str(panel_line, panel_width, &config.truncation_symbol).to_string();
    }

    let (bg_fill_mode, fill_style) = get_right_fill_style_for_panel(
        panel_line_is_empty,
        line_index,
        &diff_style_sections,
        state,
        panel_side,
        background_color_extends_to_terminal_width,
        config,
    );

    match bg_fill_mode {
        Some(BgFillMethod::TryAnsiSequence) => {
            Painter::right_fill_background_color(panel_line, fill_style)
        }
        Some(BgFillMethod::Spaces) if text_width >= panel_width => (),
        Some(BgFillMethod::Spaces) => panel_line.push_str(
            &fill_style
                .paint(" ".repeat(panel_width - text_width))
                .to_string(),
        ),
        None => (),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::ansi::strip_ansi_codes;
    use crate::features::line_numbers::tests::*;
    use crate::tests::integration_test_utils::integration_test_utils::{
        make_config_from_args, run_delta,
    };

    #[test]
    fn test_two_minus_lines() {
        let config = make_config_from_args(&["--side-by-side", "--width", "40"]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│ 1  │a = 1         │    │", strip_ansi_codes(line_1));
        assert_eq!("│ 2  │b = 23456     │    │", strip_ansi_codes(line_2));
    }

    #[test]
    fn test_two_minus_lines_truncated() {
        let mut config = make_config_from_args(&["--side-by-side", "--width", "28"]);
        config.truncation_symbol = ">".into();
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│ 1  │a = 1   │    │", strip_ansi_codes(line_1));
        assert_eq!("│ 2  │b = 234>│    │", strip_ansi_codes(line_2));
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config_from_args(&["--side-by-side", "--width", "40"]);
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│    │              │ 1  │a = 1", strip_ansi_codes(line_1));
        assert_eq!(
            "│    │              │ 2  │b = 234567",
            strip_ansi_codes(line_2)
        );
    }

    #[test]
    fn test_two_plus_lines_truncated() {
        let mut config = make_config_from_args(&["--side-by-side", "--width", "30"]);
        config.truncation_symbol = ">".into();
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│    │         │ 1  │a = 1    ", strip_ansi_codes(line_1));
        assert_eq!("│    │         │ 2  │b = 2345>", strip_ansi_codes(line_2));
    }

    #[test]
    fn test_two_plus_lines_exact_fit() {
        let mut config = make_config_from_args(&["--side-by-side", "--width", "32"]);
        config.truncation_symbol = ">".into();
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│    │          │ 1  │a = 1", strip_ansi_codes(line_1));
        assert_eq!("│    │          │ 2  │b = 234567", strip_ansi_codes(line_2));
    }

    #[test]
    fn test_one_minus_one_plus_line() {
        let config = make_config_from_args(&["--side-by-side", "--width", "40"]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(7);
        assert_eq!("│ 1  │a = 1         │ 1  │a = 1", lines.next().unwrap());
        assert_eq!("│ 2  │b = 2         │ 2  │bb = 2", lines.next().unwrap());
    }
}
