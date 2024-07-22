// smaller helper subcommands:
pub mod generate_completion;
pub mod list_syntax_themes;
pub mod parse_ansi;
mod sample_diff;
pub mod show_colors;
pub mod show_config;
pub mod show_syntax_themes;
pub mod show_themes;

// start subprocesses:
// diff (fileA, fileB), and generic subcommands
pub mod diff;
mod generic_subcmd;
pub(crate) use generic_subcmd::extract;
pub(crate) use generic_subcmd::SubCmdKind;
pub(crate) use generic_subcmd::SubCommand;
pub(crate) use generic_subcmd::SUBCOMMANDS;
