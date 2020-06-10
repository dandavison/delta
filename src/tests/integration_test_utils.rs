#[cfg(test)]
pub mod integration_test_utils {
    use std::io::BufReader;

    use bytelines::ByteLines;
    use console::strip_ansi_codes;
    use structopt::StructOpt;

    use crate::cli;
    use crate::config;
    use crate::delta::delta;

    pub fn make_config<'a>(_args: &[&str]) -> config::Config<'a> {
        // FIXME: should not be necessary
        let (dummy_minus_file, dummy_plus_file) = ("/dev/null", "/dev/null");
        let mut args = vec![dummy_minus_file, dummy_plus_file];

        for arg in _args {
            args.push(arg);
        }
        args.push("--no-gitconfig");
        let arg_matches = cli::Opt::clap().get_matches_from(args);
        cli::process_command_line_arguments(arg_matches, &mut None)
    }

    pub fn get_line_of_code_from_delta<'a>(
        input: &str,
        line_number: usize,
        expected_text: &str,
        config: &config::Config<'a>,
    ) -> String {
        let output = run_delta(&input, config);
        let line_of_code = output.lines().nth(line_number).unwrap();
        assert!(strip_ansi_codes(line_of_code) == expected_text);
        line_of_code.to_string()
    }

    pub fn run_delta<'a>(input: &str, config: &config::Config<'a>) -> String {
        let mut writer: Vec<u8> = Vec::new();

        delta(
            ByteLines::new(BufReader::new(input.as_bytes())),
            &mut writer,
            &config,
        )
        .unwrap();
        String::from_utf8(writer).unwrap()
    }
}
