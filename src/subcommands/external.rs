use crate::cli::Opt;
use clap::CommandFactory;
use clap::{ArgMatches, Error};
use std::ffi::{OsStr, OsString};

const RG: &str = "rg";
const GIT: &str = "git";
pub const SUBCOMMANDS: &[&str] = &[RG, GIT];

#[derive(PartialEq)]
pub enum SubCmdKind {
    Git(Option<String>), // Some(subcommand) if a git subcommand (git show, git log) was found
    GitDiff,
    Diff,
    Rg,
    None,
}

impl std::fmt::Display for SubCmdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SubCmdKind::*;
        let s = match self {
            Git(Some(arg)) => return formatter.write_fmt(format_args!("git {arg}")),
            Git(_) => "git",
            GitDiff => "git diff",
            Diff => "diff",
            Rg => "rg",
            None => "<none>",
        };
        formatter.write_str(s)
    }
}

impl std::fmt::Debug for SubCmdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SubCmdKind::Git(Some(arg)) => {
                return formatter.write_fmt(format_args!("\"git {}\"", arg.escape_debug()))
            }
            _ => format!("{}", self),
        };
        formatter.write_str("\"")?;
        formatter.write_str(&s)?;
        formatter.write_str("\"")
    }
}

#[derive(Debug)]
pub struct SubCommand {
    pub kind: SubCmdKind,
    pub args: Vec<OsString>,
}

impl SubCommand {
    pub fn new(kind: SubCmdKind, args: Vec<OsString>) -> Self {
        Self { kind, args }
    }

    pub fn none() -> Self {
        Self {
            kind: SubCmdKind::None,
            args: vec![],
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self.kind, SubCmdKind::None)
    }
}

/// Find the first arg that is a registered external subcommand and return a
/// tuple containing:
/// - The args prior to that point (delta can understand these)
/// - A SubCommand representing the external subcommand and its subsequent args
pub fn extract(args: &[OsString], orig_error: Error) -> (ArgMatches, SubCommand) {
    for (subcmd_pos, arg) in args.iter().filter_map(|a| a.to_str()).enumerate() {
        if SUBCOMMANDS.contains(&arg) {
            match Opt::command().try_get_matches_from(&args[..subcmd_pos]) {
                Err(ref e) if e.kind() == clap::error::ErrorKind::DisplayVersion => {
                    unreachable!("version handled by caller");
                }
                Err(ref e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
                    unreachable!("help handled by caller");
                }
                Ok(matches) => {
                    let (subcmd_args_index, kind, subcmd) = if arg == RG {
                        (subcmd_pos + 1, SubCmdKind::Rg, vec![RG, "--json"])
                    } else if arg == GIT {
                        let subcmd_args_index = subcmd_pos + 1;
                        let git_subcmd = args
                            .get(subcmd_args_index)
                            .and_then(|cmd| OsStr::to_str(cmd))
                            .and_then(|cmd| {
                                if cmd.starts_with("-") {
                                    None
                                } else {
                                    Some(cmd.into())
                                }
                            });
                        (
                            subcmd_args_index,
                            SubCmdKind::Git(git_subcmd),
                            // git does not start the pager and sees that it does not write to a
                            // terminal, so by default it will not use colors. Override it:
                            vec![GIT, "-c", "color.ui=always"],
                        )
                    } else {
                        unreachable!("arg must be in SUBCOMMANDS");
                    };

                    let subcmd = subcmd
                        .into_iter()
                        .map(OsString::from)
                        .chain(args[subcmd_args_index..].iter().map(OsString::from))
                        .collect();

                    return (matches, SubCommand::new(kind, subcmd));
                }
                Err(_) => {
                    // part before the subcommand failed to parse, report that error
                    #[cfg(not(test))]
                    orig_error.exit();
                    #[cfg(test)]
                    panic!("parse error before subcommand ");
                }
            }
        }
    }
    // no valid subcommand found, exit with the original error
    #[cfg(not(test))]
    orig_error.exit();
    #[cfg(test)]
    {
        let _ = orig_error;
        panic!("unexpected delta argument");
    }
}

#[cfg(test)]
mod test {
    use super::RG;
    use crate::ansi::strip_ansi_codes;
    use std::ffi::OsString;
    use std::io::Cursor;

    #[test]
    #[ignore] // reachable with --ignored, useful with --nocapture
    fn test_subcmd_kind_formatter() {
        use super::SubCmdKind::*;
        for s in [
            Git(Some("foo".into())),
            Git(Some("c\"'${}".into())),
            Git(Option::None),
            GitDiff,
            Diff,
            Rg,
            None,
        ] {
            eprintln!("{0} / {0:?} ", s);
        }
    }

    #[test]
    #[should_panic(expected = "unexpected delta argument")]
    fn just_delta_argument_error() {
        let mut writer = Cursor::new(vec![]);
        let runargs = [
            "--Invalid_Delta_Args",
            "abcdefg",
            "-C1",
            "--Bad_diff_Args_ignored",
        ]
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
        crate::run_app(runargs, Some(&mut writer)).unwrap();
    }

    #[test]
    #[should_panic(expected = "parse error before subcommand")]
    fn subcommand_found_but_delta_argument_error() {
        let mut writer = Cursor::new(vec![]);
        let runargs = [
            "--Invalid_Delta_Args",
            "git",
            "show",
            "-C1",
            "--Bad_diff_Args_ignored",
        ]
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
        crate::run_app(runargs, Some(&mut writer)).unwrap();
    }

    #[test]
    fn subcommand_rg() {
        #[cfg(windows)]
        // `resolve_binary` only works on windows
        if grep_cli::resolve_binary(RG).is_err() {
            return;
        }

        #[cfg(unix)]
        // resolve `rg` binary by walking PATH
        if std::env::var_os("PATH")
            .filter(|p| {
                std::env::split_paths(&p)
                    .filter(|p| !p.as_os_str().is_empty())
                    .filter_map(|p| p.join(RG).metadata().ok())
                    .any(|md| !md.is_dir())
            })
            .is_none()
        {
            return;
        }

        let mut writer = Cursor::new(vec![]);
        let needle = format!("{}{}", "Y40ii4RihK6", "lHiK4BDsGS").to_string();
        // --minus-style has no effect, just for cmdline parsing
        let runargs = [
            "--minus-style",
            "normal",
            "rg",
            &needle,
            "src/",
            "-N",
            "-C",
            "2",
            "-C0",
        ]
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
        let exit_code = crate::run_app(runargs, Some(&mut writer)).unwrap();
        let rg_output = std::str::from_utf8(writer.get_ref()).unwrap();
        let mut lines = rg_output.lines();
        // eprintln!("{}", rg_output);
        assert_eq!(
            r#"src/utils/process.rs "#,
            strip_ansi_codes(lines.next().expect("line 1"))
        );
        let line2 = format!(r#"            .join("{}x");"#, needle);
        assert_eq!(line2, strip_ansi_codes(lines.next().expect("line 2")));
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn subcommand_git_cat_file() {
        let mut writer = Cursor::new(vec![]);

        // only 39 of the 40 long git hash, rev-parse doesn't look up full hashes
        let runargs = "git rev-parse 5a4361fa037090adf729ab3f161832d969abc57"
            .split(' ')
            .map(OsString::from)
            .collect::<Vec<_>>();
        let exit_code = crate::run_app(runargs, Some(&mut writer)).unwrap();
        assert!(exit_code == 0 || exit_code == 128);

        // ref not found, probably a shallow git clone
        if exit_code == 128 {
            eprintln!("  Commit for test not found (shallow git clone?), skipping.");
            return;
        }

        assert_eq!(
            "5a4361fa037090adf729ab3f161832d969abc576\n",
            std::str::from_utf8(writer.get_ref()).unwrap()
        );

        let mut writer = Cursor::new(vec![]);

        let runargs = "git cat-file -p 5a4361fa037090adf729ab3f161832d969abc576:src/main.rs"
            .split(' ')
            .map(OsString::from)
            .collect::<Vec<_>>();
        let exit_code = crate::run_app(runargs, Some(&mut writer)).unwrap();
        let hello_world = std::str::from_utf8(writer.get_ref()).unwrap();
        assert_eq!(
            hello_world,
            r#"fn main() {
    println!("Hello, world!");
}
"#
        );
        assert_eq!(exit_code, 0);
    }
}
