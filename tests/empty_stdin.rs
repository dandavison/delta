use std::io::Write;
use std::process::{Command, Stdio};

fn delta_bin() -> String {
    std::env::var("CARGO_BIN_EXE_delta").unwrap_or_else(|_| {
        // Fallback for when not run via cargo test
        format!("{}/target/debug/delta", env!("CARGO_MANIFEST_DIR"))
    })
}

#[test]
fn test_no_output_on_empty_stdin_when_stdout_piped() {
    let mut child = Command::new(delta_bin())
        .args(["--no-gitconfig"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn delta");

    child.stdin.take().unwrap().write_all(b"").unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "delta exited with error on empty stdin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout.len(),
        0,
        "Expected 0 bytes on empty stdin piped to delta, got {} bytes: {:?}",
        output.stdout.len(),
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_empty_stdin_does_not_hang_with_paging_always() {
    let mut child = Command::new(delta_bin())
        .args(["--no-gitconfig", "--paging=always", "--pager=cat"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn delta");

    child.stdin.take().unwrap().write_all(b"").unwrap();
    let output = child.wait_with_output().unwrap();

    // --paging=always still starts the pager even when stdout is piped.
    // Using --pager=cat avoids hanging on environments where less waits for a TTY.
    assert!(
        output.status.success(),
        "delta --paging=always exited with error on empty stdin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
