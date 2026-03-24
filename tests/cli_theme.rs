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
        version = "0.3"
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
        version = "0.3"
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
        version = "0.3"
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
        "version = \"0.3\"\n[theme.test]\ndate = \"#ff8700\"\n"
    );

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;255;135;0m"));
}


// ── Style set overrides ─────────────────────────────────────────

#[test]
fn theme_extension_colour() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"
        [theme.test]
        use-style = "myexts"
        [style.myexts]
        "*.txt" = "bold magenta"
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
        version = "0.3"
        [theme.test]
        use-style = "mynames"
        [style.mynames]
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
        version = "0.3"
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
        version = "0.3"
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
        version = "0.3"
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
        version = "0.3"
        [theme.test]
        date = "bold red"
    "#);

    cmd.env("LX_COLORS", "da=32")
        .args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"));
}


// ── Class references in styles ────────────────────────────────────

#[test]
fn style_class_reference() {
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"

        [class]
        testclass = ["*.xyz"]

        [theme.test]
        inherits = "exa"
        use-style = "mystyle"

        [style.mystyle]
        class.testclass = "bold magenta"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("data.xyz"), "").unwrap();

    // Bold magenta = 1;35
    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;35m"));
}

#[test]
fn style_class_overrides_exa_default() {
    // User style with a class reference should override the
    // compiled-in exa style for the same files.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"

        [theme.test]
        inherits = "exa"
        use-style = "custom"

        [style.custom]
        class.compressed = "bold cyan"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("archive.zip"), "").unwrap();

    // Bold cyan = 1;36 (not red, which is the exa default)
    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn style_quoted_pattern_and_class() {
    // A style can mix class references and quoted file patterns.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"

        [class]
        data = ["*.csv"]

        [theme.test]
        inherits = "exa"
        use-style = "mixed"

        [style.mixed]
        class.data = "bold green"
        "Makefile" = "bold red"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("results.csv"), "").unwrap();
    fs::write(work.path().join("Makefile"), "").unwrap();

    // Both should be coloured
    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;32m"))   // bold green
        .stdout(predicate::str::contains("\x1b[1;31m"));   // bold red
}

#[test]
fn user_class_overrides_compiled_in() {
    // A user-defined [class] entry overrides the compiled-in one.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"

        [class]
        compressed = ["*.myarc"]

        [theme.test]
        inherits = "exa"
        use-style = "exa"
    "#);

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("data.myarc"), "").unwrap();
    // .zip should NOT match compressed anymore (user redefined it)
    fs::write(work.path().join("stuff.zip"), "").unwrap();

    // .myarc gets the exa compressed colour (red = 31)
    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[31m"));
}


// ── Theme inheritance ────────────────────────────────────────────

#[test]
fn theme_inherits_exa() {
    // A theme inheriting from "exa" should get the compiled-in
    // defaults, then override specific keys.
    let (_dir, mut cmd) = lx_with_theme(r#"
        version = "0.3"
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
        version = "0.3"
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
        version = "0.3"
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
        version = "0.3"
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

// ── Default theme smoke tests ────────────────────────────────────

#[test]
fn no_theme_works() {
    lx_no_colour()
        .args(["-1", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn default_theme_produces_colour() {
    // Without any config, the compiled-in exa theme (via the
    // default personality) should produce coloured output.
    // Bold blue (1;34) for directories is the exa default.
    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
       .env("HOME", "/nonexistent")
       .env_remove("LS_COLORS")
       .env_remove("LX_COLORS")
       .arg("--colour=always")
       .args(["-l", "src"])
       .assert()
       .success()
       // Bold blue directory (from exa theme): 1;34
       .stdout(predicate::str::contains("\x1b[1;34m"))
       // Blue date (from exa theme): 34
       .stdout(predicate::str::contains("\x1b[34m"));
}

#[test]
fn default_theme_colours_filetypes() {
    // The compiled-in exa style should colour compressed files red.
    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
       .env("HOME", "/nonexistent")
       .env_remove("LS_COLORS")
       .env_remove("LX_COLORS")
       .arg("--colour=always");

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("archive.zip"), "").unwrap();

    // Red (31) for compressed files (from exa style).
    cmd.args(["-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[31m"));
}

#[test]
fn init_config_preserves_default_colours() {
    // After --init-config, output should look identical to no-config.
    // This is design invariant #2.
    let dir = tempdir().expect("failed to create tempdir");

    // Generate config.
    let mut init = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    init.args(["--init-config"])
        .env("HOME", dir.path())
        .env("LX_CONFIG", "/nonexistent")
        .assert()
        .success();

    let config_path = dir.path().join(".lxconfig.toml");
    assert!(config_path.exists());

    // Run with generated config — should have bold blue directories.
    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", &config_path)
       .env("HOME", dir.path())
       .env_remove("LS_COLORS")
       .env_remove("LX_COLORS")
       .arg("--colour=always")
       .args(["-l", "src"])
       .assert()
       .success()
       .stdout(predicate::str::contains("\x1b[1;34m"));
}
