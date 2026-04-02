//! Tests for -C/--count.

mod support;

use std::fs;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


fn count_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("alpha.txt"), "a").unwrap();
    fs::write(root.join("beta.txt"), "b").unwrap();
    fs::write(root.join("gamma.txt"), "c").unwrap();
    fs::write(root.join("sub/delta.txt"), "d").unwrap();

    dir
}


#[test]
fn count_flat_listing() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("4 items"));
}

#[test]
fn count_with_long_view() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-l"])
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("4 items"));
}

#[test]
fn count_recursive() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-R", "-1"])
        .arg(dir.path())
        .assert()
        .success()
        // 4 top-level + 1 in sub = 5
        .stderr(predicate::str::contains("5 items"));
}

#[test]
fn count_tree() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-T"])
        .arg(dir.path())
        .assert()
        .success()
        // Tree shows all entries: 3 files + sub/ + sub/delta.txt + top dir name = 6
        .stderr(predicate::str::contains("6 items"));
}

#[test]
fn count_with_filter() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-1", "-I", "*.txt"])
        .arg(dir.path())
        .assert()
        .success()
        // Only the sub/ directory remains after filtering
        .stderr(predicate::str::contains("1 items"));
}

#[test]
fn count_only_dirs() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-1", "-D"])
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("1 items"));
}

#[test]
fn count_only_files() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-C", "-1", "-f"])
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("3 items"));
}

#[test]
fn no_count_without_flag() {
    let dir = count_fixture();

    lx_no_colour()
        .args(["-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("items").not());
}
