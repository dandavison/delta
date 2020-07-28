#[cfg(test)]
pub mod integration_test_utils {
    use std::fs::File;
    use std::io::{BufReader, Write};
    use std::path::Path;

    use bytelines::ByteLines;
    use console::strip_ansi_codes;
    use itertools;

    use crate::cli;
    use crate::config;
    use crate::delta::delta;
    use crate::git_config::GitConfig;

    pub fn make_options_from_args_and_git_config(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        git_config_path: Option<&str>,
    ) -> cli::Opt {
        let mut args: Vec<&str> = itertools::chain(&["/dev/null", "/dev/null"], args)
            .map(|s| *s)
            .collect();
        let mut git_config = match (git_config_contents, git_config_path) {
            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
            _ => {
                args.push("--no-gitconfig");
                None
            }
        };
        cli::Opt::from_iter_and_git_config(args, &mut git_config)
    }

    pub fn make_options_from_args(args: &[&str]) -> cli::Opt {
        make_options_from_args_and_git_config(args, None, None)
    }

    #[allow(dead_code)]
    pub fn make_config_from_args_and_git_config(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        git_config_path: Option<&str>,
    ) -> config::Config {
        config::Config::from(make_options_from_args_and_git_config(
            args,
            git_config_contents,
            git_config_path,
        ))
    }

    pub fn make_config_from_args(args: &[&str]) -> config::Config {
        config::Config::from(make_options_from_args(args))
    }

    fn make_git_config(contents: &[u8], path: &str) -> GitConfig {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        GitConfig::from_path(&path)
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
