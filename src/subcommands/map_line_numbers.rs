use crate::config::Config;
use crate::delta::delta;
use bytelines::ByteLinesReader;
use lazy_static::lazy_static;
use std::io::{self, ErrorKind, Write};
use std::sync::Mutex;

struct MapLineNumbersData {
    file_name: String,
    minus_line_number: usize,
    plus_line_number: Option<usize>,
}

lazy_static! {
    static ref MAP_LINE_NUMBERS_DATA: Mutex<MapLineNumbersData> = Mutex::new(MapLineNumbersData {
        file_name: "".to_string(),
        minus_line_number: 0,
        plus_line_number: None,
    });
}

pub fn set_file_name(name: String) {
    let mut data = MAP_LINE_NUMBERS_DATA.lock().unwrap();
    data.file_name = name;
}

pub fn set_minus_line_number(n: usize) {
    let mut data = MAP_LINE_NUMBERS_DATA.lock().unwrap();
    data.minus_line_number = n;
}

pub fn set_plus_line_number(n: Option<usize>) {
    let mut data = MAP_LINE_NUMBERS_DATA.lock().unwrap();
    data.plus_line_number = n;
}

pub fn emit_map_entry() {
    let data = MAP_LINE_NUMBERS_DATA.lock().unwrap();
    println!(
        "{},{},{}",
        data.file_name,
        data.minus_line_number,
        data.plus_line_number.unwrap_or(0)
    )
}

#[cfg(not(tarpaulin_include))]
pub fn map_line_numbers(config: &Config) -> std::io::Result<()> {
    let mut writer = NullWriter;
    if let Err(error) = delta(io::stdin().lock().byte_lines(), &mut writer, &config) {
        match error.kind() {
            ErrorKind::BrokenPipe => return Ok(()),
            _ => eprintln!("{error}"),
        }
    };
    Ok(())
}

struct NullWriter;

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
