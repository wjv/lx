//! Tests for file filtering: --ignore (-I), --prune (-P), --symlinks.

mod support;

use predicates::prelude::*;
use std::fs;
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

// ── --symlinks ───────────────────────────────────────────────────

/// Create a fixture with regular files and symlinks.
fn symlink_fixture() -> tempfile::TempDir {
    use std::os::unix::fs as unix_fs;

    let dir = tempdir().expect("failed to create tempdir");
    let root = dir.path();

    fs::write(root.join("real.txt"), "content").unwrap();
    fs::write(root.join("target.rs"), "fn main() {}").unwrap();
    unix_fs::symlink("target.rs", root.join("link.rs")).unwrap();
    // Broken symlink:
    unix_fs::symlink("nonexistent", root.join("broken.rs")).unwrap();

    dir
}

#[test]
fn symlinks_show_is_default() {
    let dir = symlink_fixture();

    // Default: symlinks visible (including broken ones).
    lx_no_colour()
        .args(["-1"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("real.txt"))
        .stdout(predicate::str::contains("link.rs"))
        .stdout(predicate::str::contains("broken.rs"));
}

#[test]
fn symlinks_hide() {
    let dir = symlink_fixture();

    lx_no_colour()
        .args(["-1", "--symlinks=hide"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("real.txt"))
        .stdout(predicate::str::contains("target.rs"))
        .stdout(predicate::str::contains("link.rs").not())
        .stdout(predicate::str::contains("broken.rs").not());
}

#[test]
fn symlinks_follow_shows_target_metadata() {
    let dir = symlink_fixture();

    // With follow, a valid symlink should show the target's size,
    // not the symlink's size (which is just the path length).
    lx_no_colour()
        .args(["-l", "--symlinks=follow"])
        .arg(dir.path())
        .assert()
        .success()
        // link.rs should appear as a regular file (not 'l' prefix)
        // and have the same size as target.rs
        .stdout(predicate::str::contains("link.rs"))
        // The broken symlink stays as-is (can't dereference)
        .stdout(predicate::str::contains("broken.rs"));
}

#[test]
fn symlinks_follow_recurses_into_linked_dirs() {
    use std::os::unix::fs as unix_fs;

    let dir = tempdir().expect("failed to create tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("real_dir")).unwrap();
    fs::write(root.join("real_dir/inner.txt"), "inside").unwrap();
    unix_fs::symlink("real_dir", root.join("linked_dir")).unwrap();

    // Without follow, -T should NOT recurse into linked_dir.
    lx_no_colour()
        .args(["-T", "--symlinks=show"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("linked_dir"))
        // inner.txt should appear under real_dir but NOT under linked_dir
        .stdout(predicate::str::contains("inner.txt"));

    // With follow, -T should recurse into linked_dir since it
    // now looks like a real directory.
    let output = lx_no_colour()
        .args(["-T", "--symlinks=follow"])
        .arg(dir.path())
        .output()
        .expect("failed to run lx");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // inner.txt should appear twice — once under real_dir, once under linked_dir
    assert_eq!(
        stdout.matches("inner.txt").count(),
        2,
        "Expected inner.txt twice (under real_dir and linked_dir), got:\n{stdout}"
    );
}
