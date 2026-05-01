//! Tests for unified VCS flags and status display.
//!
//! These tests exercise the --vcs, --vcs-status, --vcs-ignore flags and
//! their legacy aliases --git, --git-ignore.  Git-specific tests use a
//! real temporary git repository; jj tests are skipped if jj is not on PATH.

mod support;

use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
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

/// Whether this lx binary was built with the `jj` feature.
fn jj_feature_enabled() -> bool {
    // Probe the binary: --vcs=jj with a non-existent path.  If the feature
    // is disabled, lx exits with an error mentioning "disabled".
    let output = lx_no_colour()
        .args(["--vcs=jj", "/nonexistent"])
        .output()
        .expect("failed to run lx");
    let stderr = String::from_utf8_lossy(&output.stderr);
    !stderr.contains("disabled")
}

// ── --vcs flag validation ─────────────────────────────────────────

#[test]
fn vcs_invalid_value() {
    lx().arg("--vcs=svn")
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
        .stdout(predicate::str::contains("Git"));
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
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
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
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
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
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
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

// ── non-colocated jj repo fixtures ────────────────────────────────

/// Run `jj` in `cwd` with the given args.  Panics on failure.
fn jj_run(cwd: &std::path::Path, args: &[&str]) {
    let output = StdCommand::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("jj command failed to spawn");
    assert!(
        output.status.success(),
        "jj {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create a non-colocated jj repo (`.jj/` only, no `.git/` at root).
/// Includes a tracked file, an untracked file, and a `.gitignore`
/// covering `*.log` plus an `ignored.log` file.
fn jj_non_colocated_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let path = dir.path();

    // --no-colocate keeps .git/ out of the workspace root; the git
    // store lives inside .jj/repo/store/git/ as a bare repo.
    jj_run(path, &["git", "init", "--no-colocate"]);

    fs::write(path.join("tracked.txt"), "hello").unwrap();
    fs::write(path.join(".gitignore"), "*.log\n").unwrap();
    fs::write(path.join("ignored.log"), "noise").unwrap();
    jj_run(path, &["describe", "-m", "initial"]);
    jj_run(path, &["new"]);
    fs::write(path.join("untracked.txt"), "new").unwrap();
    // jj-lib doesn't auto-snapshot when invoked as a library; force
    // a snapshot so the file shows up as added in the working copy.
    jj_run(path, &["status"]);

    dir
}

/// Create a jj repo backed by an *external* git repo
/// (`jj git init --git-repo <path>`).  The jj working directory and
/// the git working directory are the same; jj treats this as a
/// colocated layout pointing at an existing git repo.
fn jj_external_git_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let path = dir.path();

    // Bootstrap a git repo with a commit.
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
    fs::write(path.join("tracked.txt"), "hello").unwrap();
    fs::write(path.join(".gitignore"), "*.log\n").unwrap();
    StdCommand::new("git")
        .args(["add", "tracked.txt", ".gitignore"])
        .current_dir(path)
        .output()
        .expect("git add failed");
    StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    // Layer jj on top of the existing git repo.
    jj_run(path, &["git", "init", "--git-repo", "."]);

    fs::write(path.join("ignored.log"), "noise").unwrap();
    fs::write(path.join("untracked.txt"), "new").unwrap();
    // Force a snapshot so the post-init writes are visible to jj-lib.
    jj_run(path, &["status"]);

    dir
}

// ── non-colocated jj behaviour ────────────────────────────────────

#[test]
fn jj_non_colocated_auto_picks_jj() {
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
        return;
    }

    let dir = jj_non_colocated_fixture();
    // --vcs=auto should pick jj because .jj/ is present and .git/
    // is not at the workspace root.  Single-column status is the
    // jj-specific signal we look for.
    lx_no_colour()
        .args(["--vcs=auto", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.txt"))
        .stdout(predicate::str::contains("MM").not())
        .stdout(predicate::str::contains("NN").not());
}

#[test]
fn jj_non_colocated_status_column_works() {
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
        return;
    }

    let dir = jj_non_colocated_fixture();
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        // The newly-created file should show A (added) in jj.
        .stdout(predicate::str::contains("untracked.txt"))
        .stdout(predicate::str::contains("A"));
}

#[test]
fn jj_non_colocated_vcs_ignore_hides_ignored_files() {
    // Regression test: before the set_workdir patch, --vcs-ignore
    // did nothing on non-colocated repos because the bare git store
    // had no working directory and git2's is_path_ignored returned
    // false unconditionally.
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
        return;
    }

    let dir = jj_non_colocated_fixture();
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-ignore"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.txt"))
        .stdout(predicate::str::contains("untracked.txt"))
        .stdout(predicate::str::contains("ignored.log").not());
}

#[test]
fn jj_external_git_repo_status_works() {
    // jj git init --git-repo <existing-git-repo> creates a layout
    // that's colocated in placement but uses a pre-existing git
    // store rather than letting jj manage one.
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
        return;
    }

    let dir = jj_external_git_fixture();
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-status", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.txt"))
        .stdout(predicate::str::contains("untracked.txt"));
}

#[test]
fn jj_external_git_repo_vcs_ignore_works() {
    if !jj_feature_enabled() || !jj_available() {
        eprintln!("skipping: jj feature disabled or jj not available");
        return;
    }

    let dir = jj_external_git_fixture();
    lx_no_colour()
        .args(["--vcs=jj", "--vcs-ignore"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.txt"))
        .stdout(predicate::str::contains("ignored.log").not());
}
