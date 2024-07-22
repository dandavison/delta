// internal subcommands:
pub mod generate_completion;
pub mod list_syntax_themes;
pub mod parse_ansi;
mod sample_diff;
pub mod show_colors;
pub mod show_config;
pub mod show_syntax_themes;
pub mod show_themes;

// start external processes, e.g. `git diff` or `rg`, output is read by delta
pub mod diff;
mod external;
pub(crate) use external::extract;
pub(crate) use external::SubCmdKind;
pub(crate) use external::SubCommand;
pub(crate) use external::SUBCOMMANDS;
