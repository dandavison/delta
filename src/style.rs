use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};

use lazy_static::lazy_static;

use crate::ansi;
use crate::color;
use crate::git_config::GitConfig;

// PERF: Avoid deriving Copy here?
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Style {
    pub ansi_term_style: ansi_term::Style,
    pub is_emph: bool,
    pub is_omitted: bool,
    pub is_raw: bool,
    pub is_syntax_highlighted: bool,
    pub decoration_style: DecorationStyle,
}

// More compact debug output, replace false/empty with lowercase and true with uppercase.
impl fmt::Debug for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ansi = if self.ansi_term_style.is_plain() {
            "<a".into()
        } else {
            format!("ansi_term_style: {:?}, <", self.ansi_term_style)
        };

        let deco = if self.decoration_style == DecorationStyle::NoDecoration {
            "d>".into()
        } else {
            format!(">, decoration_style: {:?}", self.decoration_style)
        };

        let is_set = |c: char, set: bool| -> String {
            if set {
                c.to_uppercase().to_string()
            } else {
                c.to_lowercase().to_string()
            }
        };

        write!(
            f,
            "Style {{ {}{}{}{}{}{} }}",
            ansi,
            is_set('e', self.is_emph),
            is_set('o', self.is_omitted),
            is_set('r', self.is_raw),
            is_set('s', self.is_syntax_highlighted),
            deco
        )
    }
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

impl Default for DecorationStyle {
    fn default() -> Self {
        Self::NoDecoration
    }
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

/// Interpret `color_string` as a color specifier and return it painted accordingly.
pub fn paint_color_string<'a>(
    color_string: &'a str,
    true_color: bool,
    git_config: Option<&GitConfig>,
) -> ansi_term::ANSIGenericString<'a, str> {
    if let Some(color) = color::parse_color(color_string, true_color, git_config) {
        let style = ansi_term::Style {
            background: Some(color),
            ..ansi_term::Style::default()
        };
        style.paint(color_string)
    } else {
        ansi_term::ANSIGenericString::from(color_string)
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

// TODO: The equality methods were implemented first, and the equality_key
// methods later. The former should be re-implemented in terms of the latter.
// But why did the former not address equality of ansi_term::Color::RGB values?
#[derive(Clone)]
pub struct AnsiTermStyleEqualityKey {
    attrs_key: (bool, bool, bool, bool, bool, bool, bool, bool),
    foreground_key: Option<(u8, u8, u8, u8)>,
    background_key: Option<(u8, u8, u8, u8)>,
}

impl PartialEq for AnsiTermStyleEqualityKey {
    fn eq(&self, other: &Self) -> bool {
        let option_eq = |opt_a, opt_b| match (opt_a, opt_b) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };

        if self.attrs_key != other.attrs_key {
            false
        } else {
            option_eq(self.foreground_key, other.foreground_key)
                && option_eq(self.background_key, other.background_key)
        }
    }
}

impl Eq for AnsiTermStyleEqualityKey {}

impl Hash for AnsiTermStyleEqualityKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.attrs_key.hash(state);
        self.foreground_key.hash(state);
        self.background_key.hash(state);
    }
}

pub fn ansi_term_style_equality_key(style: ansi_term::Style) -> AnsiTermStyleEqualityKey {
    let attrs_key = (
        style.is_bold,
        style.is_dimmed,
        style.is_italic,
        style.is_underline,
        style.is_blink,
        style.is_reverse,
        style.is_hidden,
        style.is_strikethrough,
    );
    AnsiTermStyleEqualityKey {
        attrs_key,
        foreground_key: style.foreground.map(ansi_term_color_equality_key),
        background_key: style.background.map(ansi_term_color_equality_key),
    }
}

impl fmt::Debug for AnsiTermStyleEqualityKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let is_set = |c: char, set: bool| -> String {
            if set {
                c.to_uppercase().to_string()
            } else {
                c.to_lowercase().to_string()
            }
        };

        let (bold, dimmed, italic, underline, blink, reverse, hidden, strikethrough) =
            self.attrs_key;
        write!(
            f,
            "ansi_term::Style {{ {:?} {:?} {}{}{}{}{}{}{}{} }}",
            self.foreground_key,
            self.background_key,
            is_set('b', bold),
            is_set('d', dimmed),
            is_set('i', italic),
            is_set('u', underline),
            is_set('l', blink),
            is_set('r', reverse),
            is_set('h', hidden),
            is_set('s', strikethrough),
        )
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

fn ansi_term_color_equality_key(color: ansi_term::Color) -> (u8, u8, u8, u8) {
    // Same (r, g, b, a) encoding as in utils::bat::terminal::to_ansi_color.
    // When a = 0xFF, then a 256-color number is stored in the red channel, and
    // the green and blue channels are meaningless. But a=0 signifies an RGB
    // color.
    let default = 0xFF;
    match color {
        ansi_term::Color::Fixed(0) | ansi_term::Color::Black => (0, default, default, default),
        ansi_term::Color::Fixed(1) | ansi_term::Color::Red => (1, default, default, default),
        ansi_term::Color::Fixed(2) | ansi_term::Color::Green => (2, default, default, default),
        ansi_term::Color::Fixed(3) | ansi_term::Color::Yellow => (3, default, default, default),
        ansi_term::Color::Fixed(4) | ansi_term::Color::Blue => (4, default, default, default),
        ansi_term::Color::Fixed(5) | ansi_term::Color::Purple => (5, default, default, default),
        ansi_term::Color::Fixed(6) | ansi_term::Color::Cyan => (6, default, default, default),
        ansi_term::Color::Fixed(7) | ansi_term::Color::White => (7, default, default, default),
        ansi_term::Color::Fixed(n) => (n, default, default, default),
        ansi_term::Color::RGB(r, g, b) => (r, g, b, 0),
    }
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

pub fn line_has_style_other_than(line: &str, styles: &[Style]) -> bool {
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
        assert!(!line_has_style_other_than("", &[]));
        assert!(!line_has_style_other_than("", &[*GIT_DEFAULT_MINUS_STYLE]));

        // Lines from git should test negative when corresponding default is supplied
        assert!(!line_has_style_other_than(
            minus_line_from_unconfigured_git,
            &[*GIT_DEFAULT_MINUS_STYLE]
        ));
        assert!(!line_has_style_other_than(
            plus_line_from_unconfigured_git,
            &[*GIT_DEFAULT_PLUS_STYLE]
        ));

        // Styled lines should test positive when unless their style is supplied.
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            &[*GIT_DEFAULT_PLUS_STYLE]
        ));
        assert!(line_has_style_other_than(
            minus_line_from_unconfigured_git,
            &[]
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            &[*GIT_DEFAULT_MINUS_STYLE]
        ));
        assert!(line_has_style_other_than(
            plus_line_from_unconfigured_git,
            &[]
        ));
    }

    #[test]
    fn test_style_compact_debug_fmt() {
        let mut s = Style::new();
        assert_eq!(format!("{:?}", s), "Style { <aeorsd> }");
        s.is_emph = true;
        assert_eq!(format!("{:?}", s), "Style { <aEorsd> }");
        s.ansi_term_style = ansi_term::Style::new().bold();
        assert_eq!(
            format!("{:?}", s),
            "Style { ansi_term_style: Style { bold }, <Eorsd> }"
        );
        s.decoration_style = DecorationStyle::Underline(s.ansi_term_style.clone());
        assert_eq!(
            format!("{:?}", s),
            "Style { ansi_term_style: Style { bold }, <Eors>, \
                  decoration_style: Underline(Style { bold }) }"
        );
        s.ansi_term_style = ansi_term::Style::default();
        assert_eq!(
            format!("{:?}", s),
            "Style { <aEors>, decoration_style: Underline(Style { bold }) }"
        );
    }
}
