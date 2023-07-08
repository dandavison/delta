use crate::config::Config;
use crate::delta::delta;
use bytelines::ByteLinesReader;
use std::io::{self, ErrorKind, Write};

#[cfg(not(tarpaulin_include))]
pub fn map_line_numbers(config: &Config) -> std::io::Result<()> {
    let mut writer = NullWriter {};
    if let Err(error) = delta(io::stdin().lock().byte_lines(), &mut writer, &config) {
        match error.kind() {
            ErrorKind::BrokenPipe => return Ok(()),
            _ => eprintln!("{error}"),
        }
    };
    Ok(())
}

struct NullWriter {}

impl Write for NullWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn write_all(&mut self, mut _buf: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
