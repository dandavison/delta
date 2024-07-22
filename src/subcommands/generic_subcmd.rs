use crate::cli::Opt;
use clap::CommandFactory;
use clap::{ArgMatches, Error};
use std::ffi::OsString;

pub const RG: &str = "rg";
const GIT: &str = "git";
pub const SUBCOMMANDS: &[&str] = &[RG, "show", "log", "diff", "grep", "blame", GIT];

#[derive(Debug, PartialEq)]
pub enum SubCmdKind {
    Git(String),
    GitDiff,
    Diff,
    Rg,
    None,
}

impl std::fmt::Display for SubCmdKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SubCmdKind::*;
        let s = match self {
            Git(arg) => return formatter.write_fmt(format_args!("git {arg}")),
            GitDiff => "git diff",
            Diff => "diff",
            Rg => "rg",
            None => "<none>",
        };
        formatter.write_str(s)
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

pub fn extract(args: &[OsString], orig_error: Error) -> (ArgMatches, SubCommand) {
    for (i, arg) in args.iter().enumerate() {
        let arg = if let Some(arg) = arg.to_str() {
            arg
        } else {
            continue;
        };
        if SUBCOMMANDS.contains(&arg) {
            match Opt::command().try_get_matches_from(&args[..i]) {
                Err(ref e) if e.kind() == clap::error::ErrorKind::DisplayVersion => {
                    unreachable!("version handled by caller");
                }
                Err(ref e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
                    unreachable!("help handled by caller");
                }
                Ok(matches) => {
                    let mut subcmd: Vec<OsString> = if arg == RG {
                        vec![arg, "--json"]
                    } else {
                        vec!["git", "-c", "color.ui=always", arg]
                    }
                    .iter()
                    .map(OsString::from)
                    .collect();

                    if arg == GIT {
                        // we would build: delta git git show.. => delta git show ..
                        subcmd.pop();
                    }
                    let kind = if arg == RG {
                        SubCmdKind::Rg
                    } else {
                        SubCmdKind::Git(
                            args[i..]
                                .first()
                                .map(|a| std::ffi::OsStr::to_string_lossy(a.as_ref()))
                                .unwrap_or("?".into())
                                .to_string(),
                        )
                    };
                    subcmd.extend(args[i + 1..].iter().map(|arg| arg.into()));
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
        panic!("no subcommand found");
    }
}

#[cfg(test)]
mod test {
    use crate::ansi::strip_ansi_codes;
    use std::ffi::OsString;
    use std::io::Cursor;

    #[test]
    fn subcommand_rg() {
        if grep_cli::resolve_binary("rg").is_err() {
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
}
