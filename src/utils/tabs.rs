use unicode_segmentation::UnicodeSegmentation;

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
pub fn expand(line: &str, tab_cfg: &TabCfg) -> String {
    if tab_cfg.replace() && line.as_bytes().contains(&b'\t') {
        itertools::join(line.split('\t'), &tab_cfg.replacement)
    } else {
        line.to_string()
    }
}

/// Remove `prefix` chars from `line`, then call `tabs::expand()`.
pub fn remove_prefix_and_expand(prefix: usize, line: &str, tab_cfg: &TabCfg) -> String {
    let line_bytes = line.as_bytes();
    // The to-be-removed prefixes are almost always ascii +/- (or ++/ +/.. for merges) for
    // which grapheme clusters are not required.
    if line_bytes.len() >= prefix && line_bytes[..prefix].is_ascii() {
        // Safety: slicing into the utf-8 line-str is ok, upto `prefix` only ascii was present.
        expand(&line[prefix..], tab_cfg)
    } else {
        let cut_line = line.graphemes(true).skip(prefix).collect::<String>();
        expand(&cut_line, tab_cfg)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_remove_prefix_and_expand() {
        let line = "+-foo\tbar";
        let result = remove_prefix_and_expand(2, line, &TabCfg::new(3));
        assert_eq!(result, "foo   bar");
        let result = remove_prefix_and_expand(2, line, &TabCfg::new(0));
        assert_eq!(result, "foo\tbar");

        let utf8_prefix = "-│-foo\tbar";
        let n = 3;
        let result = remove_prefix_and_expand(n, utf8_prefix, &TabCfg::new(1));
        assert_eq!(result, "foo bar");
        // ensure non-ascii chars were removed:
        assert!(utf8_prefix.len() - result.len() > n);
    }
}
