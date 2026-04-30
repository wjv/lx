//! Tests for the -@ count flag, --xattr alias, --xattr-indicator
//! config key, and the macOS-overlay default for the indicator.

mod support;

use predicates::prelude::*;
use std::fs;
use support::{lx, lx_no_colour};
use tempfile::tempdir;

// ── --save-as round-trip for the -@ count flag ─────────────────

/// Create a config-and-home env so `--save-as` writes inside the
/// tempdir, then return the conf.d/ output path.
fn save_as_and_read(args: &[&str], name: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let home = dir.path();
    let conf_d = home.join(".config/lx/conf.d");
    fs::create_dir_all(&conf_d).unwrap();

    let mut cmd = lx();
    cmd.env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .args(args)
        .arg(format!("--save-as={name}"));
    cmd.assert().success();

    fs::read_to_string(conf_d.join(format!("{name}.toml"))).expect("save-as output")
}

#[test]
fn save_as_at_count_1_emits_xattr_indicator() {
    let toml = save_as_and_read(&["-@"], "test_at1");
    assert!(toml.contains("xattr-indicator = true"), "got: {toml}");
    assert!(!toml.contains("extended = true"), "got: {toml}");
}

#[test]
fn save_as_at_count_2_emits_both_keys() {
    let toml = save_as_and_read(&["-@@"], "test_at2");
    assert!(toml.contains("xattr-indicator = true"), "got: {toml}");
    assert!(toml.contains("extended = true"), "got: {toml}");
}

#[test]
fn save_as_xattr_alias_count_1_emits_indicator_only() {
    let toml = save_as_and_read(&["--xattr"], "test_xattr1");
    assert!(toml.contains("xattr-indicator = true"), "got: {toml}");
    assert!(!toml.contains("extended = true"), "got: {toml}");
}

#[test]
fn save_as_xattr_alias_doubled_emits_both() {
    let toml = save_as_and_read(&["--xattr", "--xattr"], "test_xattr2");
    assert!(toml.contains("xattr-indicator = true"), "got: {toml}");
    assert!(toml.contains("extended = true"), "got: {toml}");
}

// ── --xattr-indicator hidden flag ──────────────────────────────

#[test]
fn xattr_indicator_flag_runs_successfully() {
    // The hidden flag should be accepted on the command line.
    lx_no_colour()
        .args(["-l", "--xattr-indicator", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn xattr_indicator_flag_hidden_from_help() {
    lx_no_colour()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--xattr-indicator").not());
}

// ── --xattr is a visible alias for --extended ──────────────────

#[test]
fn xattr_alias_appears_in_help() {
    lx_no_colour()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--xattr"));
}

// ── macOS overlay personality test ─────────────────────────────

#[test]
fn show_config_includes_xattr_indicator_in_lx_chain() {
    // The compiled-in `lx` personality (via `default`) declares
    // `xattr-indicator`.  On macOS the `[[when]] platform = "macos"`
    // overlay flips it off; on other platforms it stays on.
    // Either way, --show-config should mention the key.
    lx_no_colour()
        .arg("--show-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("xattr-indicator"));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_default_disables_xattr_indicator() {
    // On macOS, the [[when]] block flips xattr-indicator to false.
    lx_no_colour()
        .arg("--show-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("xattr-indicator").and(predicate::str::contains("false")));
}

#[test]
#[cfg(not(target_os = "macos"))]
fn non_macos_default_enables_xattr_indicator() {
    lx_no_colour()
        .arg("--show-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("xattr-indicator").and(predicate::str::contains("true")));
}
