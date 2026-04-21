//! Tests for --total-size column.

mod support;

use predicates::prelude::*;
use std::fs;
use support::lx_no_colour;
use tempfile::tempdir;

#[test]
fn total_size_shows_for_directories() {
    let dir = tempdir().expect("failed to create tempdir");
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("file1.txt"), "hello").unwrap(); // 5 bytes
    fs::write(sub.join("file2.txt"), "world!!!").unwrap(); // 8 bytes

    lx_no_colour()
        .args(["-l", "--total-size"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir"));
}

#[test]
fn total_size_via_columns() {
    let dir = tempdir().expect("failed to create tempdir");
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("a.txt"), "abc").unwrap();

    lx_no_colour()
        .args(["--columns=size", "-Z"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn total_size_sort() {
    let dir = tempdir().expect("failed to create tempdir");

    // Small dir
    let small = dir.path().join("small");
    fs::create_dir(&small).unwrap();
    fs::write(small.join("tiny.txt"), "x").unwrap();

    // Big dir
    let big = dir.path().join("big");
    fs::create_dir(&big).unwrap();
    fs::write(big.join("large.txt"), "x".repeat(10000)).unwrap();

    // Sort by size with --total-size: big should come after small
    // (ascending order)
    lx_no_colour()
        .args(["-1D", "--total-size", "-s", "size"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let small_pos = output.find("small").unwrap();
            let big_pos = output.find("big").unwrap();
            small_pos < big_pos
        }));
}

#[test]
fn total_size_sort_reversed() {
    let dir = tempdir().expect("failed to create tempdir");

    let small = dir.path().join("small");
    fs::create_dir(&small).unwrap();
    fs::write(small.join("tiny.txt"), "x").unwrap();

    let big = dir.path().join("big");
    fs::create_dir(&big).unwrap();
    fs::write(big.join("large.txt"), "x".repeat(10000)).unwrap();

    // Reversed: big first
    lx_no_colour()
        .args(["-1D", "--total-size", "-rs", "size"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let big_pos = output.find("big").unwrap();
            let small_pos = output.find("small").unwrap();
            big_pos < small_pos
        }));
}

#[test]
fn stree_style() {
    // The full stree experience
    let dir = tempdir().expect("failed to create tempdir");

    let a = dir.path().join("alpha");
    fs::create_dir(&a).unwrap();
    fs::write(a.join("data.bin"), vec![0u8; 5000]).unwrap();

    let b = dir.path().join("beta");
    fs::create_dir(&b).unwrap();
    fs::write(b.join("small.txt"), "hi").unwrap();

    lx_no_colour()
        .args(["-DT", "-L1", "--columns=size", "-rs", "size", "-Z"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"));
}
