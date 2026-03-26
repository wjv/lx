//! Tests for file filtering: --ignore (-I), --prune (-P).

mod support;

use std::fs;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


/// Create a fixture directory with subdirectories and files.
fn filtering_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let root = dir.path();

    // Directories
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("target/debug")).unwrap();
    fs::create_dir_all(root.join("node_modules/foo")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    // Files
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("target/debug/binary"), "ELF").unwrap();
    fs::write(root.join("node_modules/foo/index.js"), "module").unwrap();
    fs::write(root.join("docs/readme.md"), "# Docs").unwrap();
    fs::write(root.join("Cargo.toml"), "[package]").unwrap();
    fs::write(root.join("notes.tmp"), "scratch").unwrap();

    dir
}


// ── --ignore / -I ────────────────────────────────────────────────

#[test]
fn ignore_hides_matching_files() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-1", "-I", "*.tmp"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("notes.tmp").not())
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn ignore_hides_matching_directories() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-1", "-I", "target"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("target").not())
        .stdout(predicate::str::contains("src"));
}

#[test]
fn ignore_glob_alias_still_works() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-1", "--ignore-glob", "*.tmp"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("notes.tmp").not());
}


// ── --prune / -P ─────────────────────────────────────────────────

#[test]
fn prune_shows_directory_but_hides_children_in_tree() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-T", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        // Directory itself is shown
        .stdout(predicate::str::contains("target"))
        // But its children are not
        .stdout(predicate::str::contains("debug").not())
        .stdout(predicate::str::contains("binary").not())
        // Other directories still recurse normally
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn prune_shows_directory_but_hides_children_in_recurse() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-R", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("debug").not());
}

#[test]
fn prune_multiple_patterns() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-T", "-P", "target|node_modules"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("node_modules"))
        // Neither should have children shown
        .stdout(predicate::str::contains("debug").not())
        .stdout(predicate::str::contains("foo").not())
        // Unpruned dirs still recurse
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn prune_with_total_size() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-lTZ", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        // target is shown with a size (not "-")
        .stdout(predicate::str::contains("target"))
        // children not shown
        .stdout(predicate::str::contains("debug").not());
}

#[test]
fn prune_does_not_affect_non_recursive_listing() {
    let dir = filtering_fixture();

    // Without -T or -R, --prune has no visible effect
    // (directories aren't recursed anyway).
    lx_no_colour()
        .args(["-1", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("target"));
}

#[test]
fn prune_short_flag() {
    let dir = filtering_fixture();

    lx_no_colour()
        .args(["-T", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("debug").not());
}

#[test]
fn ignore_and_prune_compose() {
    let dir = filtering_fixture();

    // -I hides tmp files entirely, -P prunes target
    lx_no_colour()
        .args(["-T", "-I", "*.tmp", "-P", "target"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("notes.tmp").not())
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("debug").not())
        .stdout(predicate::str::contains("main.rs"));
}
