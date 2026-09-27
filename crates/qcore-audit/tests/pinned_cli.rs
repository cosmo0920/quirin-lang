//! Built-in pinned command parses before any expensive source replay.
use std::process::Command;

const HOST: &str = env!("CARGO_BIN_EXE_quirin-audit");

#[test]
fn pinned_help_and_invalid_options_do_not_start_a_replay() {
    let help = Command::new(HOST)
        .args(["pinned", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("usage: quirin-audit pinned"));
    for args in [
        vec!["pinned", "--not-a-flag"],
        vec!["pinned", "--jobs", "1", "--jobs", "2"],
        vec!["pinned", "--jobs", "0"],
    ] {
        let output = Command::new(HOST).args(&args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!error.contains("could not resolve"), "{args:?}: {error}");
    }
}
