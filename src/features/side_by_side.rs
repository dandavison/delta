use itertools::Itertools;
use syntect::highlighting::Style as SyntectStyle;

use crate::ansi;
use crate::cli;
use crate::config::Config;
use crate::delta::State;
use crate::features::line_numbers;
use crate::features::OptionValueFunction;
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

pub enum PanelSide {
    Left,
    Right,
}

pub struct SideBySideData {
    pub left_panel: Panel,
    pub right_panel: Panel,
}

pub struct Panel {
    pub width: usize,
    pub offset: usize,
}

impl SideBySideData {
    pub fn new(decorations_width: &cli::Width, available_terminal_width: &usize) -> Self {
        let panel_width = match decorations_width {
            cli::Width::Fixed(w) => w / 2,
            _ => available_terminal_width / 2,
        };
        Self {
            left_panel: Panel {
                width: panel_width,
                offset: 0,
            },
            right_panel: Panel {
                width: panel_width,
                offset: 0,
            },
        }
    }
}

/// Emit a sequence of minus and plus lines in side-by-side mode.
#[allow(clippy::too_many_arguments)]
pub fn paint_minus_and_plus_lines_side_by_side<'a>(
    minus_syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
    minus_diff_style_sections: Vec<Vec<(Style, &str)>>,
    minus_states: Vec<&'a State>,
    plus_syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
    plus_diff_style_sections: Vec<Vec<(Style, &str)>>,
    plus_states: Vec<&'a State>,
    line_alignment: Vec<(Option<usize>, Option<usize>)>,
    output_buffer: &mut String,
    config: &Config,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    background_color_extends_to_terminal_width: Option<bool>,
) {
    for (minus_line_index, plus_line_index) in line_alignment {
        output_buffer.push_str(&paint_left_panel_minus_line(
            minus_line_index,
            &minus_syntax_style_sections,
            &minus_diff_style_sections,
            match minus_line_index {
                Some(i) => minus_states[i],
                None => &State::HunkMinus(None),
            },
            line_numbers_data,
            if config.keep_plus_minus_markers {
                Some(config.minus_style.paint("-"))
            } else {
                None
            },
            background_color_extends_to_terminal_width,
            config,
        ));
        output_buffer.push_str(&paint_right_panel_plus_line(
            plus_line_index,
            &plus_syntax_style_sections,
            &plus_diff_style_sections,
            match plus_line_index {
                Some(i) => plus_states[i],
                None => &State::HunkPlus(None),
            },
            line_numbers_data,
            if config.keep_plus_minus_markers {
                Some(config.plus_style.paint("+"))
            } else {
                None
            },
            background_color_extends_to_terminal_width,
            config,
        ));
        output_buffer.push('\n');
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_zero_lines_side_by_side(
    syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
    diff_style_sections: Vec<Vec<(Style, &str)>>,
    state: &State,
    output_buffer: &mut String,
    config: &Config,
    line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
    painted_prefix: Option<ansi_term::ANSIString>,
    background_color_extends_to_terminal_width: Option<bool>,
) {
    for (line_index, (syntax_sections, diff_sections)) in syntax_style_sections
        .iter()
        .zip_eq(diff_style_sections.iter())
        .enumerate()
    {
        let (mut left_panel_line, left_panel_line_is_empty) = Painter::paint_line(
            syntax_sections,
            diff_sections,
            state,
            line_numbers_data,
            Some(PanelSide::Left),
            painted_prefix.clone(),
            config,
        );
        // TODO: Avoid doing the superimpose_style_sections work twice.
        // HACK: These are getting incremented twice, so knock them back down once.
        if let Some(d) = line_numbers_data.as_mut() {
            d.hunk_minus_line_number -= 1;
            d.hunk_plus_line_number -= 1;
        }
        right_pad_left_panel_line(
            &mut left_panel_line,
            left_panel_line_is_empty,
            Some(line_index),
            &diff_style_sections,
            &State::HunkZero,
            background_color_extends_to_terminal_width,
            config,
        );
        output_buffer.push_str(&left_panel_line);

        let (mut right_panel_line, right_panel_line_is_empty) = Painter::paint_line(
            syntax_sections,
            diff_sections,
            state,
            line_numbers_data,
            Some(PanelSide::Right),
            painted_prefix.clone(),
            config,
        );
        right_fill_right_panel_line(
            &mut right_panel_line,
            right_panel_line_is_empty,
            Some(line_index),
            &diff_style_sections,
            &State::HunkZero,
            background_color_extends_to_terminal_width,
            config,
        );
        output_buffer.push_str(&right_panel_line);
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
    background_color_extends_to_terminal_width: Option<bool>,
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
    right_pad_left_panel_line(
        &mut panel_line,
        panel_line_is_empty,
        line_index,
        diff_style_sections,
        state,
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
    background_color_extends_to_terminal_width: Option<bool>,
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
    right_fill_right_panel_line(
        &mut panel_line,
        panel_line_is_empty,
        line_index,
        diff_style_sections,
        state,
        background_color_extends_to_terminal_width,
        config,
    );
    panel_line
}

fn get_right_fill_style_for_left_panel(
    line_is_empty: bool,
    line_index: Option<usize>,
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    background_color_extends_to_terminal_width: Option<bool>,
    config: &Config,
) -> Style {
    match (line_is_empty, line_index) {
        (true, _) => config.null_style,
        (false, None) => config.null_style,
        (false, Some(index)) => {
            let (should_fill, fill_style) =
                Painter::get_should_right_fill_background_color_and_fill_style(
                    &diff_style_sections[index],
                    state,
                    background_color_extends_to_terminal_width,
                    config,
                );
            if should_fill {
                fill_style
            } else {
                config.null_style
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
                d.hunk_minus_line_number -= 1;
            }
        }
        (State::HunkMinus(_), State::HunkPlus(_)) => {
            if let Some(d) = line_numbers_data.as_mut() {
                d.hunk_plus_line_number -= 1;
            }
        }
        _ => unreachable!(),
    }
    (line, line_is_empty)
}

/// Right-pad a line in the left panel with (possibly painted) spaces. A line in the left panel is
/// either a minus line or a zero line.
#[allow(clippy::comparison_chain)]
fn right_pad_left_panel_line(
    panel_line: &mut String,
    panel_line_is_empty: bool,
    line_index: Option<usize>,
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    background_color_extends_to_terminal_width: Option<bool>,
    config: &Config,
) {
    // The left panel uses spaces to pad to the midpoint. This differs from the right panel,
    // and from the non-side-by-side implementation.

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
            State::HunkZero => {}
            _ => unreachable!(),
        };
    };
    // Pad with (maybe painted) spaces to the panel width.
    let text_width = ansi::measure_text_width(&panel_line);
    let panel_width = config.side_by_side_data.left_panel.width;
    if text_width < panel_width {
        let fill_style = get_right_fill_style_for_left_panel(
            panel_line_is_empty,
            line_index,
            &diff_style_sections,
            state,
            background_color_extends_to_terminal_width,
            config,
        );
        panel_line.push_str(
            &fill_style
                .paint(" ".repeat(panel_width - text_width))
                .to_string(),
        );
    } else if text_width > panel_width {
        *panel_line =
            ansi::truncate_str(panel_line, panel_width, &config.truncation_symbol).to_string();
    }
}

/// Right-fill the background color of a line in the right panel. A line in the right panel is
/// either a zero line or a plus line. The fill is achieved using ANSI sequences instructing the
/// terminal emulator to fill the background color rightwards; it does not involve appending spaces
/// to the line.
fn right_fill_right_panel_line(
    panel_line: &mut String,
    panel_line_is_empty: bool,
    line_index: Option<usize>,
    diff_style_sections: &[Vec<(Style, &str)>],
    state: &State,
    background_color_extends_to_terminal_width: Option<bool>,
    config: &Config,
) {
    *panel_line = ansi::truncate_str(
        &panel_line,
        config.side_by_side_data.right_panel.width,
        &config.truncation_symbol,
    )
    .to_string();

    // Unlike `right_pad_left_panel_line`, the line-end emissions here are basically the same as
    // the non side-by-side implementation in Painter::paint_lines.
    let (should_right_fill_background_color, fill_style) = if let Some(index) = line_index {
        Painter::get_should_right_fill_background_color_and_fill_style(
            &diff_style_sections[index],
            state,
            background_color_extends_to_terminal_width,
            config,
        )
    } else {
        (false, config.null_style)
    };

    if should_right_fill_background_color {
        Painter::right_fill_background_color(panel_line, fill_style);
    } else if panel_line_is_empty && line_index.is_some() {
        // Emit empty line marker when the panel line is empty but not empty-by-construction. See
        // parallel comment in `paint_left_panel_minus_line`.
        match state {
            State::HunkPlus(_) => Painter::mark_empty_line(
                &config.plus_empty_line_marker_style,
                panel_line,
                Some(" "),
            ),
            State::HunkZero => {}
            _ => unreachable!(),
        }
    };
}

#[cfg(test)]
pub mod tests {
    use crate::ansi::strip_ansi_codes;
    use crate::features::line_numbers::tests::*;
    use crate::tests::integration_test_utils::{make_config_from_args, run_delta};

    #[test]
    fn test_two_minus_lines() {
        let config = make_config_from_args(&["--side-by-side", "--width", "40"]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│ 1  │a = 1         │    │", strip_ansi_codes(line_1));
        assert_eq!("│ 2  │b = 2         │    │", strip_ansi_codes(line_2));
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config_from_args(&["--side-by-side", "--width", "40"]);
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(7);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!("│    │              │ 1  │a = 1", strip_ansi_codes(line_1));
        assert_eq!("│    │              │ 2  │b = 2", strip_ansi_codes(line_2));
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
