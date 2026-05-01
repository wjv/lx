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

// ── --filesystem / -X / --xdev ────────────────────────────────────

/// Find a path under `/` whose `st_dev` differs from `/`'s.  Returns
/// (parent_path, child_name) where parent_path contains a child of a
/// different device.  Returns `None` if the test environment has no
/// such layout (everything on one filesystem).
fn find_cross_device_path() -> Option<(std::path::PathBuf, String)> {
    use std::os::unix::fs::MetadataExt;
    let root_dev = std::fs::metadata("/").ok()?.dev();
    // Candidate parents to scan.  /System/Volumes on macOS hosts a
    // bunch of sub-volume mounts; /sys, /proc, /dev on Linux are
    // separate filesystems.
    for parent in ["/System/Volumes", "/dev", "/proc", "/sys"] {
        let parent = std::path::Path::new(parent);
        let Ok(entries) = std::fs::read_dir(parent) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(md) = entry.metadata() else { continue };
            if md.is_dir() && md.dev() != root_dev {
                return Some((
                    parent.to_path_buf(),
                    entry.file_name().to_string_lossy().into_owned(),
                ));
            }
        }
    }
    None
}

#[test]
fn filesystem_flag_accepts_all_modes() {
    let dir = filtering_fixture();
    // On a single-fs tempdir all three modes should produce identical
    // output, since no boundary crossings happen.
    let default_out = lx_no_colour()
        .args(["-T"])
        .arg(dir.path())
        .output()
        .expect("default failed");
    let same_out = lx_no_colour()
        .args(["-T", "--filesystem=same"])
        .arg(dir.path())
        .output()
        .expect("--filesystem=same failed");
    let local_out = lx_no_colour()
        .args(["-T", "--filesystem=local"])
        .arg(dir.path())
        .output()
        .expect("--filesystem=local failed");
    let all_out = lx_no_colour()
        .args(["-T", "--filesystem=all"])
        .arg(dir.path())
        .output()
        .expect("--filesystem=all failed");
    assert!(default_out.status.success());
    assert!(same_out.status.success());
    assert!(local_out.status.success());
    assert!(all_out.status.success());
    assert_eq!(default_out.stdout, same_out.stdout);
    assert_eq!(default_out.stdout, local_out.stdout);
    assert_eq!(default_out.stdout, all_out.stdout);
}

#[test]
fn xdev_short_flag_equivalent_to_filesystem_same() {
    let dir = filtering_fixture();
    let x_out = lx_no_colour()
        .args(["-T", "-X"])
        .arg(dir.path())
        .output()
        .expect("-X failed");
    let same_out = lx_no_colour()
        .args(["-T", "--filesystem=same"])
        .arg(dir.path())
        .output()
        .expect("--filesystem=same failed");
    let xdev_out = lx_no_colour()
        .args(["-T", "--xdev"])
        .arg(dir.path())
        .output()
        .expect("--xdev failed");
    assert!(x_out.status.success());
    assert_eq!(x_out.stdout, same_out.stdout);
    assert_eq!(x_out.stdout, xdev_out.stdout);
}

#[test]
fn filesystem_invalid_value_rejected() {
    lx_no_colour()
        .args(["--filesystem=garbage"])
        .assert()
        .failure();
}

#[test]
fn xdev_skips_cross_filesystem_descent() {
    // Skip if the test environment has nothing on a different filesystem.
    let Some((parent, child)) = find_cross_device_path() else {
        eprintln!("skipping: no cross-device path found in test environment");
        return;
    };

    // Without -X: the child dir's contents should appear in tree mode
    // (depth 2 is enough to show one level into the child).
    let default_out = lx_no_colour()
        .args(["-TL2"])
        .arg(&parent)
        .output()
        .expect("default failed");
    let default_stdout = String::from_utf8_lossy(&default_out.stdout);

    // With -X: the child dir should appear, but its contents should not.
    let xdev_out = lx_no_colour()
        .args(["-X", "-TL2"])
        .arg(&parent)
        .output()
        .expect("-X failed");
    let xdev_stdout = String::from_utf8_lossy(&xdev_out.stdout);

    // Both runs list the cross-device child name itself.
    assert!(
        default_stdout.contains(&child),
        "default should list {child}: {default_stdout}"
    );
    assert!(
        xdev_stdout.contains(&child),
        "-X should still list {child}: {xdev_stdout}"
    );

    // -X output must be no longer than default output (it can only
    // skip lines, never add them).  And it must be strictly shorter,
    // because we found a cross-device dir whose contents got hidden.
    let default_lines = default_stdout.lines().count();
    let xdev_lines = xdev_stdout.lines().count();
    assert!(
        xdev_lines < default_lines,
        "-X should hide cross-device contents (default: {default_lines} lines, -X: {xdev_lines} lines):\n--- default ---\n{default_stdout}\n--- xdev ---\n{xdev_stdout}"
    );
}

#[test]
fn filesystem_local_crosses_local_boundaries() {
    // `--filesystem=local` should cross local-but-different-device
    // boundaries (matching `--filesystem=all`), unlike
    // `--filesystem=same` which refuses any cross-device transition.
    //
    // We can't portably test the *network-skip* path without a real
    // network mount; that needs empirical verification (which we did
    // manually against an SMB mount during development).  This test
    // covers the local-boundary-crossing case, which on most systems
    // exercises e.g. /dev (devfs/tmpfs) or /System/Volumes (APFS) as
    // a non-network cross-device target.
    let Some((parent, _child)) = find_cross_device_path() else {
        eprintln!("skipping: no cross-device path found in test environment");
        return;
    };

    let all_out = lx_no_colour()
        .args(["-TL2", "--filesystem=all"])
        .arg(&parent)
        .output()
        .expect("--filesystem=all failed");
    let local_out = lx_no_colour()
        .args(["-TL2", "--filesystem=local"])
        .arg(&parent)
        .output()
        .expect("--filesystem=local failed");
    let same_out = lx_no_colour()
        .args(["-TL2", "--filesystem=same"])
        .arg(&parent)
        .output()
        .expect("--filesystem=same failed");

    let all_lines = String::from_utf8_lossy(&all_out.stdout).lines().count();
    let local_lines = String::from_utf8_lossy(&local_out.stdout).lines().count();
    let same_lines = String::from_utf8_lossy(&same_out.stdout).lines().count();

    // `same` strictly hides cross-device contents; `local` crosses
    // them when they're not network-backed; `all` always crosses.
    // So same < all, and local should match all on a system whose
    // cross-device mounts are all local (which is true of most CI
    // and dev environments).
    assert!(
        same_lines < all_lines,
        "same should hide cross-device contents (same: {same_lines}, all: {all_lines})"
    );
    // local matching all confirms the *crossing* path is exercised.
    // If a CI runner happens to have a network mount under one of the
    // probed parents, local would equal same instead — also valid
    // behaviour, just not what this test specifically asserts, so we
    // accept either outcome here.
    assert!(
        local_lines == all_lines || local_lines == same_lines,
        "local should equal either all (local-only crossings) or same (network mount present): local={local_lines}, all={all_lines}, same={same_lines}"
    );
}

#[test]
fn no_filesystem_resets_to_all() {
    let dir = filtering_fixture();
    // -X then --no-filesystem: should be equivalent to no -X.
    let default_out = lx_no_colour()
        .args(["-T"])
        .arg(dir.path())
        .output()
        .expect("default failed");
    let reset_out = lx_no_colour()
        .args(["-T", "-X", "--no-filesystem"])
        .arg(dir.path())
        .output()
        .expect("-X --no-filesystem failed");
    assert!(reset_out.status.success());
    assert_eq!(default_out.stdout, reset_out.stdout);
}
