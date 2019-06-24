extern crate unidiff;

use std::io::{self, Read};
use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use unidiff::PatchSet;

pub const DELTA_THEME_DEFAULT: &str = "InspiredGitHub";  // base16-mocha.dark

fn main() {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes[DELTA_THEME_DEFAULT];

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Error reading input");
    let mut patch_set = PatchSet::new();
    patch_set.parse(&mut input).ok().expect("Error parsing input as a diff");
    for patched_file in patch_set {
        // TODO: use both source and target to determine language
        let path = Path::new(&patched_file.target_file);
        let extension = path.extension()
            .expect(format!("Error determining file type: {}", path.to_str().unwrap()).as_str());
        let extension_str = extension.to_str().unwrap();
        let syntax = ps.find_syntax_by_extension(extension_str);

        match syntax {
            Some(syntax) => {
                let mut highlighter = HighlightLines::new(syntax, theme);
                for hunk in patched_file {
                    for line in LinesWithEndings::from(&hunk.to_string()) {
                        let ranges: Vec<(Style, &str)> = highlighter.highlight(line, &ps);
                        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                        print!("{}", escaped);
                    }
                }
            },
            None => {
                for hunk in patched_file {
                    for line in LinesWithEndings::from(&hunk.to_string()) {
                        print!("{}", line);
                    }
                }
            }
        }
    }
    println!("");
}
