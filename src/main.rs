use std::io::{self, Read, Write};

fn main() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::new();
    stdin.read_to_string(&mut buf);
    stdout.write_all(buf.as_bytes());
    stdout.flush();
}
