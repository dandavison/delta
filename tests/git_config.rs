#![cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_git_config_global_outside_repository() {
    let dir = TempDir::new();
    let config = write_config(&dir.0);

    let output = delta(&dir.0, &config);

    assert_eq!(config_value(&output, "line-numbers"), Some("true"));
}

#[test]
fn test_git_config_global_inside_repository() {
    let dir = TempDir::new();
    let config = write_config(&dir.0);
    drop(git2::Repository::init(&dir.0).unwrap());

    let output = delta(&dir.0, &config);

    assert_eq!(config_value(&output, "line-numbers"), Some("true"));
}

fn delta(dir: &Path, global_config: &Path) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_delta"))
        .arg("--show-config")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", global_config)
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_NOSYSTEM")
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    output
}

fn config_value<'a>(output: &'a Output, key: &str) -> Option<&'a str> {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    stdout.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim())
    })
}

fn write_config(dir: &Path) -> PathBuf {
    let path = dir.join("gitconfig");
    fs::write(&path, "[delta]\n    line-numbers = true\n").unwrap();
    path
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "delta-git-config-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
