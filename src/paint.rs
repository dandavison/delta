use lazy_static::lazy_static;
use regex::Regex;
use std::io::Write;

use ansi_term;
use syntect::easy::HighlightLines;
use syntect::highlighting::Style as SyntectStyle;
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::config;
use crate::delta::State;
use crate::edits;
use crate::features::line_numbers;
use crate::paint::superimpose_style_sections::superimpose_style_sections;
use crate::style::Style;

pub const ANSI_CSI_CLEAR_TO_EOL: &str = "\x1b[0K";
pub const ANSI_CSI_CLEAR_TO_BOL: &str = "\x1b[1K";
pub const ANSI_SGR_RESET: &str = "\x1b[0m";

pub struct Painter<'a> {
    pub minus_lines: Vec<String>,
    pub plus_lines: Vec<String>,
    pub writer: &'a mut dyn Write,
    pub syntax: &'a SyntaxReference,
    pub highlighter: HighlightLines<'a>,
    pub config: &'a config::Config,
    pub output_buffer: String,
    pub minus_line_number: usize,
    pub plus_line_number: usize,
}

impl<'a> Painter<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a config::Config) -> Self {
        let default_syntax = Self::get_syntax(&config.syntax_set, None);
        // TODO: Avoid doing this.
        let dummy_highlighter = HighlightLines::new(default_syntax, &config.syntax_dummy_theme);
        Self {
            minus_lines: Vec::new(),
            plus_lines: Vec::new(),
            output_buffer: String::new(),
            syntax: default_syntax,
            highlighter: dummy_highlighter,
            writer,
            config,
            minus_line_number: 0,
            plus_line_number: 0,
        }
    }

    pub fn set_syntax(&mut self, extension: Option<&str>) {
        self.syntax = Painter::get_syntax(&self.config.syntax_set, extension);
    }

    fn get_syntax(syntax_set: &'a SyntaxSet, extension: Option<&str>) -> &'a SyntaxReference {
        syntax_set
            .find_syntax_by_extension(extension.unwrap_or("txt"))
            .unwrap_or_else(|| Painter::get_syntax(syntax_set, Some("txt")))
    }

    pub fn set_highlighter(&mut self) {
        if let Some(ref syntax_theme) = self.config.syntax_theme {
            self.highlighter = HighlightLines::new(self.syntax, &syntax_theme)
        };
    }

    pub fn paint_buffered_lines(&mut self) {
        let minus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.minus_lines,
            &State::HunkMinus,
            &mut self.highlighter,
            self.config,
        );
        let plus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.plus_lines,
            &State::HunkPlus,
            &mut self.highlighter,
            self.config,
        );
        let (minus_line_diff_style_sections, plus_line_diff_style_sections) =
            Self::get_diff_style_sections(&self.minus_lines, &self.plus_lines, self.config);

        let mut minus_line_numbers = Vec::new();
        let mut plus_line_numbers = Vec::new();
        for _line in &self.minus_lines {
            minus_line_numbers.push(Some((Some(self.minus_line_number), None)));
            self.minus_line_number += 1;
        }
        for _line in &self.plus_lines {
            plus_line_numbers.push(Some((None, Some(self.plus_line_number))));
            self.plus_line_number += 1;
        }
        // TODO: lines and style sections contain identical line text
        if !self.minus_lines.is_empty() {
            Painter::paint_lines(
                minus_line_syntax_style_sections,
                minus_line_diff_style_sections,
                minus_line_numbers,
                &mut self.output_buffer,
                self.config,
                if self.config.keep_plus_minus_markers {
                    "-"
                } else {
                    ""
                },
                self.config.minus_style,
                self.config.minus_non_emph_style,
                Some(self.config.minus_empty_line_marker_style),
                None,
            );
        }
        if !self.plus_lines.is_empty() {
            Painter::paint_lines(
                plus_line_syntax_style_sections,
                plus_line_diff_style_sections,
                plus_line_numbers,
                &mut self.output_buffer,
                self.config,
                if self.config.keep_plus_minus_markers {
                    "+"
                } else {
                    ""
                },
                self.config.plus_style,
                self.config.plus_non_emph_style,
                Some(self.config.plus_empty_line_marker_style),
                None,
            );
        }
        self.minus_lines.clear();
        self.plus_lines.clear();
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    pub fn paint_lines(
        syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
        diff_style_sections: Vec<Vec<(Style, &str)>>,
        line_number_sections: Vec<Option<(Option<usize>, Option<usize>)>>,
        output_buffer: &mut String,
        config: &config::Config,
        prefix: &str,
        style: Style,          // style for right fill if line contains no emph sections
        non_emph_style: Style, // style for right fill if line contains emph sections
        empty_line_style: Option<Style>, // a style with background color to highlight an empty line
        background_color_extends_to_terminal_width: Option<bool>,
    ) {
        // There's some unfortunate hackery going on here for two reasons:
        //
        // 1. The prefix needs to be injected into the output stream. We paint
        //    this with whatever style the line starts with.
        //
        // 2. We must ensure that we fill rightwards with the appropriate
        //    non-emph background color. In that case we don't use the last
        //    style of the line, because this might be emph.

        for ((syntax_sections, diff_sections), line_numbers) in syntax_style_sections
            .iter()
            .zip(diff_style_sections.iter())
            .zip(line_number_sections.iter())
        {
            let non_emph_style = if style_sections_contain_more_than_one_style(diff_sections) {
                non_emph_style // line contains an emph section
            } else {
                style
            };

            let mut handled_prefix = false;
            let mut ansi_strings = Vec::new();
            if config.line_numbers && line_numbers.is_some() {
                ansi_strings.extend(line_numbers::format_and_paint_line_numbers(
                    line_numbers,
                    config,
                ))
            }
            for (section_style, mut text) in superimpose_style_sections(
                syntax_sections,
                diff_sections,
                config.true_color,
                config.null_syntect_style,
            ) {
                if !handled_prefix {
                    if prefix != "" {
                        ansi_strings.push(section_style.paint(prefix));
                    }
                    if text.len() > 0 {
                        text.remove(0);
                    }
                    handled_prefix = true;
                }
                if !text.is_empty() {
                    ansi_strings.push(section_style.paint(text));
                }
            }
            let right_fill_background_color = non_emph_style.has_background_color()
                && background_color_extends_to_terminal_width
                    .unwrap_or(config.background_color_extends_to_terminal_width);
            if right_fill_background_color {
                ansi_strings.push(non_emph_style.paint(""));
            }
            let line = &mut ansi_term::ANSIStrings(&ansi_strings).to_string();
            if right_fill_background_color {
                // HACK: How to properly incorporate the ANSI_CSI_CLEAR_TO_EOL into ansi_strings?
                if line
                    .to_lowercase()
                    .ends_with(&ANSI_SGR_RESET.to_lowercase())
                {
                    line.truncate(line.len() - ANSI_SGR_RESET.len());
                }
                output_buffer.push_str(&line);
                output_buffer.push_str(ANSI_CSI_CLEAR_TO_EOL);
                output_buffer.push_str(ANSI_SGR_RESET);
            } else if line.is_empty() {
                if let Some(empty_line_style) = empty_line_style {
                    output_buffer.push_str(
                        &empty_line_style
                            .ansi_term_style
                            .paint(ANSI_CSI_CLEAR_TO_BOL)
                            .to_string(),
                    );
                }
            } else {
                output_buffer.push_str(&line);
            }
            output_buffer.push_str("\n");
        }
    }

    /// Write output buffer to output stream, and clear the buffer.
    pub fn emit(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.clear();
        Ok(())
    }

    pub fn should_compute_syntax_highlighting(state: &State, config: &config::Config) -> bool {
        if config.syntax_theme.is_none() {
            return false;
        }
        match state {
            State::HunkMinus => {
                config.minus_style.is_syntax_highlighted
                    || config.minus_emph_style.is_syntax_highlighted
            }
            State::HunkZero => config.zero_style.is_syntax_highlighted,
            State::HunkPlus => {
                config.plus_style.is_syntax_highlighted
                    || config.plus_emph_style.is_syntax_highlighted
            }
            State::HunkHeader => true,
            _ => panic!(
                "should_compute_syntax_highlighting is undefined for state {:?}",
                state
            ),
        }
    }

    pub fn get_syntax_style_sections_for_lines<'s>(
        lines: &'s Vec<String>,
        state: &State,
        highlighter: &mut HighlightLines,
        config: &config::Config,
    ) -> Vec<Vec<(SyntectStyle, &'s str)>> {
        let fake = !Painter::should_compute_syntax_highlighting(state, config);
        let mut line_sections = Vec::new();
        for line in lines.iter() {
            if fake {
                line_sections.push(vec![(config.null_syntect_style, line.as_str())])
            } else {
                line_sections.push(highlighter.highlight(line, &config.syntax_set))
            }
        }
        line_sections
    }

    /// Set background styles to represent diff for minus and plus lines in buffer.
    fn get_diff_style_sections<'b>(
        minus_lines: &'b Vec<String>,
        plus_lines: &'b Vec<String>,
        config: &config::Config,
    ) -> (Vec<Vec<(Style, &'b str)>>, Vec<Vec<(Style, &'b str)>>) {
        let mut diff_sections = edits::infer_edits(
            minus_lines,
            plus_lines,
            config.minus_style,
            config.minus_emph_style,
            config.plus_style,
            config.plus_emph_style,
            &config.tokenization_regex,
            config.max_line_distance,
            config.max_line_distance_for_naively_paired_lines,
        );

        let minus_non_emph_style = if config.minus_non_emph_style != config.minus_emph_style {
            Some(config.minus_non_emph_style)
        } else {
            None
        };
        Self::update_styles(&mut diff_sections.0, None, minus_non_emph_style);
        let plus_non_emph_style = if config.plus_non_emph_style != config.plus_emph_style {
            Some(config.plus_non_emph_style)
        } else {
            None
        };
        Self::update_styles(
            &mut diff_sections.1,
            Some(config.whitespace_error_style),
            plus_non_emph_style,
        );
        diff_sections
    }

    /// There are some rules according to which we update line section styles that were computed
    /// during the initial edit inference pass. This function applies those rules. The rules are
    /// 1. If there are multiple diff styles in the line, then the line must have some
    ///    inferred edit operations and so, if there is a special non-emph style that is
    ///    distinct from the default style, then it should be used for the non-emph style
    ///    sections.
    /// 2. If the line constitutes a whitespace error, then the whitespace error style
    ///    should be applied to the added material.
    fn update_styles(
        style_sections: &mut Vec<Vec<(Style, &str)>>,
        whitespace_error_style: Option<Style>,
        non_emph_style: Option<Style>,
    ) {
        for line_sections in style_sections {
            let line_has_emph_and_non_emph_sections =
                style_sections_contain_more_than_one_style(line_sections);
            let should_update_non_emph_styles =
                non_emph_style.is_some() && line_has_emph_and_non_emph_sections;
            let is_whitespace_error =
                whitespace_error_style.is_some() && is_whitespace_error(line_sections);
            for section in line_sections.iter_mut() {
                // If the line as a whole constitutes a whitespace error then highlight this
                // section if either (a) it is an emph section, or (b) the line lacks any
                // emph/non-emph distinction.
                if is_whitespace_error
                    && (section.0.is_emph || !line_has_emph_and_non_emph_sections)
                {
                    *section = (whitespace_error_style.unwrap(), section.1);
                }
                // Otherwise, update the style if this is a non-emph section that needs updating.
                else if should_update_non_emph_styles && !section.0.is_emph {
                    *section = (non_emph_style.unwrap(), section.1);
                }
            }
        }
    }
}

// edits::annotate doesn't return "coalesced" annotations (see comment there), so we can't assume
// that `sections.len() > 1 <=> (multiple styles)`.
fn style_sections_contain_more_than_one_style(sections: &Vec<(Style, &str)>) -> bool {
    if sections.len() > 1 {
        let (first_style, _) = sections[0];
        sections
            .iter()
            .filter(|(style, _)| *style != first_style)
            .next()
            .is_some()
    } else {
        false
    }
}

lazy_static! {
    static ref NON_WHITESPACE_REGEX: Regex = Regex::new(r"\S").unwrap();
}

/// True iff the line represented by `sections` constitutes a whitespace error.
// TODO: Git recognizes blank lines at end of file (blank-at-eof) as a whitespace error but delta
// does not yet.
// https://git-scm.com/docs/git-config#Documentation/git-config.txt-corewhitespace
fn is_whitespace_error(sections: &Vec<(Style, &str)>) -> bool {
    !sections
        .iter()
        .any(|(_, s)| NON_WHITESPACE_REGEX.is_match(s))
}

mod superimpose_style_sections {
    use syntect::highlighting::Style as SyntectStyle;

    use crate::bat::terminal::to_ansi_color;
    use crate::style::Style;

    pub fn superimpose_style_sections(
        sections_1: &[(SyntectStyle, &str)],
        sections_2: &[(Style, &str)],
        true_color: bool,
        null_syntect_style: SyntectStyle,
    ) -> Vec<(Style, String)> {
        coalesce(
            superimpose(
                explode(sections_1)
                    .iter()
                    .zip(explode(sections_2))
                    .collect::<Vec<(&(SyntectStyle, char), (Style, char))>>(),
            ),
            true_color,
            null_syntect_style,
        )
    }

    fn explode<T>(style_sections: &[(T, &str)]) -> Vec<(T, char)>
    where
        T: Copy,
    {
        let mut exploded: Vec<(T, char)> = Vec::new();
        for (style, s) in style_sections {
            for c in s.chars() {
                exploded.push((*style, c));
            }
        }
        exploded
    }

    fn superimpose(
        style_section_pairs: Vec<(&(SyntectStyle, char), (Style, char))>,
    ) -> Vec<((SyntectStyle, Style), char)> {
        let mut superimposed: Vec<((SyntectStyle, Style), char)> = Vec::new();
        for ((syntax_style, char_1), (style, char_2)) in style_section_pairs {
            if *char_1 != char_2 {
                panic!(
                    "String mismatch encountered while superimposing style sections: '{}' vs '{}'",
                    *char_1, char_2
                )
            }
            superimposed.push(((*syntax_style, style), *char_1));
        }
        superimposed
    }

    fn coalesce(
        style_sections: Vec<((SyntectStyle, Style), char)>,
        true_color: bool,
        null_syntect_style: SyntectStyle,
    ) -> Vec<(Style, String)> {
        let make_superimposed_style = |(syntect_style, style): (SyntectStyle, Style)| {
            if style.is_syntax_highlighted && syntect_style != null_syntect_style {
                Style {
                    ansi_term_style: ansi_term::Style {
                        foreground: Some(to_ansi_color(syntect_style.foreground, true_color)),
                        ..style.ansi_term_style
                    },
                    ..style
                }
            } else {
                style
            }
        };
        let mut coalesced: Vec<(Style, String)> = Vec::new();
        let mut style_sections = style_sections.iter();
        if let Some((style_pair, c)) = style_sections.next() {
            let mut current_string = c.to_string();
            let mut current_style_pair = style_pair;
            for (style_pair, c) in style_sections {
                if style_pair != current_style_pair {
                    let style = make_superimposed_style(*current_style_pair);
                    coalesced.push((style, current_string));
                    current_string = String::new();
                    current_style_pair = style_pair;
                }
                current_string.push(*c);
            }

            // TODO: This is not the ideal location for the following code.
            if current_string.ends_with("\n") {
                // Remove the terminating newline whose presence was necessary for the syntax
                // highlighter to work correctly.
                current_string.truncate(current_string.len() - 1);
            }
            let style = make_superimposed_style(*current_style_pair);
            coalesced.push((style, current_string));
        }
        coalesced
    }

    #[cfg(test)]
    mod tests {
        use lazy_static::lazy_static;

        use super::*;
        use ansi_term::{self, Color};
        use syntect::highlighting::Color as SyntectColor;
        use syntect::highlighting::FontStyle as SyntectFontStyle;
        use syntect::highlighting::Style as SyntectStyle;

        use crate::style::{DecorationStyle, Style};

        lazy_static! {
            static ref SYNTAX_STYLE: SyntectStyle = SyntectStyle {
                foreground: SyntectColor::BLACK,
                background: SyntectColor::BLACK,
                font_style: SyntectFontStyle::BOLD,
            };
        }
        lazy_static! {
            static ref SYNTAX_HIGHLIGHTED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: Some(Color::White),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: true,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }
        lazy_static! {
            static ref NON_SYNTAX_HIGHLIGHTED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: Some(Color::White),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: false,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }
        lazy_static! {
            static ref SUPERIMPOSED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: Some(to_ansi_color(SyntectColor::BLACK, true)),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: true,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }

        #[test]
        fn test_superimpose_style_sections_1() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*SYNTAX_HIGHLIGHTED_STYLE, "ab")];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_2() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![
                (*SYNTAX_HIGHLIGHTED_STYLE, "a"),
                (*SYNTAX_HIGHLIGHTED_STYLE, "b"),
            ];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, String::from("ab"))];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_3() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*NON_SYNTAX_HIGHLIGHTED_STYLE, "ab")];
            let superimposed = vec![(*NON_SYNTAX_HIGHLIGHTED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_explode() {
            let arbitrary = 0;
            assert_eq!(
                explode(&vec![(arbitrary, "ab")]),
                vec![(arbitrary, 'a'), (arbitrary, 'b')]
            )
        }

        #[test]
        fn test_superimpose() {
            let x = (*SYNTAX_STYLE, 'a');
            let pairs = vec![(&x, (*SYNTAX_HIGHLIGHTED_STYLE, 'a'))];
            assert_eq!(
                superimpose(pairs),
                vec![((*SYNTAX_STYLE, *SYNTAX_HIGHLIGHTED_STYLE), 'a')]
            );
        }
    }
}
