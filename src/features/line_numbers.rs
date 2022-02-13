use std::cmp::max;

use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::delta::State;
use crate::features::hyperlinks;
use crate::features::side_by_side::ansifill::{self, ODD_PAD_CHAR};
use crate::features::side_by_side::{Left, PanelSide, Right};
use crate::features::OptionValueFunction;
use crate::format::{self, Align, Placeholder};
use crate::minusplus::*;
use crate::style::Style;
use crate::utils;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "line-numbers",
            bool,
            None,
            _opt => true
        ),
        (
            "line-numbers-left-style",
            String,
            None,
            _opt => "blue"
        ),
        (
            "line-numbers-right-style",
            String,
            None,
            _opt => "blue"
        ),
        (
            "line-numbers-minus-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {
                "red".to_string()
            } else {
                "88".to_string()
            }
        ),
        (
            "line-numbers-zero-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {"#dddddd"} else {"#444444"}
        ),
        (
            "line-numbers-plus-style",
            String,
            None,
            opt => if opt.computed.is_light_mode {
                "green".to_string()
            } else {
                "28".to_string()
            }
        )
    ])
}

pub fn linenumbers_and_styles<'a>(
    line_numbers_data: &'a mut LineNumbersData,
    state: &State,
    config: &'a config::Config,
    increment: bool,
) -> Option<(MinusPlus<Option<usize>>, MinusPlus<Style>)> {
    let nr_left = line_numbers_data.line_number[Left];
    let nr_right = line_numbers_data.line_number[Right];
    let (minus_style, zero_style, plus_style) = (
        config.line_numbers_style_minusplus[Minus],
        config.line_numbers_zero_style,
        config.line_numbers_style_minusplus[Plus],
    );
    let ((minus_number, plus_number), (minus_style, plus_style)) = match state {
        State::HunkMinus(_, _) => {
            line_numbers_data.line_number[Left] += increment as usize;
            ((Some(nr_left), None), (minus_style, plus_style))
        }
        State::HunkMinusWrapped => ((None, None), (minus_style, plus_style)),
        State::HunkZero(_, _) => {
            line_numbers_data.line_number[Left] += increment as usize;
            line_numbers_data.line_number[Right] += increment as usize;
            ((Some(nr_left), Some(nr_right)), (zero_style, zero_style))
        }
        State::HunkZeroWrapped => ((None, None), (zero_style, zero_style)),
        State::HunkPlus(_, _) => {
            line_numbers_data.line_number[Right] += increment as usize;
            ((None, Some(nr_right)), (minus_style, plus_style))
        }
        State::HunkPlusWrapped => ((None, None), (minus_style, plus_style)),
        _ => return None,
    };
    Some((
        MinusPlus::new(minus_number, plus_number),
        MinusPlus::new(minus_style, plus_style),
    ))
}

/// Return a vec of `ansi_term::ANSIGenericString`s representing the left and right fields of the
/// two-column line number display.
pub fn format_and_paint_line_numbers<'a>(
    line_numbers_data: &'a LineNumbersData,
    side_by_side_panel: Option<PanelSide>,
    styles: MinusPlus<Style>,
    line_numbers: MinusPlus<Option<usize>>,
    config: &'a config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let mut formatted_numbers = Vec::new();

    let (emit_left, emit_right) = match (config.side_by_side, side_by_side_panel) {
        (false, _) => (true, true),
        (true, Some(Left)) => (true, false),
        (true, Some(Right)) => (false, true),
        (true, None) => unreachable!(),
    };

    if emit_left {
        formatted_numbers.extend(format_and_paint_line_number_field(
            line_numbers_data,
            Minus,
            &styles,
            &line_numbers,
            config,
        ));
    }

    if emit_right {
        formatted_numbers.extend(format_and_paint_line_number_field(
            line_numbers_data,
            Plus,
            &styles,
            &line_numbers,
            config,
        ));
    }
    formatted_numbers
}

lazy_static! {
    static ref LINE_NUMBERS_PLACEHOLDER_REGEX: Regex = format::make_placeholder_regex(&["nm", "np"]);
}

#[derive(Default, Debug)]
pub struct LineNumbersData<'a> {
    pub format_data: MinusPlus<format::FormatStringData<'a>>,
    pub line_number: MinusPlus<usize>,
    pub hunk_max_line_number_width: usize,
    pub plus_file: String,
}

pub type SideBySideLineWidth = MinusPlus<usize>;

// Although it's probably unusual, a single format string can contain multiple placeholders. E.g.
// line-numbers-right-format = "{nm} {np}|"
impl<'a> LineNumbersData<'a> {
    pub fn from_format_strings(
        format: &'a MinusPlus<String>,
        use_full_width: ansifill::UseFullPanelWidth,
    ) -> LineNumbersData<'a> {
        let insert_center_space_on_odd_width = use_full_width.pad_width();
        Self {
            format_data: MinusPlus::new(
                format::parse_line_number_format(
                    &format[Left],
                    &*LINE_NUMBERS_PLACEHOLDER_REGEX,
                    false,
                ),
                format::parse_line_number_format(
                    &format[Right],
                    &*LINE_NUMBERS_PLACEHOLDER_REGEX,
                    insert_center_space_on_odd_width,
                ),
            ),
            ..Self::default()
        }
    }

    /// Initialize line number data for a hunk.
    pub fn initialize_hunk(&mut self, line_numbers: &[(usize, usize)], plus_file: String) {
        // Typically, line_numbers has length 2: an entry for the minus file, and one for the plus
        // file. In the case of merge commits, it may be longer.
        self.line_number =
            MinusPlus::new(line_numbers[0].0, line_numbers[line_numbers.len() - 1].0);
        let hunk_max_line_number = line_numbers.iter().map(|(n, d)| n + d).max().unwrap();
        self.hunk_max_line_number_width =
            1 + (hunk_max_line_number as f64).log10().floor() as usize;
        self.plus_file = plus_file;
    }

    pub fn empty_for_sbs(use_full_width: ansifill::UseFullPanelWidth) -> LineNumbersData<'a> {
        let insert_center_space_on_odd_width = use_full_width.pad_width();
        Self {
            format_data: if insert_center_space_on_odd_width {
                let format_left = vec![format::FormatStringPlaceholderData::default()];
                let format_right = vec![format::FormatStringPlaceholderData {
                    prefix: format!("{}", ODD_PAD_CHAR).into(),
                    prefix_len: 1,
                    ..Default::default()
                }];
                MinusPlus::new(format_left, format_right)
            } else {
                MinusPlus::default()
            },
            ..Self::default()
        }
    }

    pub fn formatted_width(&self) -> SideBySideLineWidth {
        let format_data_width = |format_data: &format::FormatStringData<'a>| {
            // Provide each Placeholder with the max_line_number_width to calculate the
            // actual width. Only use prefix and suffix of the last element, otherwise
            // only the prefix (as the suffix also contains the following prefix).
            format_data
                .last()
                .map(|last| {
                    let (prefix_width, suffix_width) = last.width(self.hunk_max_line_number_width);
                    format_data
                        .iter()
                        .rev()
                        .skip(1)
                        .map(|p| p.width(self.hunk_max_line_number_width).0)
                        .sum::<usize>()
                        + prefix_width
                        + suffix_width
                })
                .unwrap_or(0)
        };
        MinusPlus::new(
            format_data_width(&self.format_data[Left]),
            format_data_width(&self.format_data[Right]),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn format_and_paint_line_number_field<'a>(
    line_numbers_data: &'a LineNumbersData,
    side: MinusPlusIndex,
    styles: &MinusPlus<Style>,
    line_numbers: &MinusPlus<Option<usize>>,
    config: &config::Config,
) -> Vec<ansi_term::ANSIGenericString<'a, str>> {
    let min_field_width = line_numbers_data.hunk_max_line_number_width;

    let format_data = &line_numbers_data.format_data[side];
    let plus_file = &line_numbers_data.plus_file;
    let style = &config.line_numbers_style_leftright[side];

    let mut ansi_strings = Vec::new();
    let mut suffix = "";
    for placeholder in format_data {
        ansi_strings.push(style.paint(placeholder.prefix.as_str()));

        let width = if let Some(placeholder_width) = placeholder.width {
            max(placeholder_width, min_field_width)
        } else {
            min_field_width
        };

        let alignment_spec = placeholder.alignment_spec.unwrap_or(Align::Center);
        match placeholder.placeholder {
            Some(Placeholder::NumberMinus) => {
                ansi_strings.push(styles[Minus].paint(format_line_number(
                    line_numbers[Minus],
                    alignment_spec,
                    width,
                    placeholder.precision,
                    None,
                    config,
                )))
            }
            Some(Placeholder::NumberPlus) => {
                ansi_strings.push(styles[Plus].paint(format_line_number(
                    line_numbers[Plus],
                    alignment_spec,
                    width,
                    placeholder.precision,
                    Some(plus_file),
                    config,
                )))
            }
            None => {}
            _ => unreachable!("Invalid placeholder"),
        }
        suffix = placeholder.suffix.as_str();
    }
    ansi_strings.push(style.paint(suffix));
    ansi_strings
}

/// Return line number formatted according to `alignment` and `width`.
fn format_line_number(
    line_number: Option<usize>,
    alignment: Align,
    width: usize,
    precision: Option<usize>,
    plus_file: Option<&str>,
    config: &config::Config,
) -> String {
    let pad = |n| format::pad(n, width, alignment, precision);
    match (line_number, config.hyperlinks, plus_file) {
        (None, _, _) => " ".repeat(width),
        (Some(n), true, Some(file)) => match utils::path::absolute_path(file, config) {
            Some(absolute_path) => {
                hyperlinks::format_osc8_file_hyperlink(absolute_path, line_number, &pad(n), config)
                    .to_string()
            }
            None => file.to_owned(),
        },
        (Some(n), _, _) => pad(n),
    }
}

#[cfg(test)]
pub mod tests {
    use regex::Captures;

    use crate::ansi::strip_ansi_codes;
    use crate::features::side_by_side::ansifill::ODD_PAD_CHAR;
    use crate::format::FormatStringData;
    use crate::tests::integration_test_utils::{make_config_from_args, run_delta, DeltaTest};

    use super::*;

    pub fn parse_line_number_format_with_default_regex<'a>(
        format_string: &'a str,
    ) -> FormatStringData<'a> {
        format::parse_line_number_format(format_string, &LINE_NUMBERS_PLACEHOLDER_REGEX, false)
    }

    #[test]
    fn test_line_number_format_regex_1() {
        assert_eq!(
            parse_line_number_format_with_default_regex("{nm}"),
            vec![format::FormatStringPlaceholderData {
                prefix: "".into(),
                placeholder: Some(Placeholder::NumberMinus),
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_2() {
        assert_eq!(
            parse_line_number_format_with_default_regex("{np:4}"),
            vec![format::FormatStringPlaceholderData {
                prefix: "".into(),
                placeholder: Some(Placeholder::NumberPlus),
                alignment_spec: None,
                width: Some(4),
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_3() {
        assert_eq!(
            parse_line_number_format_with_default_regex("{np:>4}"),
            vec![format::FormatStringPlaceholderData {
                prefix: "".into(),
                placeholder: Some(Placeholder::NumberPlus),
                alignment_spec: Some(Align::Right),
                width: Some(4),
                precision: None,
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_4() {
        assert_eq!(
            parse_line_number_format_with_default_regex("{np:_>4}"),
            vec![format::FormatStringPlaceholderData {
                prefix: "".into(),
                placeholder: Some(Placeholder::NumberPlus),
                alignment_spec: Some(Align::Right),
                width: Some(4),
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_5() {
        assert_eq!(
            parse_line_number_format_with_default_regex("__{np:_>4}@@"),
            vec![format::FormatStringPlaceholderData {
                prefix: "__".into(),
                placeholder: Some(Placeholder::NumberPlus),
                alignment_spec: Some(Align::Right),
                width: Some(4),
                precision: None,
                suffix: "@@".into(),
                prefix_len: 2,
                suffix_len: 2,
                ..Default::default()
            }]
        )
    }

    #[test]
    fn test_line_number_format_regex_6() {
        assert_eq!(
            parse_line_number_format_with_default_regex("__{nm:<3}@@---{np:_>4}**"),
            vec![
                format::FormatStringPlaceholderData {
                    prefix: "__".into(),
                    placeholder: Some(Placeholder::NumberMinus),
                    alignment_spec: Some(Align::Left),
                    width: Some(3),
                    precision: None,
                    suffix: "@@---{np:_>4}**".into(),
                    prefix_len: 2,
                    suffix_len: 15,
                    ..Default::default()
                },
                format::FormatStringPlaceholderData {
                    prefix: "@@---".into(),
                    placeholder: Some(Placeholder::NumberPlus),
                    alignment_spec: Some(Align::Right),
                    width: Some(4),
                    precision: None,
                    suffix: "**".into(),
                    prefix_len: 5,
                    suffix_len: 2,
                    ..Default::default()
                }
            ]
        )
    }

    #[test]
    fn test_line_number_format_regex_7() {
        assert_eq!(
            parse_line_number_format_with_default_regex("__@@---**",),
            vec![format::FormatStringPlaceholderData {
                prefix: "".into(),
                placeholder: None,
                alignment_spec: None,
                width: None,
                precision: None,
                suffix: "__@@---**".into(),
                prefix_len: 0,
                suffix_len: 9,
                ..Default::default()
            },]
        )
    }

    #[test]
    fn test_line_number_format_odd_width_one() {
        assert_eq!(
            format::parse_line_number_format("|{nm:<4}|", &LINE_NUMBERS_PLACEHOLDER_REGEX, true),
            vec![format::FormatStringPlaceholderData {
                prefix: format!("{}|", ODD_PAD_CHAR).into(),
                placeholder: Some(Placeholder::NumberMinus),
                alignment_spec: Some(Align::Left),
                width: Some(4),
                precision: None,
                suffix: "|".into(),
                prefix_len: 2,
                suffix_len: 1,
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_line_number_format_odd_width_two() {
        assert_eq!(
            format::parse_line_number_format(
                "|{nm:<4}+{np:<4}|",
                &LINE_NUMBERS_PLACEHOLDER_REGEX,
                true
            ),
            vec![
                format::FormatStringPlaceholderData {
                    prefix: format!("{}|", ODD_PAD_CHAR).into(),
                    placeholder: Some(Placeholder::NumberMinus),
                    alignment_spec: Some(Align::Left),
                    width: Some(4),
                    precision: None,
                    suffix: "+{np:<4}|".into(),
                    prefix_len: 2,
                    suffix_len: 9,
                    ..Default::default()
                },
                format::FormatStringPlaceholderData {
                    prefix: "+".into(),
                    placeholder: Some(Placeholder::NumberPlus),
                    alignment_spec: Some(Align::Left),
                    width: Some(4),
                    precision: None,
                    suffix: "|".into(),
                    prefix_len: 1,
                    suffix_len: 1,
                    ..Default::default()
                }
            ]
        );
    }
    #[test]
    fn test_line_number_format_odd_width_none() {
        assert_eq!(
            format::parse_line_number_format("|++|", &LINE_NUMBERS_PLACEHOLDER_REGEX, true),
            vec![format::FormatStringPlaceholderData {
                prefix: format!("{}", ODD_PAD_CHAR).into(),
                placeholder: None,
                alignment_spec: None,
                width: None,
                precision: None,
                suffix: "|++|".into(),
                prefix_len: 1,
                suffix_len: 4,
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_line_number_format_long() {
        let long = "line number format which is too large for SSO";
        assert!(long.len() > std::mem::size_of::<smol_str::SmolStr>());
        assert_eq!(
            parse_line_number_format_with_default_regex(&format!(
                "{long}{{nm}}{long}",
                long = long
            ),),
            vec![format::FormatStringPlaceholderData {
                prefix: long.into(),
                prefix_len: long.len(),
                placeholder: Some(Placeholder::NumberMinus),
                alignment_spec: None,
                width: None,
                precision: None,
                suffix: long.into(),
                suffix_len: long.len(),
                ..Default::default()
            },]
        )
    }

    #[test]
    fn test_line_number_placeholder_width_one() {
        let data = parse_line_number_format_with_default_regex("");
        assert_eq!(data[0].width(0), (0, 0));

        let data = parse_line_number_format_with_default_regex("");
        assert_eq!(data[0].width(4), (0, 0));

        let data = parse_line_number_format_with_default_regex("│+│");
        assert_eq!(data[0].width(4), (0, 3));

        let data = parse_line_number_format_with_default_regex("{np}");
        assert_eq!(data[0].width(4), (4, 0));

        let data = parse_line_number_format_with_default_regex("│{np}│");
        assert_eq!(data[0].width(4), (5, 1));

        let data = parse_line_number_format_with_default_regex("│{np:2}│");
        assert_eq!(data[0].width(4), (5, 1));

        let data = parse_line_number_format_with_default_regex("│{np:6}│");
        assert_eq!(data[0].width(4), (7, 1));
    }

    #[test]
    fn test_line_number_placeholder_width_two() {
        let data = parse_line_number_format_with_default_regex("│{nm}│{np}│");
        assert_eq!(data[0].width(1), (2, 6));
        assert_eq!(data[1].width(1), (2, 1));

        let data = parse_line_number_format_with_default_regex("│{nm:_>5}│{np:1}│");
        assert_eq!(data[0].width(1), (6, 8));
        assert_eq!(data[1].width(1), (2, 1));

        let data = parse_line_number_format_with_default_regex("│{nm}│{np:5}│");
        assert_eq!(data[0].width(7), (8, 8));
        assert_eq!(data[1].width(7), (8, 1));
    }

    #[test]
    fn test_line_numbers_data() {
        use crate::features::side_by_side::ansifill;
        let w = ansifill::UseFullPanelWidth(false);
        let format = MinusPlus::new("".into(), "".into());
        let mut data = LineNumbersData::from_format_strings(&format, w.clone());
        data.initialize_hunk(&[(10, 11), (10000, 100001)], "a".into());
        assert_eq!(data.formatted_width(), MinusPlus::new(0, 0));

        let format = MinusPlus::new("│".into(), "│+│".into());
        let mut data = LineNumbersData::from_format_strings(&format, w.clone());

        data.initialize_hunk(&[(10, 11), (10000, 100001)], "a".into());
        assert_eq!(data.formatted_width(), MinusPlus::new(1, 3));

        let format = MinusPlus::new("│{nm:^3}│".into(), "│{np:^3}│".into());
        let mut data = LineNumbersData::from_format_strings(&format, w.clone());

        data.initialize_hunk(&[(10, 11), (10000, 100001)], "a".into());
        assert_eq!(data.formatted_width(), MinusPlus::new(8, 8));

        let format = MinusPlus::new("│{nm:^3}│ │{np:<12}│ │{nm}│".into(), "".into());
        let mut data = LineNumbersData::from_format_strings(&format, w.clone());

        data.initialize_hunk(&[(10, 11), (10000, 100001)], "a".into());
        assert_eq!(data.formatted_width(), MinusPlus::new(32, 0));

        let format = MinusPlus::new("│{np:^3}│ │{nm:<12}│ │{np}│".into(), "".into());
        let mut data = LineNumbersData::from_format_strings(&format, w.clone());

        data.initialize_hunk(&[(10, 11), (10000, 100001)], "a".into());
        assert_eq!(data.formatted_width(), MinusPlus::new(32, 0));
    }

    fn _get_capture<'a>(i: usize, j: usize, caps: &'a Vec<Captures>) -> &'a str {
        caps[i].get(j).map_or("", |m| m.as_str())
    }

    #[test]
    fn test_two_minus_lines() {
        DeltaTest::with_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ])
        .with_input(TWO_MINUS_LINES_DIFF)
        .expect_after_header(
            r#"
             #indent_mark
               1 ⋮    │a = 1
               2 ⋮    │b = 23456"#,
        );
    }

    #[test]
    fn test_two_plus_lines() {
        DeltaTest::with_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ])
        .with_input(TWO_PLUS_LINES_DIFF)
        .expect_after_header(
            r#"
             #indent_mark
                 ⋮  1 │a = 1
                 ⋮  2 │b = 234567"#,
        );
    }

    #[test]
    fn test_one_minus_one_plus_line() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        assert_eq!(lines.next().unwrap(), "  1 ⋮  1 │a = 1");
        assert_eq!(lines.next().unwrap(), "  2 ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮  2 │bb = 2");
    }

    #[test]
    fn test_repeated_placeholder() {
        let config = make_config_from_args(&[
            "--line-numbers",
            "--line-numbers-left-format",
            "{nm:^4} {nm:^4}⋮",
            "--line-numbers-right-format",
            "{np:^4}│",
            "--line-numbers-left-style",
            "0 1",
            "--line-numbers-minus-style",
            "0 2",
            "--line-numbers-right-style",
            "0 3",
            "--line-numbers-plus-style",
            "0 4",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        assert_eq!(lines.next().unwrap(), "  1    1 ⋮  1 │a = 1");
        assert_eq!(lines.next().unwrap(), "  2    2 ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "         ⋮  2 │bb = 2");
    }

    #[test]
    fn test_five_digit_line_number() {
        let config = make_config_from_args(&["--line-numbers"]);
        let output = run_delta(FIVE_DIGIT_LINE_NUMBER_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        assert_eq!(lines.next().unwrap(), "10000⋮10000│a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10001│bb = 2");
    }

    #[test]
    fn test_unequal_digit_line_number() {
        let config = make_config_from_args(&["--line-numbers"]);
        let output = run_delta(UNEQUAL_DIGIT_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(crate::config::HEADER_LEN);
        assert_eq!(lines.next().unwrap(), "10000⋮ 9999│a = 1");
        assert_eq!(lines.next().unwrap(), "10001⋮     │b = 2");
        assert_eq!(lines.next().unwrap(), "     ⋮10000│bb = 2");
    }

    #[test]
    fn test_color_only() {
        let config = make_config_from_args(&["--line-numbers", "--color-only"]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let mut lines = output.lines().skip(5);
        let (line_1, line_2) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(strip_ansi_codes(line_1), "  1 ⋮    │-a = 1");
        assert_eq!(strip_ansi_codes(line_2), "  2 ⋮    │-b = 23456");
    }

    #[test]
    fn test_hunk_header_style_is_omit() {
        let config = make_config_from_args(&["--line-numbers", "--hunk-header-style", "omit"]);
        let output = run_delta(TWO_LINE_DIFFS, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), "  1 ⋮  1 │a = 1");
        assert_eq!(lines.next().unwrap(), "  2 ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮  2 │bb = 2");
        assert_eq!(lines.next().unwrap(), "");
        assert_eq!(lines.next().unwrap(), " 499⋮ 499│a = 3");
        assert_eq!(lines.next().unwrap(), " 500⋮    │b = 4");
        assert_eq!(lines.next().unwrap(), "    ⋮ 500│bb = 4");
    }

    #[test]
    fn test_line_numbers_continue_correctly() {
        DeltaTest::with_args(&["--side-by-side", "--width", "44", "--line-fill-method=ansi"])
            .with_input(DIFF_PLUS_MINUS_WITH_1_CONTEXT_DIFF)
            .expect_after_header(
                r#"
                │  1 │abc             │  1 │abc
                │  2 │a = left side   │  2 │a = right side
                │  3 │xyz             │  3 │xyz"#,
            );
    }

    #[test]
    fn test_line_numbers_continue_correctly_after_wrapping() {
        DeltaTest::with_args(&[
            "--side-by-side",
            "--width",
            "32",
            "--line-fill-method=ansi",
            "--wrap-left-symbol",
            "@",
            "--wrap-right-symbol",
            "@",
        ])
        .with_input(DIFF_PLUS_MINUS_WITH_1_CONTEXT_DIFF)
        .expect_after_header(
            r#"
            │  1 │abc       │  1 │abc
            │  2 │a = left @│  2 │a = right@
            │    │side      │    │ side
            │  3 │xyz       │  3 │xyz"#,
        );

        let cfg = &[
            "--side-by-side",
            "--width",
            "42",
            "--line-fill-method=ansi",
            "--wrap-left-symbol",
            "@",
            "--wrap-right-symbol",
            "@",
        ];

        DeltaTest::with_args(cfg)
            .with_input(DIFF_WITH_LONGER_MINUS_1_CONTEXT)
            .expect_after_header(
                r#"
                │  1 │abc            │  1 │abc
                │  2 │a = one side   │  2 │a = one longer@
                │    │               │    │ side
                │  3 │xyz            │  3 │xyz"#,
            );

        DeltaTest::with_args(cfg)
            .with_input(DIFF_WITH_LONGER_PLUS_1_CONTEXT)
            .expect_after_header(
                r#"
                │  1 │abc            │  1 │abc
                │  2 │a = one longer@│  2 │a = one side
                │    │ side          │    │
                │  3 │xyz            │  3 │xyz"#,
            );

        DeltaTest::with_args(cfg)
            .with_input(DIFF_MISMATCH_LONGER_MINUS_1_CONTEXT)
            .expect_after_header(
                r#"
                │  1 │abc            │  1 │abc
                │  2 │a = left side @│    │
                │    │which is longer│    │
                │    │               │  2 │a = other one
                │  3 │xyz            │  3 │xyz"#,
            );

        DeltaTest::with_args(cfg)
            .with_input(DIFF_MISMATCH_LONGER_PLUS_1_CONTEXT)
            .expect_after_header(
                r#"
                │  1 │abc            │  1 │abc
                │  2 │a = other one  │    │
                │    │               │  2 │a = right side@
                │    │               │    │ which is long@
                │    │               │    │er
                │  3 │xyz            │  3 │xyz"#,
            );
    }

    pub const TWO_MINUS_LINES_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +0,0 @@
-a = 1
-b = 23456
";

    pub const TWO_PLUS_LINES_DIFF: &str = "\
diff --git c/a.py i/a.py
new file mode 100644
index 0000000..223ca50
--- /dev/null
+++ i/a.py
@@ -0,0 +1,2 @@
+a = 1
+b = 234567
";

    pub const ONE_MINUS_ONE_PLUS_LINE_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
";

    const TWO_LINE_DIFFS: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
@@ -499,2 +499,2 @@
 a = 3
-b = 4
+bb = 4
";

    const FIVE_DIGIT_LINE_NUMBER_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -10000,2 +10000,2 @@
 a = 1
-b = 2
+bb = 2
";

    const UNEQUAL_DIGIT_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -10000,2 +9999,2 @@
 a = 1
-b = 2
+bb = 2
";

    const DIFF_PLUS_MINUS_WITH_1_CONTEXT_DIFF: &str = "\
--- a/a.py
+++ b/b.py
@@ -1,3 +1,3 @@
 abc
-a = left side
+a = right side
 xyz";

    const DIFF_WITH_LONGER_MINUS_1_CONTEXT: &str = "\
--- a/a.py
+++ b/b.py
@@ -1,3 +1,3 @@
 abc
-a = one side
+a = one longer side
 xyz";

    const DIFF_WITH_LONGER_PLUS_1_CONTEXT: &str = "\
--- a/a.py
+++ b/b.py
@@ -1,3 +1,3 @@
 abc
-a = one longer side
+a = one side
 xyz";

    const DIFF_MISMATCH_LONGER_MINUS_1_CONTEXT: &str = "\
--- a/a.py
+++ b/b.py
@@ -1,3 +1,3 @@
 abc
-a = left side which is longer
+a = other one
 xyz";

    const DIFF_MISMATCH_LONGER_PLUS_1_CONTEXT: &str = "\
--- a/a.py
+++ b/b.py
@@ -1,3 +1,3 @@
 abc
-a = other one
+a = right side which is longer
 xyz";
}
