//! Tests for unified VCS flags and status display.
//!
//! These tests exercise the --vcs, --vcs-status, --vcs-ignore flags and
//! their legacy aliases --git, --git-ignore.  Git-specific tests use a
//! real temporary git repository; jj tests are skipped if jj is not on PATH.

mod support;

use std::fs;
use std::process::Command as StdCommand;
use predicates::prelude::*;
use support::{lx, lx_no_colour};
use tempfile::tempdir;


/// Create a temporary git repo with a tracked file and an untracked file.
fn git_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let path = dir.path();

    // Initialise a git repo.
    StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    // Create and commit a tracked file.
    fs::write(path.join("tracked.txt"), "hello").unwrap();
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(path)
        .output()
        .expect("git add failed");
    StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    // Create an untracked file (will show as New).
    fs::write(path.join("untracked.txt"), "new").unwrap();

    // Modify the tracked file (will show as Modified).
    fs::write(path.join("tracked.txt"), "modified").unwrap();

    dir
}

/// Whether `jj` is available on PATH.
fn jj_available() -> bool {
    StdCommand::new("jj")
        .arg("version")
        .output()
        .is_ok_and(|o| o.status.success())
}


// ── --vcs flag validation ─────────────────────────────────────────

#[test]
fn vcs_invalid_value() {
    lx()
        .arg("--vcs=svn")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn vcs_none_disables_status() {
    let dir = git_fixture();
    // With --vcs=none, the status column should not appear even if
    // --vcs-status is given.
    lx_no_colour()
        .args(["--vcs=none", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        // No status characters (M, N, -, etc.) before filenames.
        .stdout(predicate::str::contains(" M ").not())
        .stdout(predicate::str::contains(" N ").not());
}


// ── Git backend ───────────────────────────────────────────────────

#[test]
fn git_vcs_status_shows_column() {
    let dir = git_fixture();
    lx_no_colour()
        .args(["--vcs=git", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        // untracked.txt should show as New (N) in the unstaged column.
        .stdout(predicate::str::contains("N"))
        .stdout(predicate::str::contains("untracked.txt"));
}

#[test]
fn git_modified_file_shows_m() {
    let dir = git_fixture();
    lx_no_colour()
        .args(["--vcs=git", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("M"))
        .stdout(predicate::str::contains("tracked.txt"));
}

#[test]
fn git_vcs_status_header() {
    let dir = git_fixture();
    lx_no_colour()
        .args(["--vcs=git", "--vcs-status", "-lh"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("VCS"));
}

#[test]
fn git_vcs_ignore_hides_ignored_files() {
    let dir = git_fixture();
    let path = dir.path();

    // Create a .gitignore that ignores *.log files.
    fs::write(path.join(".gitignore"), "*.log\n").unwrap();
    StdCommand::new("git")
        .args(["add", ".gitignore"])
        .current_dir(path)
        .output()
        .expect("git add failed");

    fs::write(path.join("debug.log"), "log output").unwrap();

    lx_no_colour()
        .args(["--vcs=git", "--vcs-ignore", "-1"])
        .arg(path)
        .assert()
        .success()
        .stdout(predicate::str::contains("debug.log").not())
        .stdout(predicate::str::contains("tracked.txt"));
}


// ── Legacy --git / --git-ignore aliases ───────────────────────────

#[test]
fn legacy_git_flag_shows_status() {
    let dir = git_fixture();
    lx_no_colour()
        .args(["--git", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("N"))
        .stdout(predicate::str::contains("untracked.txt"));
}

#[test]
fn legacy_git_ignore_flag() {
    let dir = git_fixture();
    let path = dir.path();

    fs::write(path.join(".gitignore"), "*.tmp\n").unwrap();
    StdCommand::new("git")
        .args(["add", ".gitignore"])
        .current_dir(path)
        .output()
        .expect("git add failed");

    fs::write(path.join("scratch.tmp"), "temp").unwrap();

    lx_no_colour()
        .args(["--git-ignore", "-1"])
        .arg(path)
        .assert()
        .success()
        .stdout(predicate::str::contains("scratch.tmp").not());
}


// ── --vcs-status without --long (silently ignored) ────────────────

#[test]
fn vcs_status_without_long_is_fine() {
    lx_no_colour()
        .args(["--vcs-status", "."])
        .assert()
        .success();
}


// ── jj backend (only runs if jj is installed) ─────────────────────

#[test]
fn jj_vcs_status_in_this_repo() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    // This repo itself is a jj workspace; list it with --vcs=jj.
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-status", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn jj_single_column_display() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    // jj status should show single-column (char + space), not double.
    // Create a file that has changes in the working copy.
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-status", "-l", "."])
        .assert()
        .success()
        // Look for single-char status patterns (letter + space before filename).
        // The key point: no "MM" or "NN" — jj always shows "M " or "- ".
        .stdout(predicate::str::contains("MM").not())
        .stdout(predicate::str::contains("NN").not());
}

#[test]
fn jj_auto_detection() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    // With --vcs=auto in this repo (which has .jj/), jj should be preferred.
    // The output should use single-column display (jj style).
    lx_no_colour()
        .args(["--vcs=auto", "--vcs-status", "-l", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("MM").not());
}


// ── --vcs=auto fallback to git ────────────────────────────────────

#[test]
fn auto_falls_back_to_git() {
    let dir = git_fixture();
    // This tempdir has .git/ but no .jj/, so auto should use git.
    lx_no_colour()
        .args(["--vcs=auto", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        // Git shows two-column status for differing staged/unstaged.
        .stdout(predicate::str::contains("N"))
        .stdout(predicate::str::contains("untracked.txt"));
}
