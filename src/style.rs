use std::borrow::Cow;
use std::fmt;

use lazy_static::lazy_static;

use crate::ansi;
use crate::color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub ansi_term_style: ansi_term::Style,
    pub is_emph: bool,
    pub is_omitted: bool,
    pub is_raw: bool,
    pub is_syntax_highlighted: bool,
    pub decoration_style: DecorationStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecorationStyle {
    Box(ansi_term::Style),
    Underline(ansi_term::Style),
    Overline(ansi_term::Style),
    UnderOverline(ansi_term::Style),
    BoxWithUnderline(ansi_term::Style),
    BoxWithOverline(ansi_term::Style),
    BoxWithUnderOverline(ansi_term::Style),
    NoDecoration,
}

impl Style {
    pub fn new() -> Self {
        Self {
            ansi_term_style: ansi_term::Style::new(),
            is_emph: false,
            is_omitted: false,
            is_raw: false,
            is_syntax_highlighted: false,
            decoration_style: DecorationStyle::NoDecoration,
        }
    }

    pub fn from_colors(
        foreground: Option<ansi_term::Color>,
        background: Option<ansi_term::Color>,
    ) -> Self {
        Self {
            ansi_term_style: ansi_term::Style {
                foreground,
                background,
                ..ansi_term::Style::new()
            },
            ..Self::new()
        }
    }

    pub fn paint<'a, I, S: 'a + ToOwned + ?Sized>(
        self,
        input: I,
    ) -> ansi_term::ANSIGenericString<'a, S>
    where
        I: Into<Cow<'a, S>>,
        <S as ToOwned>::Owned: fmt::Debug,
    {
        self.ansi_term_style.paint(input)
    }

    pub fn get_background_color(&self) -> Option<ansi_term::Color> {
        if self.ansi_term_style.is_reverse {
            self.ansi_term_style.foreground
        } else {
            self.ansi_term_style.background
        }
    }

    pub fn is_applied_to(&self, s: &str) -> bool {
        match ansi::parse_first_style(s) {
            Some(parsed_style) => ansi_term_style_equality(parsed_style, self.ansi_term_style),
            None => false,
        }
    }

    pub fn to_painted_string(self) -> ansi_term::ANSIGenericString<'static, str> {
        self.paint(self.to_string())
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_raw {
            return write!(f, "raw");
        }
        let mut words = Vec::<String>::new();
        if self.is_omitted {
            words.push("omit".to_string());
        }
        if self.ansi_term_style.is_blink {
            words.push("blink".to_string());
        }
        if self.ansi_term_style.is_bold {
            words.push("bold".to_string());
        }
        if self.ansi_term_style.is_dimmed {
            words.push("dim".to_string());
        }
        if self.ansi_term_style.is_italic {
            words.push("italic".to_string());
        }
        if self.ansi_term_style.is_reverse {
            words.push("reverse".to_string());
        }
        if self.ansi_term_style.is_strikethrough {
            words.push("strike".to_string());
        }
        if self.ansi_term_style.is_underline {
            words.push("ul".to_string());
        }

        match (self.is_syntax_highlighted, self.ansi_term_style.foreground) {
            (true, _) => words.push("syntax".to_string()),
            (false, Some(color)) => {
                words.push(color::color_to_string(color));
            }
            (false, None) => words.push("normal".to_string()),
        }
        if let Some(color) = self.ansi_term_style.background {
            words.push(color::color_to_string(color))
        }
        let style_str = words.join(" ");
        write!(f, "{}", style_str)
    }
}

pub fn ansi_term_style_equality(a: ansi_term::Style, b: ansi_term::Style) -> bool {
    let a_attrs = ansi_term::Style {
        foreground: None,
        background: None,
        ..a
    };
    let b_attrs = ansi_term::Style {
        foreground: None,
        background: None,
        ..b
    };
    if a_attrs != b_attrs {
        false
    } else {
        ansi_term_color_equality(a.foreground, b.foreground)
            & ansi_term_color_equality(a.background, b.background)
    }
}

fn ansi_term_color_equality(a: Option<ansi_term::Color>, b: Option<ansi_term::Color>) -> bool {
    match (a, b) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(_), None) => false,
        (Some(a), Some(b)) => {
            if a == b {
                true
            } else {
                ansi_term_16_color_equality(a, b) || ansi_term_16_color_equality(b, a)
            }
        }
    }
}

fn ansi_term_16_color_equality(a: ansi_term::Color, b: ansi_term::Color) -> bool {
    matches!(
        (a, b),
        (ansi_term::Color::Fixed(0), ansi_term::Color::Black)
            | (ansi_term::Color::Fixed(1), ansi_term::Color::Red)
            | (ansi_term::Color::Fixed(2), ansi_term::Color::Green)
            | (ansi_term::Color::Fixed(3), ansi_term::Color::Yellow)
            | (ansi_term::Color::Fixed(4), ansi_term::Color::Blue)
            | (ansi_term::Color::Fixed(5), ansi_term::Color::Purple)
            | (ansi_term::Color::Fixed(6), ansi_term::Color::Cyan)
            | (ansi_term::Color::Fixed(7), ansi_term::Color::White)
    )
}

lazy_static! {
    pub static ref GIT_DEFAULT_MINUS_STYLE: Style = Style {
        ansi_term_style: ansi_term::Color::Red.normal(),
        ..Style::new()
    };
    pub static ref GIT_DEFAULT_PLUS_STYLE: Style = Style {
        ansi_term_style: ansi_term::Color::Green.normal(),
        ..Style::new()
    };
}

pub fn line_has_style_other_than<'a>(line: &str, styles: impl Iterator<Item = &'a Style>) -> bool {
    if !ansi::string_starts_with_ansi_style_sequence(line) {
        return false;
    }
    for style in styles {
        if style.is_applied_to(line) {
            return false;
        }
    }
    true
}

#[cfg(test)]
pub mod tests {

    use super::*;

    // To add to these tests:
    // 1. Stage a file with a single line containing the string "text"
    // 2. git -c 'color.diff.new = $STYLE_STRING' diff --cached  --color=always  | cat -A

    lazy_static! {
        pub static ref GIT_STYLE_STRING_EXAMPLES: Vec<(&'static str, &'static str)> = vec![
            // <git-default>                    "\x1b[32m+\x1b[m\x1b[32mtext\x1b[m\n"
            ("0",                               "\x1b[30m+\x1b[m\x1b[30mtext\x1b[m\n"),
            ("black",                           "\x1b[30m+\x1b[m\x1b[30mtext\x1b[m\n"),
            ("1",                               "\x1b[31m+\x1b[m\x1b[31mtext\x1b[m\n"),
            ("red",                             "\x1b[31m+\x1b[m\x1b[31mtext\x1b[m\n"),
            ("0 1",                             "\x1b[30;41m+\x1b[m\x1b[30;41mtext\x1b[m\n"),
            ("black red",                       "\x1b[30;41m+\x1b[m\x1b[30;41mtext\x1b[m\n"),
            ("19",                              "\x1b[38;5;19m+\x1b[m\x1b[38;5;19mtext\x1b[m\n"),
            ("black 19",                        "\x1b[30;48;5;19m+\x1b[m\x1b[30;48;5;19mtext\x1b[m\n"),
            ("19 black",                        "\x1b[38;5;19;40m+\x1b[m\x1b[38;5;19;40mtext\x1b[m\n"),
            ("19 20",                           "\x1b[38;5;19;48;5;20m+\x1b[m\x1b[38;5;19;48;5;20mtext\x1b[m\n"),
            ("#aabbcc",                         "\x1b[38;2;170;187;204m+\x1b[m\x1b[38;2;170;187;204mtext\x1b[m\n"),
            ("0 #aabbcc",                       "\x1b[30;48;2;170;187;204m+\x1b[m\x1b[30;48;2;170;187;204mtext\x1b[m\n"),
            ("#aabbcc 0",                       "\x1b[38;2;170;187;204;40m+\x1b[m\x1b[38;2;170;187;204;40mtext\x1b[m\n"),
            ("19 #aabbcc",                      "\x1b[38;5;19;48;2;170;187;204m+\x1b[m\x1b[38;5;19;48;2;170;187;204mtext\x1b[m\n"),
            ("#aabbcc 19",                      "\x1b[38;2;170;187;204;48;5;19m+\x1b[m\x1b[38;2;170;187;204;48;5;19mtext\x1b[m\n"),
            ("#aabbcc #ddeeff" ,                "\x1b[38;2;170;187;204;48;2;221;238;255m+\x1b[m\x1b[38;2;170;187;204;48;2;221;238;255mtext\x1b[m\n"),
            ("bold #aabbcc #ddeeff" ,           "\x1b[1;38;2;170;187;204;48;2;221;238;255m+\x1b[m\x1b[1;38;2;170;187;204;48;2;221;238;255mtext\x1b[m\n"),
            ("bold #aabbcc ul #ddeeff" ,        "\x1b[1;4;38;2;170;187;204;48;2;221;238;255m+\x1b[m\x1b[1;4;38;2;170;187;204;48;2;221;238;255mtext\x1b[m\n"),
            ("bold #aabbcc ul #ddeeff strike" , "\x1b[1;4;9;38;2;170;187;204;48;2;221;238;255m+\x1b[m\x1b[1;4;9;38;2;170;187;204;48;2;221;238;255mtext\x1b[m\n"),
            ("bold 0 ul 1 strike",              "\x1b[1;4;9;30;41m+\x1b[m\x1b[1;4;9;30;41mtext\x1b[m\n"),
            ("bold 0 ul 19 strike",             "\x1b[1;4;9;30;48;5;19m+\x1b[m\x1b[1;4;9;30;48;5;19mtext\x1b[m\n"),
            ("bold 19 ul 0 strike",             "\x1b[1;4;9;38;5;19;40m+\x1b[m\x1b[1;4;9;38;5;19;40mtext\x1b[m\n"),
            ("bold #aabbcc ul 0 strike",        "\x1b[1;4;9;38;2;170;187;204;40m+\x1b[m\x1b[1;4;9;38;2;170;187;204;40mtext\x1b[m\n"),
            ("bold #aabbcc ul 19 strike" ,      "\x1b[1;4;9;38;2;170;187;204;48;5;19m+\x1b[m\x1b[1;4;9;38;2;170;187;204;48;5;19mtext\x1b[m\n"),
            ("bold 19 ul #aabbcc strike" ,      "\x1b[1;4;9;38;5;19;48;2;170;187;204m+\x1b[m\x1b[1;4;9;38;5;19;48;2;170;187;204mtext\x1b[m\n"),
            ("bold 0 ul #aabbcc strike",        "\x1b[1;4;9;30;48;2;170;187;204m+\x1b[m\x1b[1;4;9;30;48;2;170;187;204mtext\x1b[m\n"),
            (r##"black "#ddeeff""##,            "\x1b[30;48;2;221;238;255m+\x1b[m\x1b[30;48;2;221;238;255mtext\x1b[m\n"),
            ("brightred",                       "\x1b[91m+\x1b[m\x1b[91mtext\x1b[m\n"),
            ("normal",                          "\x1b[mtext\x1b[m\n"),
            ("blink",                           "\x1b[5m+\x1b[m\x1b[5mtext\x1b[m\n"),
        ];
    }

    #[test]
    fn test_parse_git_style_string_and_ansi_code_iterator() {
        for (git_style_string, git_output) in &*GIT_STYLE_STRING_EXAMPLES {
            assert!(Style::from_git_str(git_style_string).is_applied_to(git_output));
        }
    }

    #[test]
    fn test_is_applied_to_negative_assertion() {
        let style_string_from_24 = "bold #aabbcc ul 19 strike";
        let git_output_from_25 = "\x1b[1;4;9;38;5;19;48;2;170;187;204m+\x1b[m\x1b[1;4;9;38;5;19;48;2;170;187;204mtext\x1b[m\n";
        assert!(!Style::from_git_str(style_string_from_24).is_applied_to(git_output_from_25));
    }

    #[test]
    fn test_git_default_styles() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let plus_line_from_unconfigured_git = "\x1b[32m+\x1b[m\x1b[32m____\x1b[m\n";
        assert!(GIT_DEFAULT_MINUS_STYLE.is_applied_to(minus_line_from_unconfigured_git));
        assert!(!GIT_DEFAULT_MINUS_STYLE.is_applied_to(plus_line_from_unconfigured_git));

        assert!(GIT_DEFAULT_PLUS_STYLE.is_applied_to(plus_line_from_unconfigured_git));
        assert!(!GIT_DEFAULT_PLUS_STYLE.is_applied_to(minus_line_from_unconfigured_git));
    }

    #[test]
    fn test_line_has_style_other_than() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let plus_line_from_unconfigured_git = "\x1b[32m+\x1b[m\x1b[32m____\x1b[m\n";

        // Unstyled lines should test negative, regardless of supplied styles.
        assert!(!line_has_style_other_than("", [].iter()));
        assert!(!line_has_style_other_than(
            "",
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));

        // Lines from git should test negative when corresponding default is supplied
        assert!(!line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));
        assert!(!line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [*GIT_DEFAULT_PLUS_STYLE].iter()
        ));

        // Styled lines should test positive when unless their style is supplied.
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [*GIT_DEFAULT_PLUS_STYLE].iter()
        ));
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            [].iter()
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [*GIT_DEFAULT_MINUS_STYLE].iter()
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            [].iter()
        ));
    }
}
