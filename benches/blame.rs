use std::io::BufReader;
use std::io::Cursor;
use std::process::Command;

use bytelines::*;
use criterion::{criterion_group, criterion_main, Criterion};

use libdelta::cli;
use libdelta::config;
use libdelta::delta::delta;

pub fn bench_blame(c: &mut Criterion) {
    libdelta::utils::process::set_no_calling_process();

    // The warmup will resize the writer
    let mut writer = Cursor::new(vec![]);
    let config = config::Config::from(cli::Opt::from_iter_and_git_config(
        [
            "/dev/null",
            "/dev/null",
            "--no-gitconfig",
            "--default-language",
            "rs",
        ],
        None,
    ));

    let paint_blame = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG", "/dev/null")
        .env("HOME", "/dev/null")
        .arg("blame")
        .arg("48fec2e6fdede01ed32f")
        .arg("--")
        .arg("src/paint.rs")
        .output()
        .unwrap_or_else(|err| panic!("git blame failed: {:?}", err))
        .stdout;

    // eprintln!("raw blame:\n{}", std::str::from_utf8(&paint_blame).unwrap());

    c.bench_function("blame paint.rs", |b| {
        b.iter(|| {
            writer.set_position(0);
            let lines = BufReader::new(paint_blame.as_slice()).byte_lines();

            delta(lines, &mut writer, &config).unwrap();
        })
    });

    // eprintln!(
    //     "\n{}\nsize: {}",
    //     std::str::from_utf8(writer.get_ref()).unwrap(),
    //     writer.get_ref().len()
    // );
}

criterion_group!(benches, bench_blame);
criterion_main!(benches);
