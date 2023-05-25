/// Expand tabs as spaces.
/// tab_width = 0 is documented to mean do not replace tabs.
pub fn expand<'a, I>(line: I, tab_width: usize) -> String
where
    I: Iterator<Item = &'a str>,
{
    if tab_width > 0 {
        let tab_replacement = " ".repeat(tab_width);
        line.map(|s| if s == "\t" { &tab_replacement } else { s })
            .collect::<String>()
    } else {
        line.collect::<String>()
    }
}
