//! Tests for the conf.d/ drop-in config directory.

mod support;

use predicates::prelude::*;
use std::fs;
use support::lx_no_colour;
use tempfile::tempdir;

/// Create a config directory with a main config and optional drop-ins.
fn config_with_dropins(
    main_config: &str,
    dropins: &[(&str, &str)],
) -> (tempfile::TempDir, assert_cmd::Command) {
    let dir = tempdir().expect("failed to create tempdir");
    let config_dir = dir.path().join("lx");
    let conf_d = config_dir.join("conf.d");
    fs::create_dir_all(&conf_d).unwrap();

    // Write main config
    let main_content = if main_config.contains("version") {
        main_config.to_string()
    } else {
        format!("version = \"0.3\"\n{main_config}")
    };
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, main_content).unwrap();

    // Write drop-in files
    for (name, content) in dropins {
        fs::write(conf_d.join(name), content).unwrap();
    }

    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("HOME", "/nonexistent")
        .env_remove("LX_CONFIG")
        .env("XDG_CONFIG_HOME", dir.path())
        .arg("--colour=never");
    (dir, cmd)
}

#[test]
fn dropin_theme_is_loaded() {
    let (_dir, mut cmd) = config_with_dropins(
        r#"
        [personality.lx]
        theme = "test-theme"
        "#,
        &[(
            "theme.toml",
            r#"
            [theme.test-theme]
            inherits = "exa"
            directory = "bold red"
        "#,
        )],
    );

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-theme"));
}

#[test]
fn dropin_files_loaded_alphabetically() {
    // Two drop-ins both define the same class; the later one (b.toml)
    // should override the earlier one (a.toml).
    let (_dir, mut cmd) = config_with_dropins(
        "",
        &[
            (
                "a-first.toml",
                r#"
                [class]
                testclass = ["*.aaa"]
            "#,
            ),
            (
                "b-second.toml",
                r#"
                [class]
                testclass = ["*.bbb"]
            "#,
            ),
        ],
    );

    cmd.args(["--dump-class=testclass"])
        .assert()
        .success()
        .stdout(predicate::str::contains("*.bbb"))
        .stdout(predicate::str::contains("*.aaa").not());
}

#[test]
fn dropin_personality_available() {
    let (_dir, mut cmd) = config_with_dropins(
        "",
        &[(
            "pers.toml",
            r#"
            [personality.custom]
            long = true
            header = true
        "#,
        )],
    );

    cmd.args(["--dump-personality=custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("long"))
        .stdout(predicate::str::contains("header"));
}

#[test]
fn dropin_without_main_config() {
    // Drop-ins work even with no main config file.
    let dir = tempdir().expect("failed to create tempdir");
    let conf_d = dir.path().join("lx").join("conf.d");
    fs::create_dir_all(&conf_d).unwrap();
    fs::write(
        conf_d.join("classes.toml"),
        r#"
        [class]
        myclass = ["*.xyz"]
    "#,
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("HOME", "/nonexistent")
        .env_remove("LX_CONFIG")
        .env("XDG_CONFIG_HOME", dir.path())
        .arg("--colour=never");

    cmd.args(["--dump-class=myclass"])
        .assert()
        .success()
        .stdout(predicate::str::contains("*.xyz"));
}

#[test]
fn show_config_lists_dropins() {
    let (_dir, mut cmd) = config_with_dropins(
        "",
        &[(
            "my-theme.toml",
            r#"
            [theme.ocean]
            inherits = "exa"
        "#,
        )],
    );

    cmd.args(["--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Drop-ins:"))
        .stdout(predicate::str::contains("my-theme.toml"));
}

#[test]
fn no_dropin_dir_is_fine() {
    // No conf.d/ directory — should work without errors.
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, "version = \"0.3\"\n").unwrap();

    lx_no_colour()
        .env("LX_CONFIG", config_path)
        .args(["-1", "."])
        .assert()
        .success();
}
