//! Tests for sorting and filtering with controlled fixtures.

mod support;

use predicates::prelude::*;
use std::fs;
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

// ── Only files ────────────────────────────────────────────────────

#[test]
fn only_files_hides_dirs() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    lx_no_colour()
        .args(["-1f"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("file.txt"))
        .stdout(predicate::str::contains("subdir").not());
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

#[test]
fn no_dirs_first_suppresses_dirs_first() {
    // Hidden --no-dirs-first overrides an earlier -F (typical use:
    // suppressing a personality's group-dirs=first default).
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("zzz_dir")).unwrap();

    lx_no_colour()
        .args(["-1", "-F", "--no-dirs-first"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            file_pos < dir_pos // sorted alphabetically, dirs no longer pulled forward
        }));
}

#[test]
fn no_dirs_last_suppresses_dirs_last() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(dir.path().join("zzz_dir")).unwrap();

    lx_no_colour()
        .args(["-1", "-J", "--no-dirs-last"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            file_pos < dir_pos
        }));
}

// ── Batch D: expanded sort fields ──────────────────────────

#[test]
fn sort_by_blocks() {
    // Larger file → more blocks (on any reasonable filesystem).
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("small.txt"), "x").unwrap();
    fs::write(dir.path().join("large.txt"), vec![0u8; 16384]).unwrap();

    lx_no_colour()
        .args(["-1", "--sort=blocks"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            output.find("small.txt").unwrap() < output.find("large.txt").unwrap()
        }));
}

#[test]
fn sort_by_perms() {
    let dir = tempdir().expect("failed to create tempdir");
    let restricted = dir.path().join("restricted.txt");
    let open = dir.path().join("open.txt");
    fs::write(&restricted, "").unwrap();
    fs::write(&open, "").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&restricted, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o644)).unwrap();
    }

    lx_no_colour()
        .args(["-1", "--sort=permissions"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            // 0o600 (384) sorts before 0o644 (420).
            output.find("restricted.txt").unwrap() < output.find("open.txt").unwrap()
        }));
}

#[test]
fn sort_version_is_alias_for_name() {
    // `-s version` should behave like `-s name` — natord already
    // handles embedded-number ordering.
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file2.txt"), "").unwrap();
    fs::write(dir.path().join("file10.txt"), "").unwrap();
    fs::write(dir.path().join("file1.txt"), "").unwrap();

    lx_no_colour()
        .args(["-1", "--sort=version"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let p1 = output.find("file1.txt").unwrap();
            let p2 = output.find("file2.txt").unwrap();
            let p10 = output.find("file10.txt").unwrap();
            p1 < p2 && p2 < p10
        }));
}

#[test]
fn sort_octal_is_alias_for_perms() {
    // `-s octal` should resolve to the same sort as `-s perms`.
    // We don't verify ordering here (that's the perms test); we
    // just verify that the flag is accepted without error.
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file.txt"), "").unwrap();

    lx_no_colour()
        .args(["-1", "--sort=octal"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn sort_by_links() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("single.txt"), "").unwrap();
    let source = dir.path().join("source.txt");
    fs::write(&source, "").unwrap();
    // A hard link increases the source file's link count from 1 to 2.
    #[cfg(unix)]
    fs::hard_link(&source, dir.path().join("linked.txt")).unwrap();

    lx_no_colour()
        .args(["-1", "--sort=links"])
        .arg(dir.path())
        .assert()
        .success();
    // We don't assert ordering because source.txt and linked.txt
    // share the same inode and link count, so their relative order
    // depends on the secondary (name) sort. The main thing is that
    // --sort=links parses and runs.
}

#[test]
fn sort_by_user_runs() {
    // Same user owns everything in a tempdir — sort by user is a
    // no-op on the ordering, but should succeed.
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();

    lx_no_colour()
        .args(["-1", "--sort=user"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn sort_by_uid_runs() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("a.txt"), "").unwrap();

    lx_no_colour()
        .args(["-1", "--sort=uid"])
        .arg(dir.path())
        .assert()
        .success();
}
