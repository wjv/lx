//! Tests for the [theme] config section and --theme flag.

mod support;

use std::fs;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


/// Helper: run lx with colour enabled and a given config.
fn lx_with_theme(config_content: &str) -> (tempfile::TempDir, assert_cmd::Command) {
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", config_path)
       .env("HOME", "/nonexistent")
       .env_remove("LS_COLORS")
       .env_remove("LX_COLORS")
       .arg("--colour=always");
    (dir, cmd)
}


// ── UI element overrides ─────────────────────────────────────────

#[test]
fn theme_directory_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        directory = "bold red"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::create_dir(work.path().join("mydir")).unwrap();

    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"));
}

#[test]
fn theme_date_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        date = "bold cyan"
    "#);

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_x11_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        date = "tomato"
    "#);

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;255;99;71m"));
}

#[test]
fn theme_hex_colour() {
    let (_dir, mut cmd) = lx_with_theme(
        "version = \"0.2\"\n[theme.test]\ndate = \"#ff8700\"\n"
    );

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;255;135;0m"));
}


// ── Extension and filename overrides ─────────────────────────────

#[test]
fn theme_extension_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        use-extensions = "myexts"
        [extensions.myexts]
        txt = "bold magenta"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("readme.txt"), "").unwrap();

    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;35m"));
}

#[test]
fn theme_filename_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        use-filenames = "mynames"
        [filenames.mynames]
        Makefile = "bold underline yellow"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("Makefile"), "").unwrap();

    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;4;33m"));
}


// ── Personality integration ──────────────────────────────────────

#[test]
fn theme_via_personality() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [personality.lx]
        theme = "ocean"

        [theme.ocean]
        date = "bold cyan"
    "#);

    // The lx personality should activate the ocean theme.
    cmd.args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_inherited_through_personality() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [personality.default]
        theme = "ocean"

        [personality.myview]
        inherits = "default"
        format = "long"

        [theme.ocean]
        date = "bold cyan"
    "#);

    cmd.args(["-pmyview", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_cli_overrides_personality() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [personality.lx]
        theme = "ocean"

        [theme.ocean]
        date = "bold cyan"

        [theme.warm]
        date = "bold red"
    "#);

    // --theme=warm should override the personality's theme = "ocean"
    cmd.args(["--theme=warm", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"));
}


// ── Precedence over env vars ─────────────────────────────────────

#[test]
fn theme_overrides_env() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        date = "bold red"
    "#);

    cmd.env("LX_COLORS", "da=32")
        .args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"));
}


// ── Reset extensions ─────────────────────────────────────────────

#[test]
fn theme_reset_extensions() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.test]
        reset-extensions = true
    "#);

    cmd.args(["--theme=test", "-1", "Cargo.toml"])
        .assert()
        .success();
}


// ── Theme inheritance ────────────────────────────────────────────

#[test]
fn theme_inherits_exa() {
    // A theme inheriting from "exa" should get the compiled-in
    // defaults, then override specific keys.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.custom]
        inherits = "exa"
        date = "bold red"
    "#);

    // date is bold red (overridden), but directory should still be
    // bold blue (inherited from exa).
    cmd.args(["--theme=custom", "-l", "src"])
        .assert()
        .success()
        // Bold blue directory (from exa): 1;34
        .stdout(predicate::str::contains("\x1b[1;34m"))
        // Bold red date (from custom): 1;31
        .stdout(predicate::str::contains("\x1b[1;31m"));
}

#[test]
fn theme_without_inherits_is_blank() {
    // A theme without inherits starts from plain — only its own
    // keys apply.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.bare]
        date = "bold red"
    "#);

    // date is bold red, but directory should NOT be bold blue
    // (no exa defaults).
    cmd.args(["--theme=bare", "-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"))
        // No bold blue (1;34m) — directories are unstyled.
        .stdout(predicate::str::contains("\x1b[1;34m").not());
}

#[test]
fn theme_inherits_custom() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.base]
        inherits = "exa"
        date = "bold cyan"

        [theme.child]
        inherits = "base"
        directory = "bold red"
    "#);

    // child: directory=bold red, date=bold cyan (from base),
    // everything else from exa.
    cmd.args(["--theme=child", "-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"))   // bold red dir
        .stdout(predicate::str::contains("\x1b[1;36m"));   // bold cyan date
}

#[test]
fn theme_inheritance_cycle_detected() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.2"
        [theme.a]
        inherits = "b"
        [theme.b]
        inherits = "a"
    "#);

    // Should still work (warning emitted), just no theme applied.
    cmd.args(["--theme=a", "-1", "Cargo.toml"])
        .assert()
        .success();
}


// ── No theme is fine ─────────────────────────────────────────────

#[test]
fn no_theme_works() {
    lx_no_colour()
        .args(["-1", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}
