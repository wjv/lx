//! Tests for display-related flags: --absolute, --classify, --octal,
//! --only-files, --quotes, --width.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


fn display_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("subdir")).unwrap();
    fs::write(root.join("hello.rs"), "fn main() {}").unwrap();
    fs::write(root.join("readme.md"), "# Hello").unwrap();
    fs::write(root.join("space file.txt"), "spaces").unwrap();

    // Make an executable
    fs::write(root.join("run.sh"), "#!/bin/sh").unwrap();
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(root.join("run.sh"), perms).unwrap();

    dir
}


// ── --absolute / -A ──────────────────────────────────────────────

#[test]
fn absolute_shows_full_paths() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-A", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(dir.path().to_string_lossy().as_ref()));
}

#[test]
fn no_absolute_shows_relative_names() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(dir.path().to_string_lossy().as_ref()).not());
}


// ── --classify ───────────────────────────────────────────────────

#[test]
fn classify_always_shows_indicators() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["--classify=always", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir/"))
        .stdout(predicate::str::contains("run.sh*"));
}

#[test]
fn classify_never_hides_indicators() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["--classify=never", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir/").not())
        .stdout(predicate::str::contains("run.sh\n"));
}


// ── --octal / -o ─────────────────────────────────────────────────

#[test]
fn octal_shows_permissions() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-o", "-l"])
        .arg(dir.path().join("hello.rs"))
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"0\d{3}").unwrap());
}

#[test]
fn octal_permissions_alias_works() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["--octal-permissions", "-l"])
        .arg(dir.path().join("hello.rs"))
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"0\d{3}").unwrap());
}


// ── --only-files / -f ────────────────────────────────────────────

#[test]
fn only_files_hides_directories() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-f", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir").not())
        .stdout(predicate::str::contains("hello.rs"));
}


// ── --quotes ─────────────────────────────────────────────────────

#[test]
fn quotes_always_wraps_spaces() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["--quotes=always", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"space file.txt\""));
}

#[test]
fn quotes_never_no_wrapping() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["--quotes=never", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"space file.txt\"").not())
        .stdout(predicate::str::contains("space file.txt"));
}


// ── --width / -w ─────────────────────────────────────────────────

#[test]
fn width_controls_grid() {
    let dir = display_fixture();

    // Very narrow width should force oneline-style output
    lx_no_colour()
        .args(["-w", "20"])
        .arg(dir.path())
        .assert()
        .success();
}
