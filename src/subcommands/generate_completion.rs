use std::io::Write;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli;

pub fn generate_completion_file(shell: Shell) -> std::io::Result<()> {
    let mut cmd = cli::Opt::command();
    let bin_name = cmd.get_bin_name().unwrap_or(cmd.get_name()).to_string();
    let mut buf = Vec::new();
    generate(shell, &mut cmd, bin_name, &mut buf);
    std::io::stdout().write_all(&buf)?;
    std::io::stdout().flush()
}
