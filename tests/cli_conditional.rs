//! Tests for conditional config: [[personality.NAME.when]] blocks.

use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Create a config with conditional overrides and return a command.
fn lx_with_conditional_config(config_content: &str) -> (tempfile::TempDir, assert_cmd::Command) {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    let content = if config_content.contains("version") {
        config_content.to_string()
    } else {
        format!("version = \"0.4\"\n{config_content}")
    };
    fs::write(&config_path, content).unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", config_path)
        .env("HOME", "/nonexistent")
        .arg("--colour=never");
    (dir, cmd)
}

// ── Basic conditional override ───────────────────────────────────

#[test]
fn when_env_matches_overrides_setting() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_COND = "yes"
        sort = "size"
    "#,
    );

    cmd.env("LX_TEST_COND", "yes")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_env_not_set_uses_base() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_COND = "yes"
        sort = "size"
    "#,
    );

    // LX_TEST_COND not set — should use base sort = "name"
    cmd.env_remove("LX_TEST_COND")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_env_wrong_value_uses_base() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_COND = "yes"
        sort = "size"
    "#,
    );

    cmd.env("LX_TEST_COND", "no")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

// ── Multiple conditions (AND) ────────────────────────────────────

#[test]
fn when_multiple_env_all_must_match() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_A = "one"
        env.LX_TEST_B = "two"
        sort = "size"
    "#,
    );

    // Only one matches — should NOT override.
    cmd.env("LX_TEST_A", "one")
        .env("LX_TEST_B", "wrong")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_multiple_env_both_match() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_A = "one"
        env.LX_TEST_B = "two"
        sort = "size"
    "#,
    );

    cmd.env("LX_TEST_A", "one")
        .env("LX_TEST_B", "two")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

// ── Multiple when blocks (OR, later wins) ────────────────────────

#[test]
fn when_later_block_overrides_earlier() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_FIRST = "yes"
        sort = "size"

        [[personality.lx.when]]
        env.LX_TEST_SECOND = "yes"
        sort = "extension"
    "#,
    );

    // Both match — second block's sort = "extension" should win.
    cmd.env("LX_TEST_FIRST", "yes")
        .env("LX_TEST_SECOND", "yes")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("extension")));
}

// ── env.VAR = true/false (set/unset) ─────────────────────────────

#[test]
fn when_env_true_matches_when_present() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_SET = true
        sort = "size"
    "#,
    );

    cmd.env("LX_TEST_SET", "anything")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_env_true_fails_when_absent() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_SET = true
        sort = "size"
    "#,
    );

    cmd.env_remove("LX_TEST_SET")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_env_false_matches_when_absent() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_UNSET = false
        sort = "size"
    "#,
    );

    cmd.env_remove("LX_TEST_UNSET")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_env_false_fails_when_present() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_UNSET = false
        sort = "size"
    "#,
    );

    cmd.env("LX_TEST_UNSET", "something")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_env_true_and_exact_combined() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_SSH = true
        env.LX_TEST_TERM = "ghostty"
        sort = "size"
    "#,
    );

    // Both conditions met.
    cmd.env("LX_TEST_SSH", "1")
        .env("LX_TEST_TERM", "ghostty")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_env_true_and_exact_partial_fail() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_SSH = true
        env.LX_TEST_TERM = "ghostty"
        sort = "size"
    "#,
    );

    // env-set met but env exact match fails.
    cmd.env("LX_TEST_SSH", "1")
        .env("LX_TEST_TERM", "wezterm")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

// ── Version warning ──────────────────────────────────────────────

#[test]
fn when_blocks_in_v03_config_warns() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        version = "0.3"

        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        env.LX_TEST_COND = "yes"
        sort = "size"
    "#,
    );

    cmd.args(["-1", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("version is \"0.3\""));
}

#[test]
fn v03_config_without_when_no_warning() {
    let (_dir, mut cmd) = lx_with_conditional_config(
        r#"
        version = "0.3"

        [personality.lx]
        sort = "name"
    "#,
    );

    cmd.args(["-1", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("0.3").not());
}

// ── Platform predicate ───────────────────────────────────────────

#[test]
fn when_platform_matches_current_overrides() {
    // The test runs on whatever the host OS happens to be; pick
    // that as the matching platform so the override always fires.
    let here = std::env::consts::OS;
    let toml = format!(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        platform = "{here}"
        sort = "size"
        "#
    );
    let (_dir, mut cmd) = lx_with_conditional_config(&toml);

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_platform_other_does_not_override() {
    // A platform string that we are definitely not running on.
    let other = if std::env::consts::OS == "macos" {
        "linux"
    } else {
        "macos"
    };
    let toml = format!(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        platform = "{other}"
        sort = "size"
        "#
    );
    let (_dir, mut cmd) = lx_with_conditional_config(&toml);

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_platform_array_any_match_overrides() {
    let here = std::env::consts::OS;
    let toml = format!(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        platform = ["{here}", "haiku"]
        sort = "size"
        "#
    );
    let (_dir, mut cmd) = lx_with_conditional_config(&toml);

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}

#[test]
fn when_platform_array_no_match_uses_base() {
    let toml = r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        platform = ["plan9", "haiku"]
        sort = "size"
    "#;
    let (_dir, mut cmd) = lx_with_conditional_config(toml);

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_platform_and_env_both_required() {
    // Both conditions in a block must match (AND).  Set the env
    // variable to the right value, but use the wrong platform —
    // override should NOT apply.
    let other = if std::env::consts::OS == "macos" {
        "linux"
    } else {
        "macos"
    };
    let toml = format!(
        r#"
        [personality.lx]
        sort = "name"

        [[personality.lx.when]]
        platform = "{other}"
        env.LX_TEST_COND = "yes"
        sort = "size"
        "#
    );
    let (_dir, mut cmd) = lx_with_conditional_config(&toml);

    cmd.env("LX_TEST_COND", "yes")
        .args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("name")));
}

#[test]
fn when_platform_inherits_through_chain() {
    // A platform-gated override on a parent personality should
    // apply when the child inherits it.
    let here = std::env::consts::OS;
    let toml = format!(
        r#"
        [personality.parent]
        sort = "name"

        [[personality.parent.when]]
        platform = "{here}"
        sort = "size"

        [personality.child]
        inherits = "parent"
        "#
    );
    let (_dir, mut cmd) = lx_with_conditional_config(&toml);

    cmd.args(["-p", "child", "--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sort").and(predicate::str::contains("size")));
}
