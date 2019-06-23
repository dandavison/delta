extern crate unidiff;

use std::io::{self, Read, Write};

use unidiff::PatchSet;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input);
    let mut patch_set = PatchSet::new();
    patch_set.parse(&mut input).ok().expect("Error parsing diff");
    for patched_file in patch_set {
        for hunk in patched_file {
            io::stdout().write_all(b"\n<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n");
            io::stdout().write_all(hunk.to_string().as_bytes());
        }
    }
}
