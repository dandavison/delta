#![cfg(test)]

use std::io::Write;
use std::process::{Command, Stdio};

use crate::tests::integration_test_utils::EnvVarGuard;

#[test]
fn test_pager_integration_with_complex_command() {
    // This test demonstrates a bug where complex PAGER commands with arguments
    // cause "command not found" errors because bat::config::get_pager_executable
    // strips the arguments, leaving only the executable path.

    let mut delta_cmd = {
        // Acquire the environment access lock to prevent race conditions with other tests
        let _lock = crate::env::tests::ENV_ACCESS.lock().unwrap();

        // Use RAII guard to ensure environment variable is properly restored even if test panics
        let _env_guard = EnvVarGuard::new("PAGER", "/bin/sh -c \"head -10000 | cat\"");

        // Run delta as a subprocess with paging enabled - this will spawn the actual pager
        Command::new("cargo")
            .args(&["run", "--bin", "delta", "--", "--paging=always"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("delta to start successfully")
    };

    // Send test input to delta
    if let Some(stdin) = delta_cmd.stdin.as_mut() {
        stdin
            .write_all(b"line1\nline2\nline3\nline4\nline5\n")
            .unwrap();
    }

    // Wait for delta to complete and capture output
    let output = delta_cmd
        .wait_with_output()
        .expect("delta to finish and produce output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The bug: when bat strips arguments from "/bin/sh -c \"head -10000 | cat\""
    // to just "/bin/sh", the shell tries to execute each input line as a command,
    // resulting in "command not found" errors in stderr
    assert!(
        !stderr.contains("command not found"),
        "Pager integration failed: 'command not found' errors in stderr indicate that \
         bat::config::get_pager_executable stripped arguments from the PAGER command. \
         Stderr: {}\nStdout: {}",
        stderr,
        stdout
    );
}
