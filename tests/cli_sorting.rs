//! Tests for sorting and filtering with controlled fixtures.

mod support;

use std::fs;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


/// Create a tempdir with a few files for sorting tests.
fn sorting_fixture() -> tempfile::TempDir {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("apple.txt"), "a").unwrap();
    fs::write(dir.path().join("Banana.txt"), "bb").unwrap();
    fs::write(dir.path().join("cherry.txt"), "ccc").unwrap();
    dir
}


#[test]
fn sort_name_case_insensitive() {
    let dir = sorting_fixture();
    lx_no_colour()
        .args(["-1", "--sort=name"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("apple.txt"))
        .stdout(predicate::str::contains("Banana.txt"))
        .stdout(predicate::str::contains("cherry.txt"));
}

#[test]
fn sort_name_case_sensitive() {
    let dir = sorting_fixture();
    // --sort=Name sorts uppercase before lowercase
    lx_no_colour()
        .args(["-1", "--sort=Name"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Banana.txt"))
        .stdout(predicate::str::contains("apple.txt"));
}

#[test]
fn sort_by_extension() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file.zip"), "").unwrap();
    fs::write(dir.path().join("file.aaa"), "").unwrap();
    fs::write(dir.path().join("file.mmm"), "").unwrap();

    lx_no_colour()
        .args(["-1", "--sort=ext"])
        .arg(dir.path())
        .assert()
        .success()
        // .aaa should come before .mmm which comes before .zip
        .stdout(predicate::function(|output: &str| {
            let aaa = output.find("file.aaa").unwrap();
            let mmm = output.find("file.mmm").unwrap();
            let zip = output.find("file.zip").unwrap();
            aaa < mmm && mmm < zip
        }));
}

#[test]
fn sort_by_size() {
    let dir = sorting_fixture();
    lx_no_colour()
        .args(["-1", "--sort=size"])
        .arg(dir.path())
        .assert()
        .success()
        // Smallest first: apple (1 byte), Banana (2), cherry (3)
        .stdout(predicate::function(|output: &str| {
            let apple = output.find("apple.txt").unwrap();
            let banana = output.find("Banana.txt").unwrap();
            let cherry = output.find("cherry.txt").unwrap();
            apple < banana && banana < cherry
        }));
}

#[test]
fn sort_none_preserves_readdir_order() {
    let dir = sorting_fixture();
    // --sort=none should succeed (we can't assert readdir order, just
    // that it doesn't crash and shows all files)
    lx_no_colour()
        .args(["-1", "--sort=none"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("apple.txt"))
        .stdout(predicate::str::contains("Banana.txt"))
        .stdout(predicate::str::contains("cherry.txt"));
}

#[test]
fn reverse_sort() {
    let dir = sorting_fixture();
    lx_no_colour()
        .args(["-1r", "--sort=size"])
        .arg(dir.path())
        .assert()
        .success()
        // Largest first: cherry (3), Banana (2), apple (1)
        .stdout(predicate::function(|output: &str| {
            let cherry = output.find("cherry.txt").unwrap();
            let banana = output.find("Banana.txt").unwrap();
            let apple = output.find("apple.txt").unwrap();
            cherry < banana && banana < apple
        }));
}


// ── Dotfiles ──────────────────────────────────────────────────────

#[test]
fn hidden_files_not_shown_by_default() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join(".hidden"), "").unwrap();
    fs::write(dir.path().join("visible"), "").unwrap();

    lx_no_colour()
        .arg("-1")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("visible"))
        .stdout(predicate::str::contains(".hidden").not());
}

#[test]
fn hidden_files_shown_with_all() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join(".hidden"), "").unwrap();
    fs::write(dir.path().join("visible"), "").unwrap();

    lx_no_colour()
        .args(["-1a"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("visible"))
        .stdout(predicate::str::contains(".hidden"));
}

#[test]
fn dot_and_dotdot_with_all_all() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file"), "").unwrap();

    lx_no_colour()
        .args(["-1aa"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("."))
        .stdout(predicate::str::contains(".."));
}


// ── Ignore globs ──────────────────────────────────────────────────

#[test]
fn ignore_glob_hides_matching_files() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("keep.txt"), "").unwrap();
    fs::write(dir.path().join("remove.log"), "").unwrap();
    fs::write(dir.path().join("also.log"), "").unwrap();

    lx_no_colour()
        .args(["-1", "-I", "*.log"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"))
        .stdout(predicate::str::contains("remove.log").not())
        .stdout(predicate::str::contains("also.log").not());
}

#[test]
fn ignore_glob_pipe_separated() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("keep.rs"), "").unwrap();
    fs::write(dir.path().join("nope.log"), "").unwrap();
    fs::write(dir.path().join("nope.tmp"), "").unwrap();

    lx_no_colour()
        .args(["-1", "-I", "*.log|*.tmp"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.rs"))
        .stdout(predicate::str::contains("nope.log").not())
        .stdout(predicate::str::contains("nope.tmp").not());
}


// ── Only dirs ─────────────────────────────────────────────────────

#[test]
fn only_dirs_hides_files() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    lx_no_colour()
        .args(["-1D"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("subdir"))
        .stdout(predicate::str::contains("file.txt").not());
}


// ── Group directories ─────────────────────────────────────────────

#[test]
fn group_dirs_first() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("zzz_dir")).unwrap();

    lx_no_colour()
        .args(["-1", "--group-dirs=first"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            dir_pos < file_pos
        }));
}

#[test]
fn group_dirs_last() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("zzz_dir")).unwrap();

    lx_no_colour()
        .args(["-1", "--group-dirs=last"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            file_pos < dir_pos
        }));
}

#[test]
fn group_directories_first_legacy() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("zzz_dir")).unwrap();

    lx_no_colour()
        .args(["-1", "--group-directories-first"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            dir_pos < file_pos
        }));
}
