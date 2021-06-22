use std::io::Write;

use itertools::Itertools;
use syntect::easy::HighlightLines;
use syntect::highlighting::Style as SyntectStyle;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::config::{self, delta_unreachable};
use crate::delta::State;
use crate::edits;
use crate::features::line_numbers;
use crate::features::side_by_side;
use crate::paint::superimpose_style_sections::superimpose_style_sections;
use crate::style::Style;

pub struct Painter<'a> {
    pub minus_lines: Vec<(String, State)>,
    pub plus_lines: Vec<(String, State)>,
    pub writer: &'a mut dyn Write,
    pub syntax: &'a SyntaxReference,
    pub highlighter: HighlightLines<'a>,
    pub config: &'a config::Config,
    pub output_buffer: String,
    pub line_numbers_data: line_numbers::LineNumbersData<'a>,
}

impl<'a> Painter<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a config::Config) -> Self {
        let default_syntax = Self::get_syntax(&config.syntax_set, None);
        // TODO: Avoid doing this.
        let dummy_highlighter = HighlightLines::new(default_syntax, &config.syntax_dummy_theme);

        let line_numbers_data = if config.line_numbers {
            line_numbers::LineNumbersData::from_format_strings(
                &config.line_numbers_left_format,
                &config.line_numbers_right_format,
            )
        } else {
            line_numbers::LineNumbersData::default()
        };
        Self {
            minus_lines: Vec::new(),
            plus_lines: Vec::new(),
            output_buffer: String::new(),
            syntax: default_syntax,
            highlighter: dummy_highlighter,
            writer,
            config,
            line_numbers_data,
        }
    }

    pub fn set_syntax(&mut self, extension: Option<&str>) {
        self.syntax = Painter::get_syntax(&self.config.syntax_set, extension);
    }

    fn get_syntax(syntax_set: &'a SyntaxSet, extension: Option<&str>) -> &'a SyntaxReference {
        if let Some(extension) = extension {
            if let Some(syntax) = syntax_set.find_syntax_by_extension(extension) {
                return syntax;
            }
        }
        syntax_set
            .find_syntax_by_extension("txt")
            .unwrap_or_else(|| delta_unreachable("Failed to find any language syntax definitions."))
    }

    pub fn set_highlighter(&mut self) {
        if let Some(ref syntax_theme) = self.config.syntax_theme {
            self.highlighter = HighlightLines::new(self.syntax, &syntax_theme)
        };
    }

    /// Replace initial -/+ character with ' ', expand tabs as spaces, and optionally terminate with
    /// newline.
    // Terminating with newline character is necessary for many of the sublime syntax definitions to
    // highlight correctly.
    // See https://docs.rs/syntect/3.2.0/syntect/parsing/struct.SyntaxSetBuilder.html#method.add_from_folder
    pub fn prepare(&self, line: &str) -> String {
        if !line.is_empty() {
            let mut line = line.graphemes(true);

            // The first column contains a -/+/space character, added by git. We substitute it for
            // a space now, so that it is not present during syntax highlighting. When emitting the
            // line in Painter::paint_line, we drop the space (unless --keep-plus-minus-markers is
            // in effect in which case we replace it with the appropriate marker).
            // TODO: Things should, but do not, work if this leading space is omitted at this stage.
            // See comment in align::Alignment::new.
            line.next();
            format!(" {}\n", self.expand_tabs(line))
        } else {
            "\n".to_string()
        }
    }

    /// Remove the initial +/- character of a line that will be emitted unchanged, including any
    /// ANSI escape sequences.
    pub fn prepare_raw_line(&self, line: &str) -> String {
        ansi::ansi_preserving_slice(
            &self.expand_tabs(line.graphemes(true)),
            if self.config.keep_plus_minus_markers {
                0
            } else {
                1
            },
        )
    }

    /// Expand tabs as spaces.
    /// tab_width = 0 is documented to mean do not replace tabs.
    pub fn expand_tabs<'b, I>(&self, line: I) -> String
    where
        I: Iterator<Item = &'b str>,
    {
        if self.config.tab_width > 0 {
            let tab_replacement = " ".repeat(self.config.tab_width);
            line.map(|s| if s == "\t" { &tab_replacement } else { s })
                .collect::<String>()
        } else {
            line.collect::<String>()
        }
    }

    pub fn paint_buffered_minus_and_plus_lines(&mut self) {
        let minus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.minus_lines,
            &State::HunkMinus(None),
            &mut self.highlighter,
            self.config,
        );
        let plus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.plus_lines,
            &State::HunkPlus(None),
            &mut self.highlighter,
            self.config,
        );
        let (minus_line_diff_style_sections, plus_line_diff_style_sections, line_alignment) =
            Self::get_diff_style_sections(&self.minus_lines, &self.plus_lines, self.config);

        if self.config.side_by_side {
            side_by_side::paint_minus_and_plus_lines_side_by_side(
                minus_line_syntax_style_sections,
                minus_line_diff_style_sections,
                self.minus_lines.iter().map(|(_, state)| state).collect(),
                plus_line_syntax_style_sections,
                plus_line_diff_style_sections,
                self.plus_lines.iter().map(|(_, state)| state).collect(),
                line_alignment,
                &mut self.output_buffer,
                self.config,
                &mut Some(&mut self.line_numbers_data),
                None,
            );
        } else {
            if !self.minus_lines.is_empty() {
                Painter::paint_lines(
                    minus_line_syntax_style_sections,
                    minus_line_diff_style_sections,
                    self.minus_lines.iter().map(|(_, state)| state),
                    &mut self.output_buffer,
                    self.config,
                    &mut Some(&mut self.line_numbers_data),
                    if self.config.keep_plus_minus_markers {
                        Some(self.config.minus_style.paint("-"))
                    } else {
                        None
                    },
                    Some(self.config.minus_empty_line_marker_style),
                    None,
                );
            }
            if !self.plus_lines.is_empty() {
                Painter::paint_lines(
                    plus_line_syntax_style_sections,
                    plus_line_diff_style_sections,
                    self.plus_lines.iter().map(|(_, state)| state),
                    &mut self.output_buffer,
                    self.config,
                    &mut Some(&mut self.line_numbers_data),
                    if self.config.keep_plus_minus_markers {
                        Some(self.config.plus_style.paint("+"))
                    } else {
                        None
                    },
                    Some(self.config.plus_empty_line_marker_style),
                    None,
                );
            }
        }
        self.minus_lines.clear();
        self.plus_lines.clear();
    }

    pub fn paint_zero_line(&mut self, line: &str) {
        let state = State::HunkZero;
        let painted_prefix = if self.config.keep_plus_minus_markers && !line.is_empty() {
            Some(self.config.zero_style.paint(&line[..1]))
        } else {
            None
        };
        let lines = vec![(self.prepare(line), state.clone())];
        let syntax_style_sections = Painter::get_syntax_style_sections_for_lines(
            &lines,
            &state,
            &mut self.highlighter,
            &self.config,
        );
        let diff_style_sections = vec![(self.config.zero_style, lines[0].0.as_str())]; // TODO: compute style from state

        if self.config.side_by_side {
            side_by_side::paint_zero_lines_side_by_side(
                syntax_style_sections,
                vec![diff_style_sections],
                &State::HunkZero,
                &mut self.output_buffer,
                self.config,
                &mut Some(&mut self.line_numbers_data),
                painted_prefix,
                None,
            );
        } else {
            Painter::paint_lines(
                syntax_style_sections,
                vec![diff_style_sections],
                [state].iter(),
                &mut self.output_buffer,
                self.config,
                &mut Some(&mut self.line_numbers_data),
                painted_prefix,
                None,
                None,
            );
        }
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_lines(
        syntax_style_sections: Vec<Vec<(SyntectStyle, &str)>>,
        diff_style_sections: Vec<Vec<(Style, &str)>>,
        states: impl Iterator<Item = &'a State>,
        output_buffer: &mut String,
        config: &config::Config,
        line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
        painted_prefix: Option<ansi_term::ANSIString>,
        empty_line_style: Option<Style>, // a style with background color to highlight an empty line
        background_color_extends_to_terminal_width: Option<bool>,
    ) {
        // There's some unfortunate hackery going on here for two reasons:
        //
        // 1. The prefix needs to be injected into the output stream.
        //
        // 2. We must ensure that we fill rightwards with the appropriate
        //    non-emph background color. In that case we don't use the last
        //    style of the line, because this might be emph.
        for (state, (syntax_sections, diff_sections)) in states.zip_eq(
            syntax_style_sections
                .iter()
                .zip_eq(diff_style_sections.iter()),
        ) {
            let (mut line, line_is_empty) = Painter::paint_line(
                syntax_sections,
                diff_sections,
                state,
                line_numbers_data,
                None,
                painted_prefix.clone(),
                config,
            );
            let (should_right_fill_background_color, fill_style) =
                Painter::get_should_right_fill_background_color_and_fill_style(
                    diff_sections,
                    state,
                    background_color_extends_to_terminal_width,
                    config,
                );
            if should_right_fill_background_color {
                Painter::right_fill_background_color(&mut line, fill_style);
            } else if line_is_empty {
                if let Some(empty_line_style) = empty_line_style {
                    Painter::mark_empty_line(
                        &empty_line_style,
                        &mut line,
                        if config.line_numbers { Some(" ") } else { None },
                    );
                }
            };
            output_buffer.push_str(&line);
            output_buffer.push('\n');
        }
    }

    /// Determine whether the terminal should fill the line rightwards with a background color, and
    /// the style for doing so.
    pub fn get_should_right_fill_background_color_and_fill_style(
        diff_sections: &[(Style, &str)],
        state: &State,
        background_color_extends_to_terminal_width: Option<bool>,
        config: &config::Config,
    ) -> (bool, Style) {
        // style:          for right fill if line contains no emph sections
        // non_emph_style: for right fill if line contains emph sections
        let (style, non_emph_style) = match state {
            State::HunkMinus(None) => (config.minus_style, config.minus_non_emph_style),
            State::HunkMinus(Some(raw_line)) => {
                // TODO: This is the second time we are parsing the ANSI sequences
                if let Some(ansi_term_style) = ansi::parse_first_style(raw_line) {
                    let style = Style {
                        ansi_term_style,
                        ..Style::new()
                    };
                    (style, style)
                } else {
                    (config.minus_style, config.minus_non_emph_style)
                }
            }
            State::HunkZero => (config.zero_style, config.zero_style),
            State::HunkPlus(None) => (config.plus_style, config.plus_non_emph_style),
            State::HunkPlus(Some(raw_line)) => {
                // TODO: This is the second time we are parsing the ANSI sequences
                if let Some(ansi_term_style) = ansi::parse_first_style(raw_line) {
                    let style = Style {
                        ansi_term_style,
                        ..Style::new()
                    };
                    (style, style)
                } else {
                    (config.plus_style, config.plus_non_emph_style)
                }
            }
            _ => (config.null_style, config.null_style),
        };
        let fill_style = if style_sections_contain_more_than_one_style(diff_sections) {
            non_emph_style // line contains an emph section
        } else {
            style
        };
        let should_right_fill_background_color = fill_style.get_background_color().is_some()
            && background_color_extends_to_terminal_width
                .unwrap_or(config.background_color_extends_to_terminal_width);
        (should_right_fill_background_color, fill_style)
    }

    /// Emit line with ANSI sequences that extend the background color to the terminal width.
    pub fn right_fill_background_color(line: &mut String, fill_style: Style) {
        // HACK: How to properly incorporate the ANSI_CSI_CLEAR_TO_EOL into ansi_strings?
        line.push_str(&ansi_term::ANSIStrings(&[fill_style.paint("")]).to_string());
        if line
            .to_lowercase()
            .ends_with(&ansi::ANSI_SGR_RESET.to_lowercase())
        {
            line.truncate(line.len() - ansi::ANSI_SGR_RESET.len());
        }
        line.push_str(ansi::ANSI_CSI_CLEAR_TO_EOL);
        line.push_str(ansi::ANSI_SGR_RESET);
    }

    /// Use ANSI sequences to visually mark the current line as empty. If `marker` is None then the
    /// line is marked using terminal emulator colors only, i.e. without appending any marker text
    /// to the line. This is typically appropriate only when the `line` buffer is empty, since
    /// otherwise the ANSI_CSI_CLEAR_TO_BOL instruction would overwrite the text to the left of the
    /// current buffer position.
    pub fn mark_empty_line(empty_line_style: &Style, line: &mut String, marker: Option<&str>) {
        line.push_str(
            &empty_line_style
                .paint(marker.unwrap_or(ansi::ANSI_CSI_CLEAR_TO_BOL))
                .to_string(),
        );
    }

    /// Return painted line (maybe prefixed with line numbers field) and an is_empty? boolean.
    pub fn paint_line(
        syntax_sections: &[(SyntectStyle, &str)],
        diff_sections: &[(Style, &str)],
        state: &State,
        line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
        side_by_side_panel: Option<side_by_side::PanelSide>,
        painted_prefix: Option<ansi_term::ANSIString>,
        config: &config::Config,
    ) -> (String, bool) {
        let output_line_numbers = config.line_numbers && line_numbers_data.is_some();
        let mut handled_prefix = false;
        let mut ansi_strings = Vec::new();
        if output_line_numbers {
            ansi_strings.extend(line_numbers::format_and_paint_line_numbers(
                line_numbers_data.as_mut().unwrap(),
                state,
                side_by_side_panel,
                config,
            ))
        }
        match state {
            State::HunkMinus(Some(raw_line)) | State::HunkPlus(Some(raw_line)) => {
                // This line has been identified as one which should be emitted unchanged,
                // including any ANSI escape sequences that it has.
                return (
                    format!(
                        "{}{}",
                        ansi_term::ANSIStrings(&ansi_strings).to_string(),
                        raw_line
                    ),
                    false,
                );
            }
            _ => {}
        }
        let mut is_empty = true;
        for (section_style, mut text) in superimpose_style_sections(
            syntax_sections,
            diff_sections,
            config.true_color,
            config.null_syntect_style,
        ) {
            if !handled_prefix {
                if let Some(painted_prefix) = painted_prefix.clone() {
                    ansi_strings.push(painted_prefix);
                }
                if !text.is_empty() {
                    text.remove(0);
                }
                handled_prefix = true;
            }
            if !text.is_empty() {
                ansi_strings.push(section_style.paint(text));
                is_empty = false;
            }
        }
        (ansi_term::ANSIStrings(&ansi_strings).to_string(), is_empty)
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
            State::HunkMinus(None) => {
                config.minus_style.is_syntax_highlighted
                    || config.minus_emph_style.is_syntax_highlighted
            }
            State::HunkZero => config.zero_style.is_syntax_highlighted,
            State::HunkPlus(None) => {
                config.plus_style.is_syntax_highlighted
                    || config.plus_emph_style.is_syntax_highlighted
            }
            State::HunkHeader => true,
            State::HunkMinus(Some(_)) | State::HunkPlus(Some(_)) => false,
            _ => panic!(
                "should_compute_syntax_highlighting is undefined for state {:?}",
                state
            ),
        }
    }

    pub fn get_syntax_style_sections_for_lines<'s>(
        lines: &'s [(String, State)],
        state: &State,
        highlighter: &mut HighlightLines,
        config: &config::Config,
    ) -> Vec<Vec<(SyntectStyle, &'s str)>> {
        let fake = !Painter::should_compute_syntax_highlighting(state, config);
        let mut line_sections = Vec::new();
        for (line, _) in lines.iter() {
            if fake {
                line_sections.push(vec![(config.null_syntect_style, line.as_str())])
            } else {
                // The first character is a space injected by delta. See comment in
                // Painter:::prepare.
                let mut this_line_sections = highlighter.highlight(&line[1..], &config.syntax_set);
                this_line_sections.insert(0, (config.null_syntect_style, &line[..1]));
                line_sections.push(this_line_sections);
            }
        }
        line_sections
    }

    /// Set background styles to represent diff for minus and plus lines in buffer.
    #[allow(clippy::type_complexity)]
    fn get_diff_style_sections<'b>(
        minus_lines: &'b [(String, State)],
        plus_lines: &'b [(String, State)],
        config: &config::Config,
    ) -> (
        Vec<Vec<(Style, &'b str)>>,
        Vec<Vec<(Style, &'b str)>>,
        Vec<(Option<usize>, Option<usize>)>,
    ) {
        let (minus_lines, minus_styles): (Vec<&str>, Vec<Style>) = minus_lines
            .iter()
            .map(|(s, t)| (s.as_str(), *config.get_style(&t)))
            .unzip();
        let (plus_lines, plus_styles): (Vec<&str>, Vec<Style>) = plus_lines
            .iter()
            .map(|(s, t)| (s.as_str(), *config.get_style(&t)))
            .unzip();
        let mut diff_sections = edits::infer_edits(
            minus_lines,
            plus_lines,
            minus_styles,
            config.minus_emph_style, // FIXME
            plus_styles,
            config.plus_emph_style, // FIXME
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
fn style_sections_contain_more_than_one_style(sections: &[(Style, &str)]) -> bool {
    if sections.len() > 1 {
        let (first_style, _) = sections[0];
        sections.iter().any(|(style, _)| *style != first_style)
    } else {
        false
    }
}

/// True iff the line represented by `sections` constitutes a whitespace error.
// Note that a space is always present as the first character in the line (it was put there as a
// replacement for the leading +/- marker; see paint::prepare()). A line is a whitespace error iff,
// beyond the initial space character, (a) there are more characters and (b) they are all
// whitespace characters.
// TODO: Git recognizes blank lines at end of file (blank-at-eof) as a whitespace error but delta
// does not yet.
// https://git-scm.com/docs/git-config#Documentation/git-config.txt-corewhitespace
fn is_whitespace_error(sections: &[(Style, &str)]) -> bool {
    let mut any_chars = false;
    for c in sections.iter().flat_map(|(_, s)| s.chars()).skip(1) {
        if c == '\n' {
            return any_chars;
        } else if c != ' ' && c != '\t' {
            return false;
        } else {
            any_chars = true;
        }
    }
    false
}

mod superimpose_style_sections {
    use syntect::highlighting::Style as SyntectStyle;

    use crate::bat_utils::terminal::to_ansi_color;
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

    #[allow(clippy::type_complexity)]
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
                        foreground: to_ansi_color(syntect_style.foreground, true_color),
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
            if current_string.ends_with('\n') {
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
                    foreground: to_ansi_color(SyntectColor::BLACK, true),
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
                explode(&[(arbitrary, "ab")]),
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
