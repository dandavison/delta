use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn delta_bin() -> OsString {
    std::env::var_os("CARGO_BIN_EXE_delta").unwrap_or_else(|| {
        // Fallback for when not run via cargo test
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("target");
        path.push("debug");
        path.push(format!("delta{}", std::env::consts::EXE_SUFFIX));
        path.into_os_string()
    })
}

#[test]
fn test_no_output_on_empty_stdin_when_stdout_piped() {
    let mut child = Command::new(delta_bin())
        .args(["--no-gitconfig"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
    // Use a non-interactive pager that exists on all platforms.
    // On Windows, `more` is available; on Unix, `cat` is available.
    let pager = if cfg!(windows) { "more" } else { "cat" };

    let mut child = Command::new(delta_bin())
        .args(["--no-gitconfig", "--paging=always", &format!("--pager={}", pager)])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn delta");

    child.stdin.take().unwrap().write_all(b"").unwrap();
    let output = child.wait_with_output().unwrap();

    // --paging=always still starts the pager even when stdout is piped.
    assert!(
        output.status.success(),
        "delta --paging=always exited with error on empty stdin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
