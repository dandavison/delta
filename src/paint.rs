use std::io::Write;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, StyleModifier};
use syntect::parsing::SyntaxReference;
use unicode_segmentation::UnicodeSegmentation;

use crate::bat::assets::HighlightingAssets;
use crate::config;
use crate::edits;
use crate::paint::superimpose_style_sections::superimpose_style_sections;
use crate::style;

pub struct Painter<'a> {
    pub minus_lines: Vec<String>,
    pub plus_lines: Vec<String>,
    pub writer: &'a mut Write,
    pub syntax: Option<&'a SyntaxReference>,
    pub highlighter: HighlightLines<'a>,
    pub config: &'a config::Config<'a>,
    pub output_buffer: String,
}

impl<'a> Painter<'a> {
    pub fn new(
        writer: &'a mut Write,
        config: &'a config::Config,
        assets: &'a HighlightingAssets,
    ) -> Self {
        let dummy_highlighter = HighlightLines::new(
            assets.syntax_set.find_syntax_by_extension("txt").unwrap(),
            &assets.theme_set.themes[style::DEFAULT_LIGHT_THEME],
        );
        Self {
            minus_lines: Vec::new(),
            plus_lines: Vec::new(),
            output_buffer: String::new(),
            syntax: None,
            highlighter: dummy_highlighter,
            writer,
            config,
        }
    }

    pub fn reset_highlighter(&mut self) {
        if let Some(theme) = self.config.theme {
            self.highlighter = HighlightLines::new(self.syntax.unwrap(), theme)
        };
    }

    pub fn paint_buffered_lines(&mut self) {
        let (minus_line_syntax_style_sections, plus_line_syntax_style_sections) =
            Self::get_syntax_style_sections(
                &self.minus_lines,
                &self.plus_lines,
                &mut self.highlighter,
                self.config,
            );
        let (minus_line_diff_style_sections, plus_line_diff_style_sections) =
            Self::get_diff_style_sections(&self.minus_lines, &self.plus_lines, self.config);
        // TODO: lines and style sections contain identical line text
        if !self.minus_lines.is_empty() {
            Painter::paint_lines(
                &mut self.output_buffer,
                minus_line_syntax_style_sections,
                minus_line_diff_style_sections,
                self.config,
                self.config.minus_style_modifier,
                true,
            );
        }
        if !self.plus_lines.is_empty() {
            Painter::paint_lines(
                &mut self.output_buffer,
                plus_line_syntax_style_sections,
                plus_line_diff_style_sections,
                self.config,
                self.config.plus_style_modifier,
                true,
            );
        }
        self.minus_lines.clear();
        self.plus_lines.clear();
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    pub fn paint_lines(
        output_buffer: &mut String,
        syntax_style_sections: Vec<Vec<(Style, &str)>>,
        diff_style_sections: Vec<Vec<(StyleModifier, &str)>>,
        config: &config::Config,
        background_style_modifier: StyleModifier,
        should_trim_newline_and_right_pad: bool,
    ) {
        use std::fmt::Write;
        for (syntax_sections, diff_sections) in
            syntax_style_sections.iter().zip(diff_style_sections.iter())
        {
            let mut text_width = 0;
            for (style, text) in superimpose_style_sections(syntax_sections, diff_sections) {
                paint_text(&text, style, output_buffer).unwrap();
                if config.width.is_some() {
                    text_width += text.graphemes(true).count();
                }
            }
            if should_trim_newline_and_right_pad {
                // Remove the terminating newline whose presence was necessary for the syntax
                // highlighter to work correctly.
                output_buffer.truncate(output_buffer.len() - 1);
                // Right pad with background-highlighted white space.
                match config.width {
                    Some(width) if width > text_width => {
                        // Right pad to requested width with spaces.
                        let background_style = config.no_style.apply(background_style_modifier);
                        paint_text(
                            &" ".repeat(width - text_width),
                            background_style,
                            output_buffer,
                        )
                        .unwrap();
                    }
                    _ => (),
                }
            }
            writeln!(output_buffer).unwrap();
        }
    }

    /// Write output buffer to output stream, and clear the buffer.
    pub fn emit(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.clear();
        Ok(())
    }

    /// Perform syntax highlighting for minus and plus lines in buffer.
    fn get_syntax_style_sections<'m, 'p>(
        minus_lines: &'m [String],
        plus_lines: &'p [String],
        highlighter: &mut HighlightLines,
        config: &config::Config,
    ) -> (Vec<Vec<(Style, &'m str)>>, Vec<Vec<(Style, &'p str)>>) {
        let mut minus_line_sections = Vec::new();
        for line in minus_lines.iter() {
            minus_line_sections.push(Painter::get_line_syntax_style_sections(
                &line,
                highlighter,
                &config,
                config.opt.highlight_removed,
            ));
        }
        let mut plus_line_sections = Vec::new();
        for line in plus_lines.iter() {
            plus_line_sections.push(Painter::get_line_syntax_style_sections(
                &line,
                highlighter,
                &config,
                true,
            ));
        }
        (minus_line_sections, plus_line_sections)
    }

    pub fn get_line_syntax_style_sections(
        line: &'a str,
        highlighter: &mut HighlightLines,
        config: &config::Config,
        should_syntax_highlight: bool,
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
            config.opt.max_line_distance,
        )
    }
}

/// Write section text to buffer with color escape codes.
pub fn paint_text(text: &str, style: Style, output_buffer: &mut String) -> std::fmt::Result {
    use std::fmt::Write;

    if text.is_empty() {
        return Ok(());
    }

    match style.background {
        style::NO_COLOR => (),
        _ => write!(
            output_buffer,
            "\x1b[48;2;{};{};{}m",
            style.background.r, style.background.g, style.background.b
        )?,
    }
    match style.foreground {
        style::NO_COLOR => write!(output_buffer, "{}", text)?,
        _ => write!(
            output_buffer,
            "\x1b[38;2;{};{};{}m{}",
            style.foreground.r, style.foreground.g, style.foreground.b, text
        )?,
    };
    Ok(())
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
