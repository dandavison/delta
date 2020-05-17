use std::io::Write;
use std::str::FromStr;

use ansi_term;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, StyleModifier};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::bat::terminal::to_ansi_color;
use crate::config;
use crate::delta::State;
use crate::edits;
use crate::paint::superimpose_style_sections::superimpose_style_sections;
use crate::style;

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
                self.config.minus_style_modifier,
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
                self.config.plus_style_modifier,
                None,
            );
        }
        self.minus_lines.clear();
        self.plus_lines.clear();
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    pub fn paint_lines(
        syntax_style_sections: Vec<Vec<(Style, &str)>>,
        diff_style_sections: Vec<Vec<(StyleModifier, &str)>>,
        output_buffer: &mut String,
        config: &config::Config,
        prefix: &str,
        background_style_modifier: StyleModifier,
        background_color_extends_to_terminal_width: Option<bool>,
    ) {
        let background_style = config.no_style.apply(background_style_modifier);
        let background_ansi_style = to_ansi_style(background_style, config.true_color);
        for (syntax_sections, diff_sections) in
            syntax_style_sections.iter().zip(diff_style_sections.iter())
        {
            let mut ansi_strings = Vec::new();
            if prefix != "" {
                ansi_strings.push(background_ansi_style.paint(prefix));
            }
            let mut dropped_prefix = prefix == ""; // TODO: Hack
            for (style, mut text) in superimpose_style_sections(syntax_sections, diff_sections) {
                if !dropped_prefix {
                    if text.len() > 0 {
                        text.remove(0);
                    }
                    dropped_prefix = true;
                }
                ansi_strings.push(to_ansi_style(style, config.true_color).paint(text));
            }
            ansi_strings.push(background_ansi_style.paint(""));
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
    ) -> Vec<Vec<(Style, &'s str)>> {
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
    ) -> Vec<(Style, &'a str)> {
        if should_syntax_highlight && config.theme.is_some() {
            highlighter.highlight(line, &config.syntax_set)
        } else {
            vec![(config.no_style, line)]
        }
    }

    /// Set background styles to represent diff for minus and plus lines in buffer.
    fn get_diff_style_sections<'b>(
        minus_lines: &'b [String],
        plus_lines: &'b [String],
        config: &config::Config,
    ) -> (
        Vec<Vec<(StyleModifier, &'b str)>>,
        Vec<Vec<(StyleModifier, &'b str)>>,
    ) {
        edits::infer_edits(
            minus_lines,
            plus_lines,
            config.minus_style_modifier,
            config.minus_emph_style_modifier,
            config.plus_style_modifier,
            config.plus_emph_style_modifier,
            config.max_line_distance,
            config.max_line_distance_for_naively_paired_lines,
        )
    }
}

pub fn to_ansi_style(style: Style, true_color: bool) -> ansi_term::Style {
    let mut ansi_style = ansi_term::Style::new();
    if style.background != style::NO_COLOR {
        ansi_style = ansi_style.on(to_ansi_color(style.background, true_color));
    }
    if style.foreground != style::NO_COLOR {
        ansi_style = ansi_style.fg(to_ansi_color(style.foreground, true_color));
    }
    if style.font_style.contains(FontStyle::BOLD) {
        ansi_style.is_bold = true;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ansi_style.is_italic = true;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        ansi_style.is_underline = true;
    }
    ansi_style
}

/// Write section text to buffer with shell escape codes specifying foreground and background color.
pub fn paint_text(text: &str, style: Style, output_buffer: &mut String, true_color: bool) {
    if text.is_empty() {
        return;
    }
    let ansi_style = to_ansi_style(style, true_color);
    output_buffer.push_str(&ansi_style.paint(text).to_string());
}

/// Return text together with shell escape codes specifying the foreground color.
pub fn paint_text_foreground(text: &str, color: Color, true_color: bool) -> String {
    to_ansi_color(color, true_color).paint(text).to_string()
}

#[allow(dead_code)]
pub fn paint_text_background(text: &str, color: Color, true_color: bool) -> String {
    let style = ansi_term::Style::new().on(to_ansi_color(color, true_color));
    style.paint(text).to_string()
}

// See
// https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
pub fn ansi_color_name_to_number(name: &str) -> Option<u8> {
    match name.to_lowercase().as_ref() {
        "black" => Some(0),
        "red" => Some(1),
        "green" => Some(2),
        "yellow" => Some(3),
        "blue" => Some(4),
        "magenta" => Some(5),
        "purple" => Some(5),
        "cyan" => Some(6),
        "white" => Some(7),
        "bright-black" => Some(8),
        "bright-red" => Some(9),
        "bright-green" => Some(10),
        "bright-yellow" => Some(11),
        "bright-blue" => Some(12),
        "bright-magenta" => Some(13),
        "bright-purple" => Some(13),
        "bright-cyan" => Some(14),
        "bright-white" => Some(15),
        _ => None,
    }
}

pub fn color_from_ansi_name(name: &str) -> Option<Color> {
    ansi_color_name_to_number(name).and_then(color_from_ansi_number)
}

/// Convert 8-bit ANSI code to #RGBA string with ANSI code in red channel and 0 in alpha channel.
// See https://github.com/sharkdp/bat/pull/543
pub fn color_from_ansi_number(n: u8) -> Option<Color> {
    Color::from_str(&format!("#{:02x}000000", n)).ok()
}

mod superimpose_style_sections {
    use syntect::highlighting::{Style, StyleModifier};

    pub fn superimpose_style_sections(
        sections_1: &[(Style, &str)],
        sections_2: &[(StyleModifier, &str)],
    ) -> Vec<(Style, String)> {
        coalesce(superimpose(
            explode(sections_1)
                .iter()
                .zip(explode(sections_2))
                .collect::<Vec<(&(Style, char), (StyleModifier, char))>>(),
        ))
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
        style_section_pairs: Vec<(&(Style, char), (StyleModifier, char))>,
    ) -> Vec<(Style, char)> {
        let mut superimposed: Vec<(Style, char)> = Vec::new();
        for ((style, char_1), (modifier, char_2)) in style_section_pairs {
            if *char_1 != char_2 {
                panic!(
                    "String mismatch encountered while superimposing style sections: '{}' vs '{}'",
                    *char_1, char_2
                )
            }
            superimposed.push((style.apply(modifier), *char_1));
        }
        superimposed
    }

    fn coalesce(style_sections: Vec<(Style, char)>) -> Vec<(Style, String)> {
        let mut coalesced: Vec<(Style, String)> = Vec::new();
        let mut style_sections = style_sections.iter();
        if let Some((style, c)) = style_sections.next() {
            let mut current_string = c.to_string();
            let mut current_style = style;
            for (style, c) in style_sections {
                if style != current_style {
                    coalesced.push((*current_style, current_string));
                    current_string = String::new();
                    current_style = style;
                }
                current_string.push(*c);
            }

            // TODO: This is not the ideal location for the following code.
            if current_string.ends_with("\n") {
                // Remove the terminating newline whose presence was necessary for the syntax
                // highlighter to work correctly.
                current_string.truncate(current_string.len() - 1);
            }

            coalesced.push((*current_style, current_string));
        }
        coalesced
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use syntect::highlighting::{Color, FontStyle, Style, StyleModifier};

        const STYLE: Style = Style {
            foreground: Color::BLACK,
            background: Color::BLACK,
            font_style: FontStyle::BOLD,
        };
        const STYLE_MODIFIER: StyleModifier = StyleModifier {
            foreground: Some(Color::WHITE),
            background: Some(Color::WHITE),
            font_style: Some(FontStyle::UNDERLINE),
        };
        const SUPERIMPOSED_STYLE: Style = Style {
            foreground: Color::WHITE,
            background: Color::WHITE,
            font_style: FontStyle::UNDERLINE,
        };

        #[test]
        fn test_superimpose_style_sections_1() {
            let sections_1 = vec![(STYLE, "ab")];
            let sections_2 = vec![(STYLE_MODIFIER, "ab")];
            let superimposed = vec![(SUPERIMPOSED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_2() {
            let sections_1 = vec![(STYLE, "ab")];
            let sections_2 = vec![(STYLE_MODIFIER, "a"), (STYLE_MODIFIER, "b")];
            let superimposed = vec![(SUPERIMPOSED_STYLE, String::from("ab"))];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2),
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
            let x = (STYLE, 'a');
            let pairs = vec![(&x, (STYLE_MODIFIER, 'a'))];
            assert_eq!(superimpose(pairs), vec![(SUPERIMPOSED_STYLE, 'a')]);
        }
    }
}
