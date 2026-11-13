use clap::{Command, CommandFactory};
use clap_complete::{generate, Shell};

use crate::cli;

pub fn generate_completion_file(shell: Shell) -> std::io::Result<()> {
    let mut cmd = completion_command();
    let bin_name = cmd.get_bin_name().unwrap_or(cmd.get_name()).to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}

// External subcommands accept more positional arguments than a two-file diff.
// Relax only the completion schema; runtime argument parsing is unchanged.
fn completion_command() -> Command {
    cli::Opt::command().mut_arg("plus_file", |arg| {
        arg.num_args(0..)
            .allow_hyphen_values(true)
            .trailing_var_arg(true)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn completion_accepts_external_command_arguments() {
        for args in [
            vec!["rg", "pattern", "first.txt", "dir/second file.txt"],
            vec!["rg", "--hidden", "--glob", "*.rs", "pattern", "src/"],
            vec!["rg", "pattern", "--", "-file.txt"],
            vec!["git", "diff", "--", "src/", "other file.txt"],
        ] {
            let matches = completion_command()
                .try_get_matches_from(
                    ["delta", "--side-by-side"]
                        .iter()
                        .copied()
                        .chain(args.iter().copied()),
                )
                .unwrap();
            let remaining: Vec<_> = matches
                .get_many::<PathBuf>("plus_file")
                .unwrap()
                .map(|path| path.to_str().unwrap())
                .collect();
            assert_eq!(remaining, args[1..]);
        }
    }

    #[test]
    fn zsh_completes_paths_beyond_two_positionals() {
        let mut output = Vec::new();
        generate(Shell::Zsh, &mut completion_command(), "delta", &mut output);
        let output = String::from_utf8(output).unwrap();
        // `*` is essential: two fixed positionals stop completion after
        // `delta rg pattern`, before the user can complete any search paths.
        assert!(output
            .lines()
            .any(|line| line.starts_with("'*::plus_file ") && line.contains(":_files'")));
    }

    #[test]
    fn runtime_file_arguments_are_unchanged() {
        let args = ["delta", "first.txt", "second.txt"];
        for command in [cli::Opt::command(), completion_command()] {
            let matches = command.try_get_matches_from(args).unwrap();
            assert_eq!(
                matches.get_one::<PathBuf>("minus_file").unwrap(),
                &PathBuf::from("first.txt")
            );
            assert_eq!(
                matches.get_one::<PathBuf>("plus_file").unwrap(),
                &PathBuf::from("second.txt")
            );
        }
        assert!(cli::Opt::command()
            .try_get_matches_from(["delta", "first.txt", "second.txt", "third.txt"])
            .is_err());
    }

    #[test]
    fn all_supported_shells_generate() {
        for shell in [
            Shell::Bash,
            Shell::Elvish,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Zsh,
        ] {
            let mut output = Vec::new();
            generate(shell, &mut completion_command(), "delta", &mut output);
            assert!(!output.is_empty(), "empty completion for {}", shell);
        }
    }
}
