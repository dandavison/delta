// These run the delta binary, which prints nothing when CI runs the Linux ARM tests under emulation
// with cross.
#![cfg(all(
    feature = "gitu",
    not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))
))]

use std::process::{Command, Output};

#[test]
fn test_gitu_is_an_alias_of_tui() {
    for arg in ["--version", "--help"] {
        assert_eq!(delta(&["--gitu", arg]), delta(&["--tui", arg]));
    }
}

#[test]
fn test_tui_help_uses_tui() {
    // --tui, as delta 0.22.0 ran the TUI, still does.
    for tui in ["tui", "--tui"] {
        let help = String::from_utf8(delta(&[tui, "--help"]).stdout).unwrap();
        assert!(help.contains("Usage: delta tui"), "{}", help);
    }
}

// FIXME: stop ignoring this when delta depends on a delta-gitu that has --agent-instructions, and
// revert the change that makes delta --agent-instructions print nothing until then.
#[test]
#[ignore]
fn test_agent_instructions_are_the_tuis() {
    let text = String::from_utf8(delta(&["--agent-instructions"]).stdout).unwrap();
    assert!(text.starts_with("---\nname: "), "{}", text);
    let tui = String::from_utf8(delta(&["tui", "--agent-instructions"]).stdout).unwrap();
    assert_eq!(text, tui);
}

fn delta(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_delta"))
        .args(args)
        .output()
        .unwrap()
}
