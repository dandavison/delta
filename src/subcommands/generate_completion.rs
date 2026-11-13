use std::io::Write;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli;

pub fn generate_completion_file(shell: Shell) -> std::io::Result<()> {
    let mut cmd = cli::Opt::command();
    let bin_name = cmd.get_bin_name().unwrap_or(cmd.get_name()).to_string();
    // Buffer first so clap_complete's infallible writer cannot panic on EPIPE;
    // BrokenPipe from the real stdout write is mapped to exit 0 in main.
    let mut buf = Vec::new();
    generate(shell, &mut cmd, bin_name, &mut buf);
    let mut stdout = std::io::stdout();
    stdout.write_all(&buf)?;
    stdout.flush()?;
    Ok(())
}
