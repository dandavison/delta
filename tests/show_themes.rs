use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const GIT_CONFIG: &str = r#"
[delta "test-dark-theme"]
    dark = true
[delta "test-light-theme"]
    light = true
"#;

// delta locates themes in the global git config, so give the child process its own HOME.
fn make_home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("delta-test-{name}-{}", std::process::id()));
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(".gitconfig"), GIT_CONFIG).unwrap();
    home
}

fn show_themes(home: &Path, args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_delta"))
        .args(["--show-themes", "--detect-dark-light", "never"])
        .args(args)
        .env("HOME", home)
        .env("DELTA_PAGER", "cat")
        .current_dir(home)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn test_show_themes_dark_and_light() {
    let home = make_home("show-themes");

    let (ok, out) = show_themes(&home, &["--dark", "--light"]);
    assert!(ok, "--show-themes --dark --light exited with an error");
    assert!(out.contains("test-dark-theme"), "no dark theme in: {}", out);
    assert!(
        out.contains("test-light-theme"),
        "no light theme in: {}",
        out
    );

    let (ok, out) = show_themes(&home, &["--dark"]);
    assert!(ok);
    assert!(out.contains("test-dark-theme"));
    assert!(!out.contains("test-light-theme"));

    let (ok, out) = show_themes(&home, &["--light"]);
    assert!(ok);
    assert!(out.contains("test-light-theme"));
    assert!(!out.contains("test-dark-theme"));

    fs::remove_dir_all(&home).unwrap();
}
