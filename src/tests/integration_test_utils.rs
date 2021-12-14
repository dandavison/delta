#![cfg(test)]

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use bytelines::ByteLines;
use itertools;

use crate::ansi;
use crate::cli;
use crate::config;
use crate::delta::delta;
use crate::git_config::GitConfig;
use crate::utils::process::tests::FakeParentArgs;

pub fn make_options_from_args_and_git_config(
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
) -> cli::Opt {
    _make_options_from_args_and_git_config(args, git_config_contents, git_config_path, false)
}

pub fn make_options_from_args_and_git_config_honoring_env_var(
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
) -> cli::Opt {
    _make_options_from_args_and_git_config(args, git_config_contents, git_config_path, true)
}

fn _make_options_from_args_and_git_config(
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
    honor_env_var: bool,
) -> cli::Opt {
    let mut args: Vec<&str> = itertools::chain(&["/dev/null", "/dev/null"], args)
        .map(|s| *s)
        .collect();
    let git_config = match (git_config_contents, git_config_path) {
        (Some(contents), Some(path)) => Some(make_git_config(contents, path, honor_env_var)),
        _ => {
            args.push("--no-gitconfig");
            None
        }
    };
    cli::Opt::from_iter_and_git_config(args, git_config)
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

pub fn make_git_config(contents: &[u8], path: &str, honor_env_var: bool) -> GitConfig {
    let path = Path::new(path);
    let mut file = File::create(path).unwrap();
    file.write_all(contents).unwrap();
    GitConfig::from_path(&path, honor_env_var)
}

pub fn get_line_of_code_from_delta(
    input: &str,
    line_number: usize,
    expected_text: &str,
    config: &config::Config,
) -> String {
    let output = run_delta(&input, config);
    let line_of_code = output.lines().nth(line_number).unwrap();
    assert!(ansi::strip_ansi_codes(line_of_code) == expected_text);
    line_of_code.to_string()
}

// Given an `expected` block as a raw string like: `r#"
//     #indent_mark [optional]
//     line1"#;`  // line 2 etc.
// ignore the first newline and compare the following `lines()` to those produced
// by `have`, `skip`-ping the first few. The leading spaces of the first line
// are stripped from every following line (and verified), unless the first line
// marks the indentation level with `#indent_mark`.
pub fn lines_match(expected: &str, have: &str, skip: Option<usize>) {
    let mut exp = expected.lines().peekable();
    assert!(exp.next() == Some(""), "first line must be empty");
    let line1 = exp.peek().unwrap();
    let indentation = line1.find(|c| c != ' ').unwrap_or(0);
    let ignore_indent = &line1[indentation..] == "#indent_mark";
    if ignore_indent {
        let _indent_mark = exp.next();
    }

    let mut it = have.lines().skip(skip.unwrap_or(0));

    for (i, expected) in exp.enumerate() {
        if !ignore_indent {
            let next_indentation = expected.find(|c| c != ' ').unwrap_or(0);
            assert!(
                indentation == next_indentation,
                "The expected block has mixed indentation (use #indent_mark if that is on purpose)"
            );
        }
        assert_eq!(
            &expected[indentation..],
            it.next().unwrap(),
            "on line {} of input",
            i + 1
        );
    }
    assert_eq!(it.next(), None, "more input than expected");
}

pub struct DeltaTest {
    config: config::Config,
    calling_process: Option<String>,
}

impl DeltaTest {
    pub fn with(args: &[&str]) -> Self {
        Self {
            config: make_config_from_args(args),
            calling_process: None,
        }
    }

    pub fn set_cfg<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut config::Config),
    {
        f(&mut self.config);
        self
    }

    pub fn with_calling_process(mut self, command: &str) -> Self {
        self.calling_process = Some(command.to_string());
        self
    }

    pub fn with_config_and_input(config: &config::Config, input: &str) -> DeltaTestOutput {
        DeltaTestOutput {
            output: run_delta(input, &config),
            explain_ansi_: false,
        }
    }

    pub fn with_input(&self, input: &str) -> DeltaTestOutput {
        let _args = FakeParentArgs::for_scope(self.calling_process.as_deref().unwrap_or(""));
        DeltaTest::with_config_and_input(&self.config, input)
    }
}

pub struct DeltaTestOutput {
    output: String,
    explain_ansi_: bool,
}

impl DeltaTestOutput {
    /// Print output, either without ANSI escape sequences or, if explain_ansi() has been called,
    /// with ASCII explanation of ANSI escape sequences.
    #[allow(unused)]
    pub fn inspect(self) -> Self {
        eprintln!("{}", "▼".repeat(100));
        eprintln!("{}", self.format_output());
        eprintln!("{}", "▲".repeat(100));
        self
    }

    /// Print raw output, with any ANSI escape sequences.
    #[allow(unused)]
    pub fn inspect_raw(self) -> Self {
        eprintln!("{}", "▼".repeat(100));
        eprintln!("{}", self.output);
        eprintln!("{}", "▲".repeat(100));
        self
    }

    pub fn explain_ansi(mut self) -> Self {
        self.explain_ansi_ = true;
        self
    }

    pub fn expect_skip(self, skip: usize, expected: &str) -> String {
        let processed = self.format_output();
        lines_match(expected, &processed, Some(skip));
        processed
    }

    pub fn expect(self, expected: &str) -> String {
        self.expect_skip(crate::config::HEADER_LEN, expected)
    }

    fn format_output(&self) -> String {
        if self.explain_ansi_ {
            ansi::explain_ansi(&self.output, false)
        } else {
            ansi::strip_ansi_codes(&self.output)
        }
    }
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

pub mod tests {
    use super::*;

    #[test]
    fn test_lines_match_ok() {
        let expected = r#"
        one
        two
        three"#;
        lines_match(expected, "one\ntwo\nthree", None);

        let expected = r#"
        #indent_mark
        one
          2
        three"#;
        lines_match(expected, "one\n  2\nthree", None);

        let expected = r#"
            #indent_mark
             1 
              2  
               3"#;
        lines_match(expected, " 1 \n  2  \n   3", None);

        let expected = r#"
        #indent_mark
         1 
ignored!  2  
           3"#;
        lines_match(expected, " 1 \n  2  \n   3", None);
        let expected = "\none\ntwo\nthree";
        lines_match(expected, "one\ntwo\nthree", None);
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_nl() {
        let expected = r#"bad
        lines"#;
        lines_match(expected, "bad\nlines", None);
    }

    #[test]
    #[should_panic]
    fn test_lines_match_iter_not_consumed() {
        let expected = r#"
        one
        two
        three"#;
        lines_match(expected, "one\ntwo\nthree\nFOUR", None);
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_indent_mark_1() {
        let expected = r#"
        ok
          wrong_indent
        "#;
        lines_match(expected, "ok", None);
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_indent_mark_2() {
        let expected = r#"
        ok
       wrong_indent
        "#;
        lines_match(expected, "ok", None);
    }

    #[test]
    fn test_delta_test() {
        let input = "@@ -1,1 +1,1 @@ fn foo() {\n-1\n+2\n";
        DeltaTest::with(&["--raw"])
            .set_cfg(|c| c.pager = None)
            .set_cfg(|c| c.line_numbers = true)
            .with_input(input)
            .expect_skip(
                0,
                r#"
                 #indent_mark
                 @@ -1,1 +1,1 @@ fn foo() {
                  1  ⋮    │-1
                     ⋮ 1  │+2"#,
            );

        DeltaTest::with(&[]).with_input(input).expect_skip(
            4,
            r#"
                1
                2"#,
        );

        DeltaTest::with(&["--raw"])
            .with_input(input)
            .explain_ansi()
            .expect_skip(
                0,
                "\n\
                (normal)@@ -1,1 +1,1 @@ fn foo() {\n\
                (red)-1(normal)\n\
                (green)+2(normal)",
            );
    }
}
