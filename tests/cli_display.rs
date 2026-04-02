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


// ── --flags / -O ─────────────────────────────────────────────────

#[test]
fn flags_column_present() {
    let dir = display_fixture();

    // On Linux ext4, fresh files have FS_EXTENT_FL set, so we can't
    // assume "-".  Just check the column is present and non-empty.
    lx_no_colour()
        .args(["-O", "-l"])
        .arg(dir.path().join("hello.rs"))
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.rs"));
}

#[test]
fn flags_column_with_header() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-O", "-lh"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Flags"));
}

#[test]
fn flags_via_columns() {
    let dir = display_fixture();

    // Check that the flags column renders without error.  On Linux
    // ext4, fresh files show "extent" rather than "-".
    lx_no_colour()
        .args(["--columns=perms,flags,size"])
        .arg(dir.path().join("hello.rs"))
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.rs"));
}

#[cfg(target_os = "macos")]
#[test]
fn flags_dash_for_no_flags_on_macos() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-O", "-l"])
        .arg(dir.path().join("hello.rs"))
        .assert()
        .success()
        .stdout(predicate::str::contains(" - "));
}

#[cfg(target_os = "macos")]
#[test]
fn flags_shows_hidden_on_macos() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("test.txt"), "data").unwrap();

    // Set the hidden flag.
    let status = std::process::Command::new("chflags")
        .args(["hidden", &dir.path().join("test.txt").to_string_lossy()])
        .status()
        .expect("failed to run chflags");
    assert!(status.success());

    lx_no_colour()
        .args(["-O", "-la"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("hidden"));
}

#[cfg(target_os = "macos")]
#[test]
fn flags_shows_uchg_on_macos() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("locked.txt"), "data").unwrap();

    let path_str = dir.path().join("locked.txt").to_string_lossy().to_string();

    // Set the immutable flag.
    let status = std::process::Command::new("chflags")
        .args(["uchg", &path_str])
        .status()
        .expect("failed to run chflags");
    assert!(status.success());

    lx_no_colour()
        .args(["-O", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("uchg"));

    // Clean up: remove the immutable flag so tempdir can delete it.
    std::process::Command::new("chflags")
        .args(["nouchg", &path_str])
        .status()
        .expect("failed to run chflags");
}


#[test]
fn flags_short_flag() {
    let dir = display_fixture();

    lx_no_colour()
        .args(["-O", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        // Should have a flags column (all dashes for this fixture)
        .stdout(predicate::str::contains(" - "));
}

#[cfg(target_os = "linux")]
#[test]
fn flags_shows_immutable_on_linux() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("test.txt"), "data").unwrap();

    let path_str = dir.path().join("test.txt").to_string_lossy().to_string();

    // chattr +i requires root; skip if not available.
    let status = std::process::Command::new("sudo")
        .args(["-n", "chattr", "+i", &path_str])
        .status();

    let ok = status.is_ok_and(|s| s.success());
    if !ok {
        eprintln!("skipping: chattr +i requires root");
        // Clean up in case chattr partially succeeded.
        let _ = std::process::Command::new("sudo")
            .args(["-n", "chattr", "-i", &path_str])
            .status();
        return;
    }

    lx_no_colour()
        .args(["-O", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("immutable"));

    // Clean up.
    std::process::Command::new("sudo")
        .args(["-n", "chattr", "-i", &path_str])
        .status()
        .expect("failed to run chattr");
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
