use std::io::{self, BufRead};

#[cfg(not(tarpaulin_include))]
pub fn parse_ansi() -> std::io::Result<()> {
    use crate::{ansi, style::Style};

    for line in io::stdin().lock().lines() {
        for (ansi_term_style, s) in ansi::parse_style_sections(
            &line.unwrap_or_else(|line| panic!("Invalid utf-8: {:?}", line)),
        ) {
            let style = Style {
                ansi_term_style,
                ..Style::default()
            };
            print!("({}){}", style.to_painted_string(), style.paint(s));
        }
        println!();
    }
    Ok(())
}
