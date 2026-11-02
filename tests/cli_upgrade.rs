//! Tests for config versions and `--upgrade-config`.

mod support;

use predicates::prelude::*;
use std::fs;
use support::lx_no_colour;
use tempfile::tempdir;

/// A config written for a later lx gets a clear error on an ordinary
/// run, not advice to "upgrade" it to an older version.
#[test]
fn newer_config_says_lx_is_too_old() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    fs::write(&path, "version = \"9.9\"\n").unwrap();

    lx_no_colour()
        .env("LX_CONFIG", &path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "uses version 9.9, which is newer than this lx understands",
        ))
        .stderr(predicate::str::contains("--upgrade-config").not());
}

/// `--upgrade-config` refuses a newer config and leaves it untouched,
/// instead of rebuilding it from scratch.
#[test]
fn upgrade_of_a_newer_config_is_refused() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let newer = "# keep me\nversion = \"9.9\"\n\n[theme.ocean]\ndirectory = \"blue\"\n";
    fs::write(&path, newer).unwrap();

    lx_no_colour()
        .env("LX_CONFIG", &path)
        .arg("--upgrade-config")
        .assert()
        .failure()
        .stderr(predicate::str::contains("newer than this lx understands"));

    assert_eq!(fs::read_to_string(&path).unwrap(), newer);
    assert!(!dir.path().join("config.toml.bak").exists());
}

/// The case that prompted the fix: a 0.7 config (what 0.11 writes)
/// read by 0.10, whose own schema is 0.6.
#[test]
fn config_from_the_next_release_is_newer() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    fs::write(&path, "version = \"0.7\"\n").unwrap();

    lx_no_colour()
        .env("LX_CONFIG", &path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "uses version 0.7, which is newer than this lx understands (up to 0.6)",
        ));
}
