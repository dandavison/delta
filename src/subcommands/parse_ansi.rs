use std::io::{self, BufRead};

#[cfg(not(tarpaulin_include))]
pub fn parse_ansi() -> std::io::Result<()> {
    use crate::ansi;

    for line in io::stdin().lock().lines() {
        println!(
            "{}",
            ansi::explain_ansi(
                &line.unwrap_or_else(|line| panic!("Invalid utf-8: {:?}", line)),
                true
            )
        );
    }
    Ok(())
}
