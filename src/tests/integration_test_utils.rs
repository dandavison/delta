#[cfg(test)]
pub mod integration_test_utils {
    use std::ffi::OsString;
    use std::io::BufReader;

    use bytelines::ByteLines;
    use console::strip_ansi_codes;
    use structopt::{clap, StructOpt};

    use crate::cli;
    use crate::config;
    use crate::delta::delta;

    pub fn get_command_line_options() -> cli::Opt {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.syntax_theme = None; // TODO: Why does opt.syntax_theme have the value Some("")?
        opt.no_gitconfig = true;
        opt
    }

    pub fn get_line_of_code_from_delta<'a>(
        input: &str,
        line_number: usize,
        expected_text: &str,
        options: cli::Opt,
    ) -> (String, config::Config<'a>) {
        let (output, config) = run_delta(&input, options);
        let line_of_code = output.lines().nth(line_number).unwrap();
        assert!(strip_ansi_codes(line_of_code) == expected_text);
        (line_of_code.to_string(), config)
    }

    pub fn run_delta<'a>(input: &str, options: cli::Opt) -> (String, config::Config<'a>) {
        let mut writer: Vec<u8> = Vec::new();

        let config =
            cli::process_command_line_arguments(options, clap::ArgMatches::new(), &mut None);

        delta(
            ByteLines::new(BufReader::new(input.as_bytes())),
            &mut writer,
            &config,
        )
        .unwrap();
        (String::from_utf8(writer).unwrap(), config)
    }
}
