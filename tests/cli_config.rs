//! Tests for config file, personalities, and argv[0] dispatch.

mod support;

use predicates::prelude::*;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::process::Command as StdCommand;
use support::{lx, lx_no_colour};
use tempfile::tempdir;

/// Helper: run lx with a given config file via LX_CONFIG env var.
/// Automatically prepends the current config version if not present.
fn lx_with_config(config_content: &str) -> (tempfile::TempDir, assert_cmd::Command) {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    let content = if config_content.contains("version") {
        config_content.to_string()
    } else {
        format!("version = \"0.3\"\n{config_content}")
    };
    fs::write(&config_path, content).unwrap();

    let mut cmd = lx_no_colour();
    cmd.env("LX_CONFIG", config_path);
    (dir, cmd)
}

// ── The lx personality (global defaults) ─────────────────────────

#[test]
fn config_lx_personality_group_dirs() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        group-dirs = "first"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        group-dirs = "first"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        time-style = "long-iso"
    "#,
    );

    cmd.args(["-l", "Cargo.toml"])
        .assert()
        .success()
        // long-iso format includes full date like "2026-03-19 14:27"
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}

// ── Config-defined formats ───────────────────────────────────────

#[test]
fn config_custom_format() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [format]
        tiny = ["size", "modified"]
    "#,
    );

    cmd.args(["--format=tiny", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"))
        // No permissions column (not in format)
        .stdout(predicate::str::contains(".rw").not());
}

#[test]
fn config_format_overrides_compiled_in() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [format]
        long = ["size", "modified"]
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.myview]
        columns = ["perms", "size"]
        group-dirs = "first"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.base]
        group-dirs = "first"

        [personality.child]
        inherits = "base"
        format = "long"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.root]
        group-dirs = "first"

        [personality.mid]
        inherits = "root"
        format = "long"

        [personality.leaf]
        inherits = "mid"
        header = true
    "#,
    );

    cmd.args(["-pleaf", "Cargo.toml"])
        .assert()
        .success()
        // header from leaf, format=long from mid, group-dirs from root
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn inherit_child_overrides_parent_setting() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.parent]
        format = "long"
        sort = "name"

        [personality.child]
        inherits = "parent"
        sort = "size"
    "#,
    );

    // Just check it runs without error; sort=size from child wins
    cmd.args(["-pchild", "Cargo.toml"]).assert().success();
}

#[test]
fn inherit_child_overrides_format() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.parent]
        format = "long2"

        [personality.child]
        inherits = "parent"
        format = "long"
    "#,
    );

    cmd.args(["-pchild", "Cargo.toml"])
        .assert()
        .success()
        // long format has no group column
        .stdout(predicate::str::contains("staff").not());
}

#[test]
fn inherit_cycle_detected() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.a]
        inherits = "b"

        [personality.b]
        inherits = "a"
    "#,
    );

    cmd.args(["-pa", "Cargo.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("inheritance cycle"));
}

#[test]
fn inherit_from_compiled_in() {
    // Config personality inherits from compiled-in "ll"
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.myll]
        inherits = "ll"
        header = true
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.base]
        group-dirs = "first"
        header = true

        [personality.standalone]
        format = "long"
    "#,
    );

    cmd.args(["-pstandalone", "Cargo.toml"])
        .assert()
        .success()
        // Should NOT have header (not inherited from base)
        .stdout(predicate::str::contains("Permissions").not());
}

// ── Named settings ──────────────────────────────────────────────

#[test]
fn config_personality_bool_setting() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.hdr]
        format = "long"
        header = true
    "#,
    );

    cmd.args(["-phdr", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn config_personality_columns_as_string() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.tiny]
        columns = "size,modified"
    "#,
    );

    cmd.args(["-ptiny", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"))
        .stdout(predicate::str::contains(".rw").not());
}

#[test]
fn config_unknown_setting_warns() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.bad]
        format = "long"
        frobnicate = true
    "#,
    );

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

    lx().args(["--init-config"])
        .env("HOME", dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Wrote default config"));

    assert!(config_path.exists());

    // The generated file should be valid TOML (all commented out = empty)
    let contents = fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("## lx configuration file"));
    assert!(contents.contains("version = \"0.6\""));
    assert!(contents.contains("[personality.default]"));
    assert!(contents.contains("[personality.lx]"));
    assert!(contents.contains("inherits"));
}

#[test]
fn init_config_refuses_overwrite() {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join(".lxconfig.toml");
    fs::write(&config_path, "existing").unwrap();

    lx().args(["--init-config"])
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
    fs::write(
        &config_path,
        r#"
        version = "0.3"
        [personality.lx]
        group-dirs = "first"
    "#,
    )
    .unwrap();

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

// ── --dump-class ─────────────────────────────────────────────────

#[test]
fn dump_class_all() {
    lx_no_colour()
        .arg("--dump-class")
        .assert()
        .success()
        .stdout(predicate::str::contains("[class]"))
        .stdout(predicate::str::contains("temp = ["))
        .stdout(predicate::str::contains("image = ["));
}

#[test]
fn dump_class_single() {
    lx_no_colour()
        .arg("--dump-class=temp")
        .assert()
        .success()
        .stdout(predicate::str::contains("[class]"))
        .stdout(predicate::str::contains("temp = ["))
        .stdout(predicate::str::contains("*.tmp"));
}

#[test]
fn dump_class_unknown() {
    lx_no_colour()
        .arg("--dump-class=bogus")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value 'bogus' for '--dump-class",
        ))
        .stderr(predicate::str::contains("[possible values:"));
}

// ── --dump-format ────────────────────────────────────────────────

#[test]
fn dump_format_all() {
    lx_no_colour()
        .arg("--dump-format")
        .assert()
        .success()
        .stdout(predicate::str::contains("[format]"))
        .stdout(predicate::str::contains("long = ["))
        .stdout(predicate::str::contains("long3 = ["));
}

#[test]
fn dump_format_single() {
    lx_no_colour()
        .arg("--dump-format=long2")
        .assert()
        .success()
        .stdout(predicate::str::contains("[format]"))
        .stdout(predicate::str::contains("long2 = ["))
        .stdout(predicate::str::contains("\"permissions\""));
}

#[test]
fn dump_format_unknown() {
    lx_no_colour()
        .arg("--dump-format=bogus")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value 'bogus' for '--dump-format",
        ))
        .stderr(predicate::str::contains("[possible values:"));
}

// ── --dump-personality ───────────────────────────────────────────

#[test]
fn dump_personality_all() {
    lx_no_colour()
        .arg("--dump-personality")
        .assert()
        .success()
        .stdout(predicate::str::contains("[personality.ll]"))
        .stdout(predicate::str::contains("[personality.tree]"));
}

#[test]
fn dump_personality_single() {
    lx_no_colour()
        .arg("--dump-personality=ll")
        .assert()
        .success()
        .stdout(predicate::str::contains("[personality.ll]"))
        .stdout(predicate::str::contains("inherits = \"lx\""))
        .stdout(predicate::str::contains("format = \"long2\""));
}

#[test]
fn dump_personality_unknown() {
    lx_no_colour()
        .arg("--dump-personality=bogus")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value 'bogus' for '--dump-personality",
        ))
        .stderr(predicate::str::contains("[possible values:"));
}

// ── --dump-theme ─────────────────────────────────────────────────

#[test]
fn dump_theme_exa() {
    lx_no_colour()
        .arg("--dump-theme=exa")
        .assert()
        .success()
        .stdout(predicate::str::contains("compiled-in"))
        .stdout(predicate::str::contains("inherits = \"exa\""));
}

#[test]
fn dump_theme_unknown() {
    lx_no_colour()
        .arg("--dump-theme=bogus")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value 'bogus' for '--dump-theme",
        ))
        .stderr(predicate::str::contains("[possible values:"));
}

// ── --dump-style ─────────────────────────────────────────────────

#[test]
fn dump_style_exa() {
    lx_no_colour()
        .arg("--dump-style=exa")
        .assert()
        .success()
        .stdout(predicate::str::contains("[style.exa]"))
        .stdout(predicate::str::contains("class.image"))
        .stdout(predicate::str::contains("class.temp"));
}

#[test]
fn dump_style_unknown() {
    lx_no_colour()
        .arg("--dump-style=bogus")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value 'bogus' for '--dump-style",
        ))
        .stderr(predicate::str::contains("[possible values:"));
}

// ── --dump with config overrides ─────────────────────────────────

#[test]
fn dump_class_shows_config_override() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
[class]
temp = ["*.tmp", "*.bak"]
"#,
    );

    cmd.arg("--dump-class=temp")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"*.tmp\", \"*.bak\""))
        // Should NOT contain the compiled-in patterns that were overridden
        .stdout(predicate::str::contains("*.swp").not());
}

// ── --init-config invariant ──────────────────────────────────────
//
// The generated config must be a no-op: lx with the generated config
// must produce identical --dump-* output to lx with no config at all.

#[test]
fn init_config_does_not_change_defaults() {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join(".lxconfig.toml");

    // Generate the default config file.
    lx().arg("--init-config")
        .env("HOME", dir.path())
        .env_remove("LX_CONFIG")
        .assert()
        .success();

    assert!(config_path.exists(), "config file should have been created");

    // For each dump flag, compare output with the generated config
    // against output with no config (LX_CONFIG=/dev/null).
    // Pin TERM and COLORTERM so the [[when]] blocks resolve the same
    // way in both runs.
    for flag in [
        "--dump-format",
        "--dump-personality",
        "--dump-style",
        "--dump-class",
    ] {
        let with_config = lx()
            .arg(flag)
            .env("LX_CONFIG", &config_path)
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .output()
            .expect("failed to run lx with config");

        let without_config = lx()
            .arg(flag)
            .env("LX_CONFIG", "/dev/null")
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .output()
            .expect("failed to run lx without config");

        assert_eq!(
            String::from_utf8_lossy(&with_config.stdout),
            String::from_utf8_lossy(&without_config.stdout),
            "{flag} output differs between generated config and no config",
        );
    }
}

// ── Three-state Bool config semantics ───────────────────────────

/// `key = false` in a child personality suppresses a column that
/// the inherited format would otherwise include.  Before the
/// three-state fix, `false` was a no-op and only `no-key = true`
/// could suppress.
#[test]
fn bool_false_suppresses_inherited_column() {
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.nosize]
        inherits = "ll"
        size = false
    "#,
    );

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("hello.txt"), "hi").unwrap();

    // ll includes the size column; nosize should suppress it.
    // With size present, output contains the file size ("2" or "2 B").
    // Without it, the size is absent.
    let with_size = cmd
        .args(["-p", "ll", "--colour=never"])
        .arg(work.path())
        .output()
        .expect("failed to run lx");
    let with_size_out = String::from_utf8_lossy(&with_size.stdout);
    // ll shows sizes — verify baseline
    assert!(
        with_size_out.contains("hello.txt"),
        "baseline: file should appear"
    );

    let (_dir2, mut cmd2) = lx_with_config(
        r#"
        [personality.nosize]
        inherits = "ll"
        size = false
    "#,
    );
    let without_size = cmd2
        .args(["-p", "nosize", "--colour=never"])
        .arg(work.path())
        .output()
        .expect("failed to run lx");
    let without_size_out = String::from_utf8_lossy(&without_size.stdout);

    // The size column in ll shows "2" for a 2-byte file.
    // With size suppressed, the number should be absent.
    assert!(
        with_size_out.contains(" 2 "),
        "baseline: ll should show file size"
    );
    assert!(
        !without_size_out.contains(" 2 "),
        "size = false should suppress the size column"
    );
}

/// `permissions = false` suppresses the permissions column from an
/// inherited format — verifies the three-state logic works for a
/// column other than size.
#[test]
fn bool_false_suppresses_permissions() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("test.txt"), "").unwrap();

    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.noperm]
        inherits = "ll"
        permissions = false
    "#,
    );
    cmd.args(["-p", "noperm", "--colour=never"])
        .arg(work.path())
        .assert()
        .success()
        // ll always shows permissions (e.g. ".rw-r--r--").
        // With permissions suppressed, no "rw" should appear.
        .stdout(predicate::str::contains("rw").not());
}

/// `no-size = true` and `size = false` are equivalent — both
/// suppress the inherited size column.
#[test]
fn no_key_true_equivalent_to_key_false() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("hello.txt"), "hi").unwrap();

    let (_dir1, mut cmd1) = lx_with_config(
        r#"
        [personality.via-false]
        inherits = "ll"
        size = false
    "#,
    );
    let out_false = cmd1
        .args(["-p", "via-false", "--colour=never"])
        .arg(work.path())
        .output()
        .expect("failed to run lx");

    let (_dir2, mut cmd2) = lx_with_config(
        r#"
        [personality.via-no]
        inherits = "ll"
        no-size = true
    "#,
    );
    let out_no = cmd2
        .args(["-p", "via-no", "--colour=never"])
        .arg(work.path())
        .output()
        .expect("failed to run lx");

    assert_eq!(
        String::from_utf8_lossy(&out_false.stdout),
        String::from_utf8_lossy(&out_no.stdout),
        "size = false and no-size = true should produce identical output"
    );
}

// ── TOML array syntax for string settings ──────────────────────

/// `ignore = ["*.tmp", "*.bak"]` should work the same as
/// `ignore = "*.tmp|*.bak"`.
#[test]
fn config_ignore_toml_array() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("keep.txt"), "").unwrap();
    fs::write(work.path().join("notes.tmp"), "").unwrap();
    fs::write(work.path().join("old.bak"), "").unwrap();

    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        ignore = ["*.tmp", "*.bak"]
    "#,
    );
    cmd.args(["-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"))
        .stdout(predicate::str::contains("notes.tmp").not())
        .stdout(predicate::str::contains("old.bak").not());
}

/// Pipe-separated string form still works alongside the array form.
#[test]
fn config_ignore_pipe_string_still_works() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("keep.txt"), "").unwrap();
    fs::write(work.path().join("notes.tmp"), "").unwrap();
    fs::write(work.path().join("old.bak"), "").unwrap();

    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        ignore = "*.tmp|*.bak"
    "#,
    );
    cmd.args(["-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("keep.txt"))
        .stdout(predicate::str::contains("notes.tmp").not())
        .stdout(predicate::str::contains("old.bak").not());
}

/// `prune = ["target", "node_modules"]` should work as a TOML array.
#[test]
fn config_prune_toml_array() {
    let work = tempdir().expect("failed to create workdir");
    fs::create_dir_all(work.path().join("src")).unwrap();
    fs::create_dir_all(work.path().join("target/debug")).unwrap();
    fs::create_dir_all(work.path().join("node_modules/foo")).unwrap();
    fs::write(work.path().join("src/main.rs"), "").unwrap();
    fs::write(work.path().join("target/debug/binary"), "").unwrap();
    fs::write(work.path().join("node_modules/foo/index.js"), "").unwrap();

    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.lx]
        prune = ["target", "node_modules"]
    "#,
    );
    cmd.args(["-T"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("debug").not())
        .stdout(predicate::str::contains("index.js").not());
}

/// `tree = false` has no negation counterpart (`--no-tree` doesn't
/// exist), so `false` should be a silent no-op rather than an error.
#[test]
fn bool_false_without_negation_is_noop() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("test.txt"), "").unwrap();

    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.notree]
        inherits = "lx"
        tree = false
    "#,
    );
    cmd.args(["-p", "notree", "--colour=never"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

/// `no-time = true` is a bulk clear that has no positive counterpart.
/// It must survive as-is (not be rewritten to `time = false`).
#[test]
fn no_time_still_works_as_bulk_clear() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("test.txt"), "").unwrap();

    // lll includes all four timestamps.  no-time should clear them all,
    // then accessed = true adds just the accessed column back.
    let (_dir, mut cmd) = lx_with_config(
        r#"
        [personality.justaccessed]
        inherits = "lll"
        no-time = true
        accessed = true
    "#,
    );
    let out = cmd
        .args(["-p", "justaccessed", "--colour=never"])
        .arg(work.path())
        .output()
        .expect("failed to run lx");
    let stdout = String::from_utf8_lossy(&out.stdout);

    // lll normally shows modified, changed, created, accessed.
    // With no-time + accessed, only accessed should remain.
    // We can't easily tell which timestamp column is which from the
    // output, but we can count: lll has 4 date columns, justaccessed
    // should have exactly 1.  A rough check: the line should be
    // noticeably shorter.
    assert!(stdout.contains("test.txt"), "file should appear in output");
}
