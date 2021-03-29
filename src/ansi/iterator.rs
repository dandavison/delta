use core::str::Bytes;
use std::convert::TryFrom;
use std::iter;
use vte::{Params, ParamsIter};

pub struct AnsiElementIterator<'a> {
    // The input bytes
    bytes: Bytes<'a>,

    // The state machine
    machine: vte::Parser,

    // Becomes non-None when the parser finishes parsing an ANSI sequence.
    // This is never Element::Text.
    element: Option<Element>,

    // Number of text bytes seen since the last element was emitted.
    text_length: usize,

    // Byte offset of start of current element.
    start: usize,

    // Byte offset of most rightward byte processed so far
    pos: usize,
}

struct Performer {
    // Becomes non-None when the parser finishes parsing an ANSI sequence.
    // This is never Element::Text.
    element: Option<Element>,

    // Number of text bytes seen since the last element was emitted.
    text_length: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Csi(ansi_term::Style, usize, usize),
    Esc(usize, usize),
    Osc(usize, usize),
    Text(usize, usize),
}

impl<'a> AnsiElementIterator<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            machine: vte::Parser::new(),
            bytes: s.bytes(),
            element: None,
            text_length: 0,
            start: 0,
            pos: 0,
        }
    }

    #[allow(dead_code)]
    pub fn dbg(s: &str) {
        for el in AnsiElementIterator::new(s) {
            match el {
                Element::Csi(_, i, j) => println!("CSI({}, {}, {:?})", i, j, &s[i..j]),
                Element::Esc(i, j) => println!("ESC({}, {}, {:?})", i, j, &s[i..j]),
                Element::Osc(i, j) => println!("OSC({}, {}, {:?})", i, j, &s[i..j]),
                Element::Text(i, j) => println!("Text({}, {}, {:?})", i, j, &s[i..j]),
            }
        }
    }
}

impl<'a> Iterator for AnsiElementIterator<'a> {
    type Item = Element;

    fn next(&mut self) -> Option<Element> {
        loop {
            // If the last element emitted was text, then there may be a non-text element waiting
            // to be emitted. In that case we do not consume a new byte.
            let byte = if self.element.is_some() {
                None
            } else {
                self.bytes.next()
            };
            if byte.is_some() || self.element.is_some() {
                if let Some(byte) = byte {
                    let mut performer = Performer {
                        element: None,
                        text_length: 0,
                    };
                    self.machine.advance(&mut performer, byte);
                    self.element = performer.element;
                    self.text_length += performer.text_length;
                    self.pos += 1;
                }
                if self.element.is_some() {
                    // There is a non-text element waiting to be emitted, but it may have preceding
                    // text, which must be emitted first.
                    if self.text_length > 0 {
                        let start = self.start;
                        self.start += self.text_length;
                        self.text_length = 0;
                        return Some(Element::Text(start, self.start));
                    }
                    let start = self.start;
                    self.start = self.pos;
                    let element = match self.element.as_ref().unwrap() {
                        Element::Csi(style, _, _) => Element::Csi(*style, start, self.pos),
                        Element::Esc(_, _) => Element::Esc(start, self.pos),
                        Element::Osc(_, _) => Element::Osc(start, self.pos),
                        Element::Text(_, _) => unreachable!(),
                    };
                    self.element = None;
                    return Some(element);
                }
            } else if self.text_length > 0 {
                self.text_length = 0;
                return Some(Element::Text(self.start, self.pos));
            } else {
                return None;
            }
        }
    }
}

// Based on https://github.com/alacritty/vte/blob/v0.9.0/examples/parselog.rs
impl vte::Perform for Performer {
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        if ignore || intermediates.len() > 1 {
            return;
        }

        if let ('m', None) = (c, intermediates.get(0)) {
            if params.is_empty() {
                // Attr::Reset
                // Probably doesn't need to be handled: https://github.com/dandavison/delta/pull/431#discussion_r536883568
            } else {
                self.element = Some(Element::Csi(
                    ansi_term_style_from_sgr_parameters(&mut params.iter()),
                    0,
                    0,
                ));
            }
        }
    }

    fn print(&mut self, c: char) {
        self.text_length += c.len_utf8();
    }

    fn execute(&mut self, byte: u8) {
        // E.g. '\n'
        if byte < 128 {
            self.text_length += 1;
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.element = Some(Element::Osc(0, 0));
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.element = Some(Element::Esc(0, 0));
    }
}

// Based on https://github.com/alacritty/alacritty/blob/9e71002e40d5487c6fa2571a3a3c4f5c8f679334/alacritty_terminal/src/ansi.rs#L1175
fn ansi_term_style_from_sgr_parameters(params: &mut ParamsIter<'_>) -> ansi_term::Style {
    let mut style = ansi_term::Style::new();
    while let Some(param) = params.next() {
        match param {
            // [0] => Some(Attr::Reset),
            [1] => style.is_bold = true,
            [2] => style.is_dimmed = true,
            [3] => style.is_italic = true,
            [4, ..] => style.is_underline = true,
            [5] => style.is_blink = true, // blink slow
            [6] => style.is_blink = true, // blink fast
            [7] => style.is_reverse = true,
            [8] => style.is_hidden = true,
            [9] => style.is_strikethrough = true,
            // [21] => Some(Attr::CancelBold),
            // [22] => Some(Attr::CancelBoldDim),
            // [23] => Some(Attr::CancelItalic),
            // [24] => Some(Attr::CancelUnderline),
            // [25] => Some(Attr::CancelBlink),
            // [27] => Some(Attr::CancelReverse),
            // [28] => Some(Attr::CancelHidden),
            // [29] => Some(Attr::CancelStrike),
            [30] => style.foreground = Some(ansi_term::Color::Black),
            [31] => style.foreground = Some(ansi_term::Color::Red),
            [32] => style.foreground = Some(ansi_term::Color::Green),
            [33] => style.foreground = Some(ansi_term::Color::Yellow),
            [34] => style.foreground = Some(ansi_term::Color::Blue),
            [35] => style.foreground = Some(ansi_term::Color::Purple),
            [36] => style.foreground = Some(ansi_term::Color::Cyan),
            [37] => style.foreground = Some(ansi_term::Color::White),
            [38] => {
                let mut iter = params.map(|param| param[0]);
                if let Some(color) = parse_sgr_color(&mut iter) {
                    style.foreground = Some(color);
                }
            }
            [38, params @ ..] => {
                let rgb_start = if params.len() > 4 { 2 } else { 1 };
                let rgb_iter = params[rgb_start..].iter().copied();
                let mut iter = iter::once(params[0]).chain(rgb_iter);

                if let Some(color) = parse_sgr_color(&mut iter) {
                    style.foreground = Some(color);
                }
            }
            // [39] => Some(Attr::Foreground(Color::Named(NamedColor::Foreground))),
            [40] => style.background = Some(ansi_term::Color::Black),
            [41] => style.background = Some(ansi_term::Color::Red),
            [42] => style.background = Some(ansi_term::Color::Green),
            [43] => style.background = Some(ansi_term::Color::Yellow),
            [44] => style.background = Some(ansi_term::Color::Blue),
            [45] => style.background = Some(ansi_term::Color::Purple),
            [46] => style.background = Some(ansi_term::Color::Cyan),
            [47] => style.background = Some(ansi_term::Color::White),
            [48] => {
                let mut iter = params.map(|param| param[0]);
                if let Some(color) = parse_sgr_color(&mut iter) {
                    style.background = Some(color);
                }
            }
            [48, params @ ..] => {
                let rgb_start = if params.len() > 4 { 2 } else { 1 };
                let rgb_iter = params[rgb_start..].iter().copied();
                let mut iter = iter::once(params[0]).chain(rgb_iter);
                if let Some(color) = parse_sgr_color(&mut iter) {
                    style.background = Some(color);
                }
            }
            // [49] => Some(Attr::Background(Color::Named(NamedColor::Background))),
            // "bright" colors. ansi_term doesn't offer a way to emit them as, e.g., 90m; instead
            // that would be 38;5;8.
            [90] => style.foreground = Some(ansi_term::Color::Fixed(8)),
            [91] => style.foreground = Some(ansi_term::Color::Fixed(9)),
            [92] => style.foreground = Some(ansi_term::Color::Fixed(10)),
            [93] => style.foreground = Some(ansi_term::Color::Fixed(11)),
            [94] => style.foreground = Some(ansi_term::Color::Fixed(12)),
            [95] => style.foreground = Some(ansi_term::Color::Fixed(13)),
            [96] => style.foreground = Some(ansi_term::Color::Fixed(14)),
            [97] => style.foreground = Some(ansi_term::Color::Fixed(15)),
            [100] => style.background = Some(ansi_term::Color::Fixed(8)),
            [101] => style.background = Some(ansi_term::Color::Fixed(9)),
            [102] => style.background = Some(ansi_term::Color::Fixed(10)),
            [103] => style.background = Some(ansi_term::Color::Fixed(11)),
            [104] => style.background = Some(ansi_term::Color::Fixed(12)),
            [105] => style.background = Some(ansi_term::Color::Fixed(13)),
            [106] => style.background = Some(ansi_term::Color::Fixed(14)),
            [107] => style.background = Some(ansi_term::Color::Fixed(15)),
            _ => {}
        };
    }
    style
}

// Based on https://github.com/alacritty/alacritty/blob/57c4ac9145a20fb1ae9a21102503458d3da06c7b/alacritty_terminal/src/ansi.rs#L1258
fn parse_sgr_color(params: &mut dyn Iterator<Item = u16>) -> Option<ansi_term::Color> {
    match params.next() {
        Some(2) => {
            let r = u8::try_from(params.next()?).ok()?;
            let g = u8::try_from(params.next()?).ok()?;
            let b = u8::try_from(params.next()?).ok()?;
            Some(ansi_term::Color::RGB(r, g, b))
        }
        Some(5) => Some(ansi_term::Color::Fixed(u8::try_from(params.next()?).ok()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {

    use super::{AnsiElementIterator, Element};
    use crate::style;

    #[test]
    fn test_iterator_parse_git_style_strings() {
        for (git_style_string, git_output) in &*style::tests::GIT_STYLE_STRING_EXAMPLES {
            let mut it = AnsiElementIterator::new(git_output);

            if *git_style_string == "normal" {
                // This one has a different pattern
                assert!(
                    matches!(it.next().unwrap(), Element::Csi(s, _, _) if s == ansi_term::Style::default())
                );
                assert!(
                    matches!(it.next().unwrap(), Element::Text(i, j) if &git_output[i..j] == "text")
                );
                assert!(
                    matches!(it.next().unwrap(), Element::Csi(s, _, _) if s == ansi_term::Style::default())
                );
                continue;
            }

            // First element should be a style
            let element = it.next().unwrap();
            match element {
                Element::Csi(style, _, _) => assert!(style::ansi_term_style_equality(
                    style,
                    style::Style::from_git_str(git_style_string).ansi_term_style
                )),
                _ => assert!(false),
            }

            // Second element should be text: "+"
            assert!(matches!(
                it.next().unwrap(),
                Element::Text(i, j) if &git_output[i..j] == "+"));

            // Third element is the reset style
            assert!(matches!(
                it.next().unwrap(),
                Element::Csi(s, _, _) if s == ansi_term::Style::default()));

            // Fourth element should be a style
            let element = it.next().unwrap();
            match element {
                Element::Csi(style, _, _) => assert!(style::ansi_term_style_equality(
                    style,
                    style::Style::from_git_str(git_style_string).ansi_term_style
                )),
                _ => assert!(false),
            }

            // Fifth element should be text: "text"
            assert!(matches!(
                it.next().unwrap(),
                Element::Text(i, j) if &git_output[i..j] == "text"));

            // Sixth element is the reset style
            assert!(matches!(
                it.next().unwrap(),
                Element::Csi(s, _, _) if s == ansi_term::Style::default()));

            assert!(matches!(
                it.next().unwrap(),
                Element::Text(i, j) if &git_output[i..j] == "\n"));

            assert!(it.next().is_none());
        }
    }

    #[test]
    fn test_iterator_1() {
        let minus_line = "\x1b[31m0123\x1b[m\n";
        let actual_elements: Vec<Element> = AnsiElementIterator::new(minus_line).collect();
        assert_eq!(
            actual_elements,
            vec![
                Element::Csi(
                    ansi_term::Style {
                        foreground: Some(ansi_term::Color::Red),
                        ..ansi_term::Style::default()
                    },
                    0,
                    5
                ),
                Element::Text(5, 9),
                Element::Csi(ansi_term::Style::default(), 9, 12),
                Element::Text(12, 13),
            ]
        );
        assert_eq!("0123", &minus_line[5..9]);
        assert_eq!("\n", &minus_line[12..13]);
    }

    #[test]
    fn test_iterator_2() {
        let minus_line = "\x1b[31m0123\x1b[m456\n";
        let actual_elements: Vec<Element> = AnsiElementIterator::new(minus_line).collect();
        assert_eq!(
            actual_elements,
            vec![
                Element::Csi(
                    ansi_term::Style {
                        foreground: Some(ansi_term::Color::Red),
                        ..ansi_term::Style::default()
                    },
                    0,
                    5
                ),
                Element::Text(5, 9),
                Element::Csi(ansi_term::Style::default(), 9, 12),
                Element::Text(12, 16),
            ]
        );
        assert_eq!("0123", &minus_line[5..9]);
        assert_eq!("456\n", &minus_line[12..16]);
    }

    #[test]
    fn test_iterator_styled_non_ascii() {
        let s = "\x1b[31mバー\x1b[0m";
        let actual_elements: Vec<Element> = AnsiElementIterator::new(s).collect();
        assert_eq!(
            actual_elements,
            vec![
                Element::Csi(
                    ansi_term::Style {
                        foreground: Some(ansi_term::Color::Red),
                        ..ansi_term::Style::default()
                    },
                    0,
                    5
                ),
                Element::Text(5, 11),
                Element::Csi(ansi_term::Style::default(), 11, 15),
            ]
        );
        assert_eq!("バー", &s[5..11]);
    }

    #[test]
    fn test_iterator_osc_hyperlinks_styled_non_ascii() {
        let s = "\x1b[38;5;4m\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b\\src/ansi/modバー.rs\x1b]8;;\x1b\\\x1b[0m\n";
        assert_eq!(&s[0..9], "\x1b[38;5;4m");
        assert_eq!(
            &s[9..58],
            "\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b"
        );
        assert_eq!(&s[58..59], "\\");
        assert_eq!(&s[59..80], "src/ansi/modバー.rs");
        assert_eq!(&s[80..86], "\x1b]8;;\x1b");
        assert_eq!(&s[86..87], "\\");
        assert_eq!(&s[87..91], "\x1b[0m");
        assert_eq!(&s[91..92], "\n");
        let actual_elements: Vec<Element> = AnsiElementIterator::new(s).collect();
        assert_eq!(
            actual_elements,
            vec![
                Element::Csi(
                    ansi_term::Style {
                        foreground: Some(ansi_term::Color::Fixed(4)),
                        ..ansi_term::Style::default()
                    },
                    0,
                    9
                ),
                Element::Osc(9, 58),
                Element::Esc(58, 59),
                Element::Text(59, 80),
                Element::Osc(80, 86),
                Element::Esc(86, 87),
                Element::Csi(ansi_term::Style::default(), 87, 91),
                Element::Text(91, 92),
            ]
        );
    }
}
