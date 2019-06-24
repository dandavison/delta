extern crate unidiff;

use std::io::{self, Read, Write};
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use unidiff::PatchSet;

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input);
    let mut patch_set = PatchSet::new();
    patch_set.parse(&mut input).ok().expect("Error parsing diff");
    for patched_file in patch_set {
        let path = Path::new(&patched_file.source_file);
        let syntax = ps.find_syntax_by_extension(path.extension().unwrap().to_str().unwrap()).unwrap();
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        io::stdout().write_all(format!("\n<<<<<<<<<<<<<<<<<<<<<< file: {}\n", patched_file.source_file).as_bytes());
        for hunk in patched_file {
            for line in LinesWithEndings::from(&hunk.to_string()) {
                let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                println!("{}", escaped);
            }
        }
    }
}
