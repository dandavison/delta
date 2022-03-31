#![cfg(test)]

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use bytelines::ByteLines;
use itertools;

use crate::ansi;
use crate::cli;
use crate::config;
use crate::delta::delta;
use crate::env::DeltaEnv;
use crate::git_config::GitConfig;
use crate::tests::test_utils;
use crate::utils::process::tests::FakeParentArgs;

pub fn make_options_from_args_and_git_config(
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
) -> cli::Opt {
    _make_options_from_args_and_git_config(
        DeltaEnv::default(),
        args,
        git_config_contents,
        git_config_path,
        false,
    )
}

pub fn make_options_from_args_and_git_config_with_custom_env(
    env: DeltaEnv,
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
) -> cli::Opt {
    _make_options_from_args_and_git_config(env, args, git_config_contents, git_config_path, false)
}

pub fn make_options_from_args_and_git_config_honoring_env_var_with_custom_env(
    env: DeltaEnv,
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
) -> cli::Opt {
    _make_options_from_args_and_git_config(env, args, git_config_contents, git_config_path, true)
}

fn _make_options_from_args_and_git_config(
    env: DeltaEnv,
    args: &[&str],
    git_config_contents: Option<&[u8]>,
    git_config_path: Option<&str>,
    honor_env_var: bool,
) -> cli::Opt {
    let mut args: Vec<&str> = itertools::chain(&["/dev/null", "/dev/null"], args)
        .map(|s| *s)
        .collect();
    let git_config = match (git_config_contents, git_config_path) {
        (Some(contents), Some(path)) => Some(make_git_config(&env, contents, path, honor_env_var)),
        _ => {
            args.push("--no-gitconfig");
            None
        }
    };
    cli::Opt::from_iter_and_git_config(env, args, git_config)
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

pub fn make_git_config(
    env: &DeltaEnv,
    contents: &[u8],
    path: &str,
    honor_env_var: bool,
) -> GitConfig {
    let path = Path::new(path);
    let mut file = File::create(path).unwrap();
    file.write_all(contents).unwrap();
    GitConfig::from_path(env, &path, honor_env_var)
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
// to indicate the last line in the list). The leading spaces of the first line
// are stripped from every following line (and verified), unless the first line
// marks the indentation level with `#indent_mark`.
pub fn assert_lines_match_after_skip(skip: usize, expected: &str, have: &str) {
    let mut exp = expected.lines().peekable();
    let mut line1 = exp.next().unwrap();
    let allow_partial = line1 == "#partial";
    assert!(
        allow_partial || line1.is_empty(),
        "first line must be empty or \"#partial\""
    );
    line1 = exp.peek().unwrap();
    let indentation = line1.find(|c| c != ' ').unwrap_or(0);
    let ignore_indent = &line1[indentation..] == "#indent_mark";
    if ignore_indent {
        let _indent_mark = exp.next();
    }

    let mut it = have.lines().skip(skip);

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
            "on line {} of input:\n{}",
            i + 1,
            delineated_string(have),
        );
    }
    if !allow_partial {
        assert_eq!(it.next(), None, "more input than expected");
    }
}

pub fn assert_lines_match(expected: &str, have: &str) {
    assert_lines_match_after_skip(0, expected, have)
}

pub fn delineated_string(txt: &str) -> String {
    let top = "▼".repeat(100);
    let btm = "▲".repeat(100);
    let nl = "\n";
    top + &nl + txt + &nl + &btm
}

pub struct DeltaTest<'a> {
    config: Cow<'a, config::Config>,
    calling_process: Option<String>,
    explain_ansi_: bool,
}

impl<'a> DeltaTest<'a> {
    pub fn with_args(args: &[&str]) -> Self {
        Self {
            config: Cow::Owned(make_config_from_args(args)),
            calling_process: None,
            explain_ansi_: false,
        }
    }

    pub fn with_config(config: &'a config::Config) -> Self {
        Self {
            config: Cow::Borrowed(config),
            calling_process: None,
            explain_ansi_: false,
        }
    }

    pub fn set_config<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut config::Config),
    {
        let mut owned_config = self.config.into_owned();
        f(&mut owned_config);
        self.config = Cow::Owned(owned_config);
        self
    }

    pub fn with_calling_process(mut self, command: &str) -> Self {
        self.calling_process = Some(command.to_string());
        self
    }

    pub fn explain_ansi(mut self) -> Self {
        self.explain_ansi_ = true;
        self
    }

    pub fn with_input(&self, input: &str) -> DeltaTestOutput {
        let _args = FakeParentArgs::for_scope(self.calling_process.as_deref().unwrap_or(""));
        let raw = run_delta(input, &self.config);
        let cooked = if self.explain_ansi_ {
            ansi::explain_ansi(&raw, false)
        } else {
            ansi::strip_ansi_codes(&raw)
        };

        DeltaTestOutput {
            raw_output: raw,
            output: cooked,
        }
    }
}

pub struct DeltaTestOutput {
    pub raw_output: String,
    pub output: String,
}

impl DeltaTestOutput {
    /// Print output, either without ANSI escape sequences or, if explain_ansi() has been called,
    /// with ASCII explanation of ANSI escape sequences.
    #[allow(unused)]
    pub fn inspect(self) -> Self {
        eprintln!("{}", delineated_string(&self.output.as_str()));
        self
    }

    /// Print raw output, with any ANSI escape sequences.
    #[allow(unused)]
    pub fn inspect_raw(self) -> Self {
        eprintln!("{}", delineated_string(&self.raw_output));
        self
    }

    pub fn expect_after_skip(self, skip: usize, expected: &str) -> Self {
        assert_lines_match_after_skip(skip, expected, &self.output);
        self
    }

    pub fn expect(self, expected: &str) -> Self {
        self.expect_after_skip(0, expected)
    }

    pub fn expect_after_header(self, expected: &str) -> Self {
        self.expect_after_skip(crate::config::HEADER_LEN, expected)
    }

    pub fn expect_contains(self, expected: &str) -> Self {
        assert!(
            self.output.contains(expected),
            "Output does not contain \"{}\":\n{}\n",
            expected,
            delineated_string(&self.output.as_str())
        );
        self
    }

    pub fn expect_raw_contains(self, expected: &str) -> Self {
        assert!(
            self.raw_output.contains(expected),
            "Raw output does not contain \"{}\":\n{}\n",
            expected,
            delineated_string(&self.raw_output.as_str())
        );
        self
    }

    pub fn expect_contains_once(self, expected: &str) -> Self {
        assert!(
            test_utils::contains_once(&self.output, expected),
            "Output does not contain \"{}\" exactly once:\n{}\n",
            expected,
            delineated_string(&self.output.as_str())
        );
        self
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
        assert_lines_match(expected, "one\ntwo\nthree");

        let expected = r#"
        #indent_mark
        one
          2
        three"#;
        assert_lines_match(expected, "one\n  2\nthree");

        let expected = r#"
            #indent_mark
             1 
              2  
               3"#;
        assert_lines_match(expected, " 1 \n  2  \n   3");

        let expected = r#"
        #indent_mark
         1 
ignored!  2  
           3"#;
        assert_lines_match(expected, " 1 \n  2  \n   3");
        let expected = "\none\ntwo\nthree";
        assert_lines_match(expected, "one\ntwo\nthree");
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_nl() {
        let expected = r#"bad
        lines"#;
        assert_lines_match(expected, "bad\nlines");
    }

    #[test]
    #[should_panic]
    fn test_lines_match_iter_not_consumed() {
        let expected = r#"
        one
        two
        three"#;
        assert_lines_match(expected, "one\ntwo\nthree\nFOUR");
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_indent_mark_1() {
        let expected = r#"
        ok
          wrong_indent
        "#;
        assert_lines_match(expected, "ok");
    }

    #[test]
    #[should_panic]
    fn test_lines_match_no_indent_mark_2() {
        let expected = r#"
        ok
       wrong_indent
        "#;
        assert_lines_match(expected, "ok");
    }

    #[test]
    fn test_delta_test() {
        let input = "@@ -1,1 +1,1 @@ fn foo() {\n-1\n+2\n";
        DeltaTest::with_args(&["--raw"])
            .set_config(|c| c.pager = None)
            .set_config(|c| c.line_numbers = true)
            .with_input(input)
            .expect(
                r#"
                 #indent_mark
                 @@ -1,1 +1,1 @@ fn foo() {
                   1 ⋮    │-1
                     ⋮  1 │+2"#,
            );

        DeltaTest::with_args(&[])
            .with_input(input)
            .expect_after_skip(
                4,
                r#"
                1
                2"#,
            );

        DeltaTest::with_args(&["--raw"])
            .explain_ansi()
            .with_input(input)
            .expect(
                "\n\
                (normal)@@ -1,1 +1,1 @@ fn foo() {\n\
                (red)-1(normal)\n\
                (green)+2(normal)",
            );
    }
}
