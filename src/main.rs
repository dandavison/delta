extern crate unidiff;

use std::io::{self, Read, Write};

use unidiff::PatchSet;

fn main() {
    let mut diff_str = String::new();
    let mut patch = PatchSet::new();
    io::stdin().read_to_string(&mut diff_str);
    patch.parse(&mut diff_str).ok().expect("Error parsing diff");
    io::stdout().write_all(diff_str.as_bytes());
}
