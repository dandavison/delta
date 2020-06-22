#[cfg(test)]
pub mod integration_test_utils {
    use std::io::BufReader;

    use bytelines::ByteLines;
    use console::strip_ansi_codes;

    use crate::cli;
    use crate::config;
    use crate::delta::delta;

    fn make_options(args: &[&str]) -> cli::Opt {
        // FIXME: should not be necessary
        let (dummy_minus_file, dummy_plus_file) = ("/dev/null", "/dev/null");
        let mut augmented_args = vec![dummy_minus_file, dummy_plus_file];

        for arg in args {
            augmented_args.push(arg);
        }
        augmented_args.push("--no-gitconfig");
        cli::Opt::from_iter_and_git_config(augmented_args, &mut None)
    }

    pub fn make_config(args: &[&str]) -> config::Config {
        config::Config::from(make_options(args))
    }

    pub fn get_line_of_code_from_delta(
        input: &str,
        line_number: usize,
        expected_text: &str,
        config: &config::Config,
    ) -> String {
        let output = run_delta(&input, config);
        let line_of_code = output.lines().nth(line_number).unwrap();
        assert!(strip_ansi_codes(line_of_code) == expected_text);
        line_of_code.to_string()
    }

    pub fn run_delta(input: &str, config: &config::Config) -> String {
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
