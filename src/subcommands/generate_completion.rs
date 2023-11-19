use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli;

pub fn generate_completion_file(shell: Shell) -> std::io::Result<()> {
    let mut cmd = cli::Opt::command();
    let bin_name = cmd.get_bin_name().unwrap_or(cmd.get_name()).to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}
