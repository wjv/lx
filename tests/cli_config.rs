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


// ── The lx personality (global defaults) ─────────────────────────

#[test]
fn config_lx_personality_group_dirs() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.lx]
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
fn config_lx_overridden_by_cli() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.lx]
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
fn config_lx_personality_time_style() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.lx]
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
        group-dirs = "first"
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


// ── Personality inheritance ───────────────────────────────────────

#[test]
fn inherit_single_level() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.base]
        group-dirs = "first"

        [personality.child]
        inherits = "base"
        format = "long"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("aaa_file.txt"), "").unwrap();
    fs::create_dir(work.path().join("zzz_dir")).unwrap();

    cmd.args(["-pchild"])
        .arg(work.path())
        .assert()
        .success()
        // group-dirs=first inherited from base
        .stdout(predicate::function(|output: &str| {
            let dir_pos = output.find("zzz_dir").unwrap();
            let file_pos = output.find("aaa_file.txt").unwrap();
            dir_pos < file_pos
        }));
}

#[test]
fn inherit_multi_level() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.root]
        group-dirs = "first"

        [personality.mid]
        inherits = "root"
        format = "long"

        [personality.leaf]
        inherits = "mid"
        header = true
    "#);

    cmd.args(["-pleaf", "Cargo.toml"])
        .assert()
        .success()
        // header from leaf, format=long from mid, group-dirs from root
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn inherit_child_overrides_parent_setting() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.parent]
        format = "long"
        sort = "name"

        [personality.child]
        inherits = "parent"
        sort = "size"
    "#);

    // Just check it runs without error; sort=size from child wins
    cmd.args(["-pchild", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn inherit_child_overrides_format() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.parent]
        format = "long2"

        [personality.child]
        inherits = "parent"
        format = "long"
    "#);

    cmd.args(["-pchild", "Cargo.toml"])
        .assert()
        .success()
        // long format has no group column
        .stdout(predicate::str::contains("staff").not());
}

#[test]
fn inherit_cycle_detected() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.a]
        inherits = "b"

        [personality.b]
        inherits = "a"
    "#);

    cmd.args(["-pa", "Cargo.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("inheritance cycle"));
}

#[test]
fn inherit_from_compiled_in() {
    // Config personality inherits from compiled-in "ll"
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.myll]
        inherits = "ll"
        header = true
    "#);

    cmd.args(["-pmyll", "Cargo.toml"])
        .assert()
        .success()
        // ll gives long2 format (includes group) + header from child
        .stdout(predicate::str::contains("Permissions"))
        .stdout(predicate::str::contains(support::current_group()));
}

#[test]
fn standalone_no_inherits() {
    // No inherits = standalone, no inherited settings
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.base]
        group-dirs = "first"
        header = true

        [personality.standalone]
        format = "long"
    "#);

    cmd.args(["-pstandalone", "Cargo.toml"])
        .assert()
        .success()
        // Should NOT have header (not inherited from base)
        .stdout(predicate::str::contains("Permissions").not());
}


// ── Named settings ──────────────────────────────────────────────

#[test]
fn config_personality_bool_setting() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.hdr]
        format = "long"
        header = true
    "#);

    cmd.args(["-phdr", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn config_personality_columns_as_string() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.tiny]
        columns = "size,modified"
    "#);

    cmd.args(["-ptiny", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"))
        .stdout(predicate::str::contains(".rw").not());
}

#[test]
fn config_unknown_setting_warns() {
    let (_dir, mut cmd) = lx_with_config(r#"
        [personality.bad]
        format = "long"
        frobnicate = true
    "#);

    cmd.args(["-pbad", "Cargo.toml"])
        .assert()
        .success()
        .stderr(predicate::str::contains("unknown setting 'frobnicate'"));
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
    assert!(contents.contains("version = \"0.2\""));
    assert!(contents.contains("[format.long]"));
    assert!(contents.contains("[personality.ll]"));
    assert!(contents.contains("inherits"));
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
        [personality.lx]
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
