use std::io::Write;

use ansi_term::{self, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::Style as SyntectStyle;
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::config;
use crate::delta::State;
use crate::edits;
use crate::paint::superimpose_style_sections::superimpose_style_sections;

pub const ANSI_CSI_ERASE_IN_LINE: &str = "\x1b[K";
pub const ANSI_SGR_RESET: &str = "\x1b[0m";

pub struct Painter<'a> {
    pub minus_lines: Vec<String>,
    pub plus_lines: Vec<String>,
    pub writer: &'a mut dyn Write,
    pub syntax: &'a SyntaxReference,
    pub highlighter: HighlightLines<'a>,
    pub config: &'a config::Config<'a>,
    pub output_buffer: String,
}

impl<'a> Painter<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a config::Config) -> Self {
        let default_syntax = Self::get_syntax(&config.syntax_set, None);
        // TODO: Avoid doing this.
        let dummy_highlighter = HighlightLines::new(default_syntax, &config.dummy_theme);
        Self {
            minus_lines: Vec::new(),
            plus_lines: Vec::new(),
            output_buffer: String::new(),
            syntax: default_syntax,
            highlighter: dummy_highlighter,
            writer,
            config,
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
        if let Some(ref theme) = self.config.theme {
            self.highlighter = HighlightLines::new(self.syntax, &theme)
        };
    }

    pub fn paint_buffered_lines(&mut self) {
        let minus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.minus_lines,
            self.config.should_syntax_highlight(&State::HunkMinus),
            &mut self.highlighter,
            self.config,
        );
        let plus_line_syntax_style_sections = Self::get_syntax_style_sections_for_lines(
            &self.plus_lines,
            self.config.should_syntax_highlight(&State::HunkPlus),
            &mut self.highlighter,
            self.config,
        );
        let (minus_line_diff_style_sections, plus_line_diff_style_sections) =
            Self::get_diff_style_sections(&self.minus_lines, &self.plus_lines, self.config);
        // TODO: lines and style sections contain identical line text
        if !self.minus_lines.is_empty() {
            Painter::paint_lines(
                minus_line_syntax_style_sections,
                minus_line_diff_style_sections,
                &mut self.output_buffer,
                self.config,
                self.config.minus_line_marker,
                self.config.minus_style,
                None,
            );
        }
        if !self.plus_lines.is_empty() {
            Painter::paint_lines(
                plus_line_syntax_style_sections,
                plus_line_diff_style_sections,
                &mut self.output_buffer,
                self.config,
                self.config.plus_line_marker,
                self.config.plus_style,
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
        output_buffer: &mut String,
        config: &config::Config,
        prefix: &str,
        background_style: Style,
        background_color_extends_to_terminal_width: Option<bool>,
    ) {
        for (syntax_sections, diff_sections) in
            syntax_style_sections.iter().zip(diff_style_sections.iter())
        {
            let mut ansi_strings = Vec::new();
            if prefix != "" {
                ansi_strings.push(background_style.paint(prefix));
            }
            let mut dropped_prefix = prefix == ""; // TODO: Hack
            for (style, mut text) in
                superimpose_style_sections(syntax_sections, diff_sections, config.true_color)
            {
                if !dropped_prefix {
                    if text.len() > 0 {
                        text.remove(0);
                    }
                    dropped_prefix = true;
                }
                ansi_strings.push(style.paint(text));
            }
            ansi_strings.push(background_style.paint(""));
            let line = &mut ansi_term::ANSIStrings(&ansi_strings).to_string();
            let background_color_extends_to_terminal_width =
                match background_color_extends_to_terminal_width {
                    Some(boolean) => boolean,
                    None => config.background_color_extends_to_terminal_width,
                };
            if background_color_extends_to_terminal_width {
                // HACK: How to properly incorporate the ANSI_CSI_ERASE_IN_LINE into ansi_strings?
                if line
                    .to_lowercase()
                    .ends_with(&ANSI_SGR_RESET.to_lowercase())
                {
                    line.truncate(line.len() - ANSI_SGR_RESET.len());
                }
                output_buffer.push_str(&line);
                output_buffer.push_str(ANSI_CSI_ERASE_IN_LINE);
                output_buffer.push_str(ANSI_SGR_RESET);
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

    fn get_syntax_style_sections_for_lines<'s>(
        lines: &'s [String],
        should_syntax_highlight: bool,
        highlighter: &mut HighlightLines,
        config: &config::Config,
    ) -> Vec<Vec<(SyntectStyle, &'s str)>> {
        let mut line_sections = Vec::new();
        for line in lines.iter() {
            line_sections.push(Painter::get_line_syntax_style_sections(
                line,
                should_syntax_highlight,
                highlighter,
                &config,
            ));
        }
        line_sections
    }

    pub fn get_line_syntax_style_sections(
        line: &'a str,
        should_syntax_highlight: bool,
        highlighter: &mut HighlightLines,
        config: &config::Config,
    ) -> Vec<(SyntectStyle, &'a str)> {
        if should_syntax_highlight && config.theme.is_some() {
            highlighter.highlight(line, &config.syntax_set)
        } else {
            vec![(config.null_syntect_style, line)]
        }
    }

    /// Set background styles to represent diff for minus and plus lines in buffer.
    fn get_diff_style_sections<'b>(
        minus_lines: &'b [String],
        plus_lines: &'b [String],
        config: &config::Config,
    ) -> (Vec<Vec<(Style, &'b str)>>, Vec<Vec<(Style, &'b str)>>) {
        edits::infer_edits(
            minus_lines,
            plus_lines,
            config.minus_style,
            config.minus_emph_style,
            config.plus_style,
            config.plus_emph_style,
            config.max_line_distance,
            config.max_line_distance_for_naively_paired_lines,
        )
    }
}

mod superimpose_style_sections {
    use ansi_term::Style;
    use syntect::highlighting::Style as SyntectStyle;

    use crate::bat::terminal::to_ansi_color;

    pub fn superimpose_style_sections(
        sections_1: &[(SyntectStyle, &str)],
        sections_2: &[(Style, &str)],
        true_color: bool,
    ) -> Vec<(Style, String)> {
        coalesce(
            superimpose(
                explode(sections_1)
                    .iter()
                    .zip(explode(sections_2))
                    .collect::<Vec<(&(SyntectStyle, char), (Style, char))>>(),
            ),
            true_color,
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
    ) -> Vec<(Style, String)> {
        let mut coalesced: Vec<(Style, String)> = Vec::new();
        let mut style_sections = style_sections.iter();
        if let Some((style_pair, c)) = style_sections.next() {
            let mut current_string = c.to_string();
            let mut current_style_pair = style_pair;
            for (style_pair, c) in style_sections {
                if style_pair != current_style_pair {
                    let (syntect_style, style) = current_style_pair;
                    let superimposed_style = Style {
                        foreground: Some(to_ansi_color(syntect_style.foreground, true_color)),
                        ..*style
                    };
                    coalesced.push((superimposed_style, current_string));
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

            let (syntect_style, style) = current_style_pair;
            let superimposed_style = Style {
                foreground: Some(to_ansi_color(syntect_style.foreground, true_color)),
                ..*style
            };
            coalesced.push((superimposed_style, current_string));
        }
        coalesced
    }

    #[cfg(test)]
    mod tests {
        use lazy_static::lazy_static;

        use super::*;
        use ansi_term::{Color, Style};
        use syntect::highlighting::Color as SyntectColor;
        use syntect::highlighting::FontStyle as SyntectFontStyle;
        use syntect::highlighting::Style as SyntectStyle;

        lazy_static! {
            static ref SYNTAX_STYLE: SyntectStyle = SyntectStyle {
                foreground: SyntectColor::BLACK,
                background: SyntectColor::BLACK,
                font_style: SyntectFontStyle::BOLD,
            };
        }
        lazy_static! {
            static ref STYLE: Style = Style {
                foreground: Some(Color::White),
                background: Some(Color::White),
                is_underline: true,
                ..Style::new()
            };
        }
        lazy_static! {
            static ref SUPERIMPOSED_STYLE: Style = Style {
                foreground: Some(to_ansi_color(SyntectColor::BLACK, true)),
                background: Some(Color::White),
                is_underline: true,
                ..Style::new()
            };
        }

        #[test]
        fn test_superimpose_style_sections_1() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*STYLE, "ab")];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_2() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*STYLE, "a"), (*STYLE, "b")];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, String::from("ab"))];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true),
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
            let pairs = vec![(&x, (*STYLE, 'a'))];
            assert_eq!(superimpose(pairs), vec![((*SYNTAX_STYLE, *STYLE), 'a')]);
        }
    }
}
