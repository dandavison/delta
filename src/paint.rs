use std::io::Write;
use std::str::FromStr;
// TODO: Functions in this module should return Result and use ? syntax.

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, StyleModifier, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::paint::superimpose_style_sections::superimpose_style_sections;

pub const LIGHT_THEMES: [&str; 4] = [
    "GitHub",
    "Monokai Extended Light",
    "OneHalfLight",
    "ansi-light",
];

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

const LIGHT_THEME_PLUS_COLOR: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0xff,
};

const LIGHT_THEME_MINUS_COLOR: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0xff,
};

const DARK_THEME_PLUS_COLOR: Color = Color {
    r: 0x01,
    g: 0x3B,
    b: 0x01,
    a: 0xff,
};

const DARK_THEME_MINUS_COLOR: Color = Color {
    r: 0x3F,
    g: 0x00,
    b: 0x01,
    a: 0xff,
};

pub const NULL_STYLE_MODIFIER: StyleModifier = StyleModifier {
    foreground: None,
    background: None,
    font_style: None,
};

pub struct Config<'a> {
    pub theme: &'a Theme,
    pub minus_style_modifier: StyleModifier,
    pub plus_style_modifier: StyleModifier,
    pub syntax_set: &'a SyntaxSet,
    pub width: Option<usize>,
    pub highlight_removed: bool,
    pub pager: &'a str,
}

pub fn get_config<'a>(
    syntax_set: &'a SyntaxSet,
    theme: &Option<String>,
    theme_set: &'a ThemeSet,
    user_requests_theme_for_light_terminal_background: bool,
    minus_color: &Option<String>,
    plus_color: &Option<String>,
    highlight_removed: bool, // TODO: honor
    width: Option<usize>,
) -> Config<'a> {
    let theme_name = match theme {
        Some(ref theme) => theme,
        None => match user_requests_theme_for_light_terminal_background {
            true => "GitHub",
            false => "Monokai Extended",
        },
    };
    let is_light_theme = LIGHT_THEMES.contains(&theme_name);

    let minus_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            minus_color,
            is_light_theme,
            LIGHT_THEME_MINUS_COLOR,
            DARK_THEME_MINUS_COLOR,
        )),
        foreground: None,
        font_style: None,
    };

    let plus_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            plus_color,
            is_light_theme,
            LIGHT_THEME_PLUS_COLOR,
            DARK_THEME_PLUS_COLOR,
        )),
        foreground: None,
        font_style: None,
    };

    Config {
        theme: &theme_set.themes[theme_name],
        minus_style_modifier: minus_style_modifier,
        plus_style_modifier: plus_style_modifier,
        width: width,
        highlight_removed: highlight_removed,
        syntax_set: &syntax_set,
        pager: "less",
    }
}

fn color_from_arg(
    arg: &Option<String>,
    is_light_theme: bool,
    light_theme_default: Color,
    dark_theme_default: Color,
) -> Color {
    arg.as_ref()
        .and_then(|s| Color::from_str(s).ok())
        .unwrap_or_else(|| {
            if is_light_theme {
                light_theme_default
            } else {
                dark_theme_default
            }
        })
}

pub struct Painter<'a> {
    pub minus_lines: Vec<String>,
    pub plus_lines: Vec<String>,

    // TODO: store slice references instead of creating Strings
    pub minus_line_style_sections: Vec<Vec<(StyleModifier, String)>>,
    pub plus_line_style_sections: Vec<Vec<(StyleModifier, String)>>,

    pub writer: &'a mut Write,
    pub syntax: Option<&'a SyntaxReference>,
    pub config: &'a Config<'a>,
    pub output_buffer: String,
}

impl<'a> Painter<'a> {
    pub fn paint_buffered_lines(&mut self) {
        self.set_background_style_sections();
        // TODO: lines and style sections contain identical line text
        if self.minus_lines.len() > 0 {
            self.paint_lines(
                // TODO: don't clone
                self.minus_lines.iter().cloned().collect(),
                self.minus_line_style_sections.iter().cloned().collect(),
            );
            self.minus_lines.clear();
            self.minus_line_style_sections.clear();
        }
        if self.plus_lines.len() > 0 {
            self.paint_lines(
                // TODO: don't clone
                self.plus_lines.iter().cloned().collect(),
                self.plus_line_style_sections.iter().cloned().collect(),
            );
            self.plus_lines.clear();
            self.plus_line_style_sections.clear();
        }
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    pub fn paint_lines(
        &mut self,
        lines: Vec<String>,
        line_style_sections: Vec<Vec<(StyleModifier, String)>>,
    ) {
        let mut highlighter = HighlightLines::new(self.syntax.unwrap(), self.config.theme);

        for (line, style_sections) in lines.iter().zip(line_style_sections) {
            // TODO:
            // 1. pad right
            // 2. remove +- in first column
            let syntax_highlighting_style_sections: Vec<(Style, String)> = highlighter
                .highlight(&line, &self.config.syntax_set)
                .iter()
                .map(|(style, s)| (*style, s.to_string()))
                .collect::<Vec<(Style, String)>>();
            let superimposed_style_sections =
                superimpose_style_sections(syntax_highlighting_style_sections, style_sections);
            for (style, text) in superimposed_style_sections {
                paint_section(&text, style, &mut self.output_buffer).unwrap();
            }
            self.output_buffer.push_str("\n");
        }
    }

    /// Write output buffer to output stream, and clear the buffer.
    pub fn emit(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.truncate(0);
        Ok(())
    }

    /// Set background styles for minus and plus lines in buffer.
    fn set_background_style_sections(&mut self) {
        for line in self.minus_lines.iter() {
            self.minus_line_style_sections
                .push(vec![(self.config.minus_style_modifier, line.to_string())]);
        }
        for line in self.plus_lines.iter() {
            self.plus_line_style_sections
                .push(vec![(self.config.plus_style_modifier, line.to_string())]);
        }
    }
}

/// Write section text to buffer with color escape codes.
fn paint_section(text: &str, style: Style, output_buffer: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(
        output_buffer,
        "\x1b[48;2;{};{};{}m",
        style.background.r, style.background.g, style.background.b
    )?;
    write!(
        output_buffer,
        "\x1b[38;2;{};{};{}m{}",
        style.foreground.r, style.foreground.g, style.foreground.b, text
    )?;
    Ok(())
}

mod superimpose_style_sections {
    use syntect::highlighting::{Style, StyleModifier};

    pub fn superimpose_style_sections(
        sections_1: Vec<(Style, String)>,
        sections_2: Vec<(StyleModifier, String)>,
    ) -> Vec<(Style, String)> {
        coalesce(superimpose(
            explode(sections_1)
                .iter()
                .zip(explode(sections_2))
                .collect::<Vec<(&(Style, char), (StyleModifier, char))>>(),
        ))
    }

    fn explode<T>(style_sections: Vec<(T, String)>) -> Vec<(T, char)>
    where
        T: Copy,
    {
        let mut exploded: Vec<(T, char)> = Vec::new();
        for (style, string) in style_sections {
            for c in string.chars() {
                exploded.push((style, c));
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
        match style_sections.next() {
            Some((style, c)) => {
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
            None => (),
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
            let string = String::from("ab");
            let sections_1 = vec![(STYLE, string.clone())];
            let sections_2 = vec![(STYLE_MODIFIER, string.clone())];
            let superimposed = vec![(SUPERIMPOSED_STYLE, string.clone())];
            assert_eq!(
                superimpose_style_sections(sections_1, sections_2),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_2() {
            let sections_1 = vec![(STYLE, String::from("ab"))];
            let sections_2 = vec![
                (STYLE_MODIFIER, String::from("a")),
                (STYLE_MODIFIER, String::from("b")),
            ];
            let superimposed = vec![(SUPERIMPOSED_STYLE, String::from("ab"))];
            assert_eq!(
                superimpose_style_sections(sections_1, sections_2),
                superimposed
            );
        }

        #[test]
        fn test_explode() {
            let arbitrary = 0;
            let string = String::from("ab");
            assert_eq!(
                explode(vec![(arbitrary, string)]),
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
