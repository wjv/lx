//! Tests for config file, personalities, and argv[0] dispatch.

mod support;

use std::fs;
use std::os::unix::fs as unix_fs;
use std::process::Command as StdCommand;
use predicates::prelude::*;
use support::{lx, lx_no_colour};
use tempfile::tempdir;


/// Helper: run lx with a given config file via LX_CONFIG env var.
fn lx_with_config(config_content: &str) -> (tempfile::TempDir, assert_cmd::Command) {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = lx_no_colour();
    cmd.env("LX_CONFIG", config_path);
    (dir, cmd)
}


// ── Config defaults ──────────────────────────────────────────────

#[test]
fn config_default_group_dirs() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [defaults]
        group-dirs = "first"
    "#);

    // Create a tempdir with a file and a directory
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(work.path().join("zzz_dir")).unwrap();

    cmd.args(["-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            dir_pos < file_pos
        }));
}

#[test]
fn config_default_overridden_by_cli() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [defaults]
        group-dirs = "first"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(work.path().join("zzz_dir")).unwrap();

    // CLI --group-dirs=last should override config first
    cmd.args(["-1", "--group-dirs=last"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let file_pos = output.find("aaa_file.txt").unwrap();
            let dir_pos = output.find("zzz_dir").unwrap();
            file_pos < dir_pos
        }));
}

#[test]
fn config_default_time_style() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [defaults]
        time-style = "long-iso"
    "#);

    cmd.args(["-l", "Cargo.toml"])
        .assert()
        .success()
        // long-iso format includes full date like "2026-03-19 14:27"
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}


// ── Config-defined formats ───────────────────────────────────────

#[test]
fn config_custom_format() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [format.tiny]
        columns = ["size", "modified"]
    "#);

    cmd.args(["--format=tiny", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"))
        // No permissions column (not in format)
        .stdout(predicate::str::contains(".rw").not());
}

#[test]
fn config_format_overrides_compiled_in() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [format.long]
        columns = ["size", "modified"]
    "#);

    // -l uses "long" format, which is now overridden in config
    cmd.args(["-l", "Cargo.toml"])
        .assert()
        .success()
        // No permissions (config format doesn't include them)
        .stdout(predicate::str::contains(".rw").not());
}


// ── Config-defined personalities ─────────────────────────────────

#[test]
fn config_custom_personality() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.myview]
        columns = ["perms", "size"]
        flags = ["--group-dirs=first"]
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("file.txt"), "").unwrap();
    fs::create_dir(work.path().join("subdir")).unwrap();

    cmd.args(["-pmyview"])
        .arg(work.path())
        .assert()
        .success()
        // Directories first (from personality flags)
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("subdir").unwrap();
            let file_pos = output.find("file.txt").unwrap();
            dir_pos < file_pos
        }));
}


// ── Compiled-in personalities ────────────────────────────────────

#[test]
fn personality_ll() {
    lx_no_colour()
        .args(["-pll", "Cargo.toml"])
        .assert()
        .success()
        // ll includes group column
        .stdout(predicate::str::contains(support::current_group()));
}

#[test]
fn personality_lll_has_header() {
    lx_no_colour()
        .args(["-plll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn personality_lll_has_long_iso() {
    lx_no_colour()
        .args(["-plll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}

#[test]
fn personality_tree() {
    lx_no_colour()
        .args(["-ptree", "-L1", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("├──").or(predicate::str::contains("└──")));
}

#[test]
fn personality_cli_override() {
    // -pll gives group; --no-group should remove it
    lx_no_colour()
        .args(["-pll", "--no-group", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").not());
}

#[test]
fn personality_long_flag() {
    // --personality=ll should work same as -pll
    lx_no_colour()
        .args(["--personality=ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(support::current_group()));
}


// ── argv[0] dispatch ─────────────────────────────────────────────

#[test]
fn argv0_ll_dispatch() {
    // Create a symlink named "ll" pointing to the lx binary
    let dir = tempdir().expect("failed to create tempdir");
    let lx_path = assert_cmd::cargo::cargo_bin("lx");
    let link_path = dir.path().join("ll");
    unix_fs::symlink(&lx_path, &link_path).unwrap();

    let output = StdCommand::new(&link_path)
        .args(["--colour=never", "Cargo.toml"])
        .output()
        .expect("failed to run symlink");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ll personality includes group
    let group = support::current_group();
    assert!(
        stdout.contains(&group),
        "argv[0]=ll should show group column ({group}), got: {stdout}"
    );
}

#[test]
fn argv0_unknown_falls_back() {
    // An unknown symlink name should just behave like lx
    let dir = tempdir().expect("failed to create tempdir");
    let lx_path = assert_cmd::cargo::cargo_bin("lx");
    let link_path = dir.path().join("unknown_name");
    unix_fs::symlink(&lx_path, &link_path).unwrap();

    let output = StdCommand::new(&link_path)
        .args(["--colour=never", "Cargo.toml"])
        .output()
        .expect("failed to run symlink");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cargo.toml"));
}


// ── --init-config ────────────────────────────────────────────────

#[test]
fn init_config_creates_file() {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join(".lxconfig.toml");

    lx()
        .args(["--init-config"])
        .env("HOME", dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Wrote default config"));

    assert!(config_path.exists());

    // The generated file should be valid TOML (all commented out = empty)
    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("# lx configuration file"));
    assert!(contents.contains("[defaults]"));
    assert!(contents.contains("[format.long]"));
    assert!(contents.contains("[personality.ll]"));
}

#[test]
fn init_config_refuses_overwrite() {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join(".lxconfig.toml");
    fs::write(&config_path, "existing").unwrap();

    lx()
        .args(["--init-config"])
        .env("HOME", dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}


// ── Config file discovery ────────────────────────────────────────

#[test]
fn lx_config_env_takes_priority() {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("custom.toml");
    fs::write(&config_path, r#"
        [defaults]
        group-dirs = "first"
    "#).unwrap();

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("aaa.txt"), "").unwrap();
    fs::create_dir(work.path().join("zzz")).unwrap();

    lx_no_colour()
        .args(["-1"])
        .arg(work.path())
        .env("LX_CONFIG", &config_path)
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz").unwrap();
            let file_pos = output.find("aaa.txt").unwrap();
            dir_pos < file_pos
        }));
}

#[test]
fn no_config_file_is_fine() {
    // With LX_CONFIG pointing to a nonexistent file and HOME in an
    // empty tempdir, lx should work fine with no config.
    let dir = tempdir().expect("failed to create tempdir");

    lx_no_colour()
        .args(["-1", "Cargo.toml"])
        .env("LX_CONFIG", dir.path().join("nonexistent.toml"))
        .env("HOME", dir.path())
        .env_remove("XDG_CONFIG_HOME")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}
