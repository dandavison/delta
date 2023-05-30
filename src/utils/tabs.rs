#[derive(Debug, Clone)]
pub struct TabCfg {
    replacement: String,
}

impl TabCfg {
    pub fn new(width: usize) -> Self {
        TabCfg {
            replacement: " ".repeat(width),
        }
    }
    pub fn width(&self) -> usize {
        self.replacement.len()
    }
    pub fn replace(&self) -> bool {
        !self.replacement.is_empty()
    }
}

/// Expand tabs as spaces.
pub fn expand<'a, I>(line: I, tab_cfg: &TabCfg) -> String
where
    I: Iterator<Item = &'a str>,
{
    if tab_cfg.replace() {
        line.map(|s| if s == "\t" { &tab_cfg.replacement } else { s })
            .collect::<String>()
    } else {
        line.collect::<String>()
    }
}
