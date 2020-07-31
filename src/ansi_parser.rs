use ansi_term;
use vte;

struct Parser {
    machine: vte::Parser,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            machine: vte::Parser::new(),
        }
    }
}

impl vte::Perform for Parser {
    fn print(&mut self, c: char) {
        println!("[print] {:?}", c);
    }

    fn execute(&mut self, byte: u8) {
        println!("[execute] {:02x}", byte);
    }

    fn hook(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[hook] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
    }

    fn put(&mut self, byte: u8) {
        println!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        println!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        println!(
            "[osc_dispatch] params={:?} bell_terminated={}",
            params, bell_terminated
        );
    }

    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[csi_dispatch] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );

        if ignore || intermediates.len() > 1 {
            return;
        }

        match (c, intermediates.get(0)) {
            ('m', None) => {
                if params.is_empty() {
                    Attr::Reset;
                } else {
                    for attr in attrs_from_sgr_parameters(args) {
                        match attr {
                            Some(attr) => handler.terminal_attribute(attr),
                            None => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        println!(
            "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
            intermediates, ignore, byte
        );
    }
}

fn attrs_from_sgr_parameters(parameters: &[i64]) -> Vec<Option<Attr>> {
    let mut i = 0;
    let mut attrs = Vec::with_capacity(parameters.len());
    loop {
        if i >= parameters.len() {
            break;
        }

        let attr = match parameters[i] {
            0 => Some(Attr::Reset),
            1 => Some(Attr::Bold),
            2 => Some(Attr::Dim),
            3 => Some(Attr::Italic),
            4 => Some(Attr::Underline),
            5 => Some(Attr::BlinkSlow),
            6 => Some(Attr::BlinkFast),
            7 => Some(Attr::Reverse),
            8 => Some(Attr::Hidden),
            9 => Some(Attr::Strike),
            21 => Some(Attr::CancelBold),
            22 => Some(Attr::CancelBoldDim),
            23 => Some(Attr::CancelItalic),
            24 => Some(Attr::CancelUnderline),
            25 => Some(Attr::CancelBlink),
            27 => Some(Attr::CancelReverse),
            28 => Some(Attr::CancelHidden),
            29 => Some(Attr::CancelStrike),
            30 => Some(Attr::Foreground(Color::Named(NamedColor::Black))),
            31 => Some(Attr::Foreground(Color::Named(NamedColor::Red))),
            32 => Some(Attr::Foreground(Color::Named(NamedColor::Green))),
            33 => Some(Attr::Foreground(Color::Named(NamedColor::Yellow))),
            34 => Some(Attr::Foreground(Color::Named(NamedColor::Blue))),
            35 => Some(Attr::Foreground(Color::Named(NamedColor::Magenta))),
            36 => Some(Attr::Foreground(Color::Named(NamedColor::Cyan))),
            37 => Some(Attr::Foreground(Color::Named(NamedColor::White))),
            38 => {
                let mut start = 0;
                if let Some(color) = parse_sgr_color(&parameters[i..], &mut start) {
                    i += start;
                    Some(Attr::Foreground(color))
                } else {
                    None
                }
            }
            39 => Some(Attr::Foreground(Color::Named(NamedColor::Foreground))),
            40 => Some(Attr::Background(Color::Named(NamedColor::Black))),
            41 => Some(Attr::Background(Color::Named(NamedColor::Red))),
            42 => Some(Attr::Background(Color::Named(NamedColor::Green))),
            43 => Some(Attr::Background(Color::Named(NamedColor::Yellow))),
            44 => Some(Attr::Background(Color::Named(NamedColor::Blue))),
            45 => Some(Attr::Background(Color::Named(NamedColor::Magenta))),
            46 => Some(Attr::Background(Color::Named(NamedColor::Cyan))),
            47 => Some(Attr::Background(Color::Named(NamedColor::White))),
            48 => {
                let mut start = 0;
                if let Some(color) = parse_sgr_color(&parameters[i..], &mut start) {
                    i += start;
                    Some(Attr::Background(color))
                } else {
                    None
                }
            }
            49 => Some(Attr::Background(Color::Named(NamedColor::Background))),
            90 => Some(Attr::Foreground(Color::Named(NamedColor::BrightBlack))),
            91 => Some(Attr::Foreground(Color::Named(NamedColor::BrightRed))),
            92 => Some(Attr::Foreground(Color::Named(NamedColor::BrightGreen))),
            93 => Some(Attr::Foreground(Color::Named(NamedColor::BrightYellow))),
            94 => Some(Attr::Foreground(Color::Named(NamedColor::BrightBlue))),
            95 => Some(Attr::Foreground(Color::Named(NamedColor::BrightMagenta))),
            96 => Some(Attr::Foreground(Color::Named(NamedColor::BrightCyan))),
            97 => Some(Attr::Foreground(Color::Named(NamedColor::BrightWhite))),
            100 => Some(Attr::Background(Color::Named(NamedColor::BrightBlack))),
            101 => Some(Attr::Background(Color::Named(NamedColor::BrightRed))),
            102 => Some(Attr::Background(Color::Named(NamedColor::BrightGreen))),
            103 => Some(Attr::Background(Color::Named(NamedColor::BrightYellow))),
            104 => Some(Attr::Background(Color::Named(NamedColor::BrightBlue))),
            105 => Some(Attr::Background(Color::Named(NamedColor::BrightMagenta))),
            106 => Some(Attr::Background(Color::Named(NamedColor::BrightCyan))),
            107 => Some(Attr::Background(Color::Named(NamedColor::BrightWhite))),
            _ => None,
        };

        attrs.push(attr);

        i += 1;
    }
    attrs
}

/// Parse a color specifier from list of attributes.
fn parse_sgr_color(attrs: &[i64], i: &mut usize) -> Option<Color> {
    if attrs.len() < 2 {
        return None;
    }

    match attrs[*i + 1] {
        2 => {
            // RGB color spec.
            if attrs.len() < 5 {
                debug!("Expected RGB color spec; got {:?}", attrs);
                return None;
            }

            let r = attrs[*i + 2];
            let g = attrs[*i + 3];
            let b = attrs[*i + 4];

            *i += 4;

            let range = 0..256;
            if !range.contains(&r) || !range.contains(&g) || !range.contains(&b) {
                debug!("Invalid RGB color spec: ({}, {}, {})", r, g, b);
                return None;
            }

            Some(Color::Spec(Rgb {
                r: r as u8,
                g: g as u8,
                b: b as u8,
            }))
        }
        5 => {
            if attrs.len() < 3 {
                debug!("Expected color index; got {:?}", attrs);
                None
            } else {
                *i += 2;
                let idx = attrs[*i];
                match idx {
                    0..=255 => Some(Color::Indexed(idx as u8)),
                    _ => {
                        debug!("Invalid color index: {}", idx);
                        None
                    }
                }
            }
        }
        _ => {
            debug!("Unexpected color attr: {}", attrs[*i + 1]);
            None
        }
    }
}
