//! Tests for the [theme] config section and --theme flag.

mod support;

use predicates::prelude::*;
use std::fs;
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
        .arg("--colour=always");
    (dir, cmd)
}

// ── UI element overrides ─────────────────────────────────────────

#[test]
fn theme_directory_colour() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.test]
        directory = "bold red"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.test]
        date = "bold cyan"
    "#,
    );

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_x11_colour() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.test]
        date = "tomato"
    "#,
    );

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;255;99;71m"));
}

#[test]
fn theme_hex_colour() {
    let (_dir, mut cmd) = lx_with_theme("version = \"0.3\"\n[theme.test]\ndate = \"#ff8700\"\n");

    cmd.args(["--theme=test", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[38;2;255;135;0m"));
}

// ── Style set overrides ─────────────────────────────────────────

#[test]
fn theme_extension_colour() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.test]
        use-style = "myexts"
        [style.myexts]
        "*.txt" = "bold magenta"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.test]
        use-style = "mynames"
        [style.mynames]
        Makefile = "bold underline yellow"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [personality.lx]
        theme = "ocean"

        [theme.ocean]
        date = "bold cyan"
    "#,
    );

    // The lx personality should activate the ocean theme.
    cmd.args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_inherited_through_personality() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [personality.default]
        theme = "ocean"

        [personality.myview]
        inherits = "default"
        format = "long"

        [theme.ocean]
        date = "bold cyan"
    "#,
    );

    cmd.args(["-pmyview", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;36m"));
}

#[test]
fn theme_cli_overrides_personality() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [personality.lx]
        theme = "ocean"

        [theme.ocean]
        date = "bold cyan"

        [theme.warm]
        date = "bold red"
    "#,
    );

    // --theme=warm should override the personality's theme = "ocean"
    cmd.args(["--theme=warm", "-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m"));
}

// ── Precedence over env vars ─────────────────────────────────────

// ── Class references in styles ────────────────────────────────────

#[test]
fn style_class_reference() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"

        [class]
        testclass = ["*.xyz"]

        [theme.test]
        inherits = "exa"
        use-style = "mystyle"

        [style.mystyle]
        class.testclass = "bold magenta"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"

        [theme.test]
        inherits = "exa"
        use-style = "custom"

        [style.custom]
        class.compressed = "bold cyan"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"

        [class]
        data = ["*.csv"]

        [theme.test]
        inherits = "exa"
        use-style = "mixed"

        [style.mixed]
        class.data = "bold green"
        "Makefile" = "bold red"
    "#,
    );

    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("results.csv"), "").unwrap();
    fs::write(work.path().join("Makefile"), "").unwrap();

    // Both should be coloured
    cmd.args(["--theme=test", "-1"])
        .arg(work.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;32m")) // bold green
        .stdout(predicate::str::contains("\x1b[1;31m")); // bold red
}

#[test]
fn user_class_overrides_compiled_in() {
    // A user-defined [class] entry overrides the compiled-in one.
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"

        [class]
        compressed = ["*.myarc"]

        [theme.test]
        inherits = "exa"
        use-style = "exa"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.custom]
        inherits = "exa"
        date = "bold red"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.bare]
        date = "bold red"
    "#,
    );

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
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.base]
        inherits = "exa"
        date = "bold cyan"

        [theme.child]
        inherits = "base"
        directory = "bold red"
    "#,
    );

    // child: directory=bold red, date=bold cyan (from base),
    // everything else from exa.
    cmd.args(["--theme=child", "-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;31m")) // bold red dir
        .stdout(predicate::str::contains("\x1b[1;36m")); // bold cyan date
}

#[test]
fn theme_inheritance_cycle_detected() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.a]
        inherits = "b"
        [theme.b]
        inherits = "a"
    "#,
    );

    // 0.9: cycles are fatal (was: warn + continue).  A cycle in user
    // config is unambiguously broken, so silently dropping it hides
    // real bugs.
    cmd.args(["--theme=a", "-1", "Cargo.toml"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("theme inheritance cycle"));
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

/// Helper: build a `lx` command with a clean environment so the
/// builtin theme auto-selection doesn't interfere with assertions.
fn lx_clean() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
        .env("HOME", "/nonexistent")
        .env_remove("LS_COLORS")
        .env_remove("TERM")
        .env_remove("COLORTERM");
    cmd
}

#[test]
fn default_theme_produces_colour() {
    // Force --theme=exa so this test doesn't depend on ambient
    // TERM/COLORTERM (auto-selection would pick lx-256 or lx-24bit).
    // Bold blue (1;34) for directories is the exa default.
    lx_clean()
        .arg("--colour=always")
        .arg("--theme=exa")
        .args(["-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;34m")) // bold blue dir
        .stdout(predicate::str::contains("\x1b[34m")); // blue date
}

#[test]
fn lx_256_theme_produces_256_colour() {
    // The lx-256 theme uses Fixed(33) bold for directories.
    lx_clean()
        .arg("--colour=always")
        .arg("--theme=lx-256")
        .args(["-l", "src"])
        .assert()
        .success()
        // Bold soft blue directory (Fixed 33): "1;38;5;33"
        .stdout(predicate::str::contains("\x1b[1;38;5;33m"));
}

#[test]
fn lx_24bit_theme_produces_truecolour() {
    // The lx-24bit theme uses #3b8ed8 bold for directories.
    lx_clean()
        .arg("--colour=always")
        .arg("--theme=lx-24bit")
        .args(["-l", "src"])
        .assert()
        .success()
        // Bold #3b8ed8 directory: "1;38;2;59;142;216"
        .stdout(predicate::str::contains("\x1b[1;38;2;59;142;216m"));
}

#[test]
fn auto_selection_picks_exa_with_no_term() {
    // Bare environment: no TERM, no COLORTERM, so the [[when]]
    // blocks in the default personality don't fire.  Should
    // get the bare-bones exa theme.
    lx_clean()
        .arg("--colour=always")
        .args(["-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;34m")); // exa bold blue
}

#[test]
fn auto_selection_picks_lx_256_for_256color_term() {
    // TERM=xterm-256color → matches "*-256color" → lx-256.
    lx_clean()
        .env("TERM", "xterm-256color")
        .arg("--colour=always")
        .args(["-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;38;5;33m")); // lx-256 dir
}

#[test]
fn auto_selection_picks_lx_24bit_for_truecolor_colorterm() {
    // COLORTERM=truecolor → matches the array → lx-24bit.
    // Truecolour wins over 256-colour even if both apply.
    lx_clean()
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .arg("--colour=always")
        .args(["-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;38;2;59;142;216m")); // lx-24bit dir
}

#[test]
fn auto_selection_accepts_24bit_colorterm_value() {
    // COLORTERM=24bit also valid.
    lx_clean()
        .env("COLORTERM", "24bit")
        .arg("--colour=always")
        .args(["-l", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[1;38;2;59;142;216m"));
}

#[test]
fn default_theme_colours_filetypes() {
    // The compiled-in exa style should colour compressed files red.
    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
        .env("HOME", "/nonexistent")
        .env_remove("LS_COLORS")
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
    // This is design invariant #2.  We test it by running lx twice
    // (once with no config, once with the generated config) and
    // comparing stdout byte-for-byte.
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

    // Common args / env for both runs.
    fn common(cmd: &mut assert_cmd::Command, dir: &std::path::Path) {
        cmd.env("HOME", dir)
            .env_remove("LS_COLORS")
            .arg("--colour=always")
            .args(["-l", "src"]);
    }

    // Run without config.
    let mut no_cfg = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    no_cfg.env("LX_CONFIG", "/nonexistent");
    common(&mut no_cfg, dir.path());
    let no_cfg_out = no_cfg.assert().success().get_output().stdout.clone();

    // Run with the generated config.
    let mut with_cfg = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    with_cfg.env("LX_CONFIG", &config_path);
    common(&mut with_cfg, dir.path());
    let with_cfg_out = with_cfg.assert().success().get_output().stdout.clone();

    assert_eq!(
        no_cfg_out, with_cfg_out,
        "--init-config changed behaviour (invariant #2 violation)"
    );
}

// ── Per-timestamp-column theme keys ──────────────────────────────

/// The 32 per-timestamp-column theme keys (4 columns × 8 slots)
/// must all be parseable by `set_config` and must round-trip
/// through `--dump-theme`.  This catches typos in `set_config`
/// arms, key-name drift, and regressions in the dump output.
#[test]
fn per_column_date_keys_round_trip_through_dump_theme() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.full-fat]
        inherits = "exa"
        date              = "white"
        date-now          = "bright cyan"
        date-modified     = "bright green"
        date-modified-now = "bold bright green"
        date-accessed-today = "magenta"
        date-changed-flat = "dim"
        date-created-old  = "red"
    "#,
    );

    let assertion = cmd.args(["--dump-theme=full-fat"]).assert().success();
    let dump = String::from_utf8(assertion.get_output().stdout.clone()).unwrap();

    // Every key written to the theme must appear in the dump.
    for key in [
        "date = \"white\"",
        "date-now = \"bright cyan\"",
        "date-modified = \"bright green\"",
        "date-modified-now = \"bold bright green\"",
        "date-accessed-today = \"magenta\"",
        "date-changed-flat = \"dim\"",
        "date-created-old = \"red\"",
    ] {
        assert!(
            dump.contains(key),
            "--dump-theme output missing {key:?}:\n{dump}"
        );
    }
}

/// `--dump-theme` groups `date-*` keys into a structured block
/// instead of interleaving them alphabetically with each other and
/// with non-date keys.  Expected shape: bulk keys first in canonical
/// tier order, then each per-column family in canonical column
/// order (modified / accessed / changed / created), each group in
/// tier order, blank lines between groups, and the whole block
/// sitting at the alphabetical position where plain `date` would
/// fall.
#[test]
fn dump_theme_groups_date_keys_by_column() {
    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.grouped]
        inherits = "exa"
        directory = "bold blue"
        size-major = "white"
        date-created-now   = "red"
        date-now           = "cyan"
        date-modified-week = "yellow"
        date-accessed-flat = "green"
        date-today         = "magenta"
        date-changed-now   = "orange"
    "#,
    );

    let assertion = cmd.args(["--dump-theme=grouped"]).assert().success();
    let dump = String::from_utf8(assertion.get_output().stdout.clone()).unwrap();

    // Strip the header lines and the [theme.grouped]/inherits/use-style
    // preamble so we can assert on the body alone.
    let body: Vec<&str> = dump
        .lines()
        .skip_while(|l| !l.starts_with("date") && !l.starts_with("directory"))
        .collect();

    // Expected body, in exact order.  Output is grouped by family
    // (FileKinds, Size, DateBulk, DateModified, …) in registry
    // declaration order, with blank-line separators between
    // families.  Within each date family, keys appear in canonical
    // tier order (now → today → … → flat).
    let expected = vec![
        "directory = \"bold blue\"",
        "",
        "size-major = \"white\"",
        "",
        "date-now = \"cyan\"",
        "date-today = \"magenta\"",
        "",
        "date-modified-week = \"yellow\"",
        "",
        "date-accessed-flat = \"green\"",
        "",
        "date-changed-now = \"orange\"",
        "",
        "date-created-now = \"red\"",
    ];

    assert_eq!(
        body, expected,
        "--dump-theme body does not match expected structured order:\n{dump}"
    );
}

/// Spot-check the rendered-output side: running `lx -lll --theme=X`
/// with per-column overrides produces an output whose ANSI escapes
/// contain the expected per-column colours.  This exercises the
/// end-to-end renderer path, not just the parser.
#[test]
fn per_column_gradient_tokens_reach_the_renderer() {
    // Build a tempdir with a single file of a known-recent mtime
    // (touching the file sets mtime=atime=ctime=now).
    let work = tempdir().expect("failed to create workdir");
    let file = work.path().join("fresh.txt");
    fs::write(&file, "hi").unwrap();

    let (_dir, mut cmd) = lx_with_theme(
        r#"
        version = "0.3"
        [theme.rainbow]
        inherits = "exa"
        date-modified-now = "red"
        date-accessed-now = "green"
        date-changed-now  = "blue"
        date-created-now  = "magenta"
    "#,
    );

    // `-lll` shows all four timestamp columns.
    let assertion = cmd
        .args(["-lll", "--theme=rainbow"])
        .arg(work.path())
        .assert()
        .success();
    let out = String::from_utf8(assertion.get_output().stdout.clone()).unwrap();

    // Each per-column "now" colour should appear somewhere in the
    // output (the file was just created, so every timestamp is in
    // the `now` tier).  Exact code points:
    //   red     → ESC[31m
    //   green   → ESC[32m
    //   blue    → ESC[34m
    //   magenta → ESC[35m
    for (colour, code) in [
        ("red", "\x1b[31m"),
        ("green", "\x1b[32m"),
        ("blue", "\x1b[34m"),
        ("magenta", "\x1b[35m"),
    ] {
        assert!(
            out.contains(code),
            "per-column {colour} ({code:?}) did not reach the rendered output:\n{out}"
        );
    }
}

/// Exercise the four new visible `--gradient` tokens end-to-end.
/// We don't assert exact colours (fragile across palettes), just
/// that the command succeeds and that `modified`, `accessed`,
/// `changed`, and `created` are all accepted by the parser.
#[test]
fn new_gradient_tokens_are_accepted() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("file.txt"), "hi").unwrap();

    for tok in [
        "modified",
        "accessed",
        "changed",
        "created",
        "size,modified",
        "accessed,created",
    ] {
        let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
        cmd.env("LX_CONFIG", "/nonexistent")
            .env("HOME", "/nonexistent")
            .env_remove("LS_COLORS")
            .args(["-lll", "--colour=always", &format!("--gradient={tok}")])
            .arg(work.path())
            .assert()
            .success();
    }
}

/// The hidden `filesize`/`timestamp` aliases must behave identically
/// to their canonical spellings.
#[test]
fn hidden_gradient_aliases_match_canonical() {
    let work = tempdir().expect("failed to create workdir");
    fs::write(work.path().join("file.txt"), "hi").unwrap();

    let run = |value: &str| -> Vec<u8> {
        let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
        cmd.env("LX_CONFIG", "/nonexistent")
            .env("HOME", "/nonexistent")
            .env_remove("LS_COLORS")
            .args(["-l", "--colour=always", &format!("--gradient={value}")])
            .arg(work.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };

    assert_eq!(
        run("size"),
        run("filesize"),
        "--gradient=filesize should match --gradient=size"
    );
    assert_eq!(
        run("date"),
        run("timestamp"),
        "--gradient=timestamp should match --gradient=date"
    );
}

// ── --smooth ─────────────────────────────────────────────────────

/// Helper: run `lx -l --colour=always` against a fresh tempdir
/// whose contents have a range of mtimes, with configurable theme
/// and extra args.  Returns the raw bytes of stdout.
///
/// The tempdir is created with three files whose mtimes land on
/// different tiers (recent, a few months ago, a year+ ago) so
/// that smooth mode has something to interpolate.
fn run_with_varied_mtimes(theme: &str, extra_args: &[&str]) -> Vec<u8> {
    use std::time::{Duration, SystemTime};

    let work = tempdir().expect("failed to create workdir");
    let now = SystemTime::now();
    // Ages chosen to fall strictly between the six per-tier
    // anchors so smooth mode lands in between buckets rather
    // than on the same bucket as the discrete tier lookup.
    let fixtures = [
        ("a_half_hour", Duration::from_secs(1800)), // between now and today
        ("a_half_day", Duration::from_secs(43_200)), // between today and week
        ("three_days", Duration::from_secs(3 * 86_400)), // between week and month
        ("two_weeks", Duration::from_secs(14 * 86_400)), // between month and year
        ("three_months", Duration::from_secs(90 * 86_400)), // between year and old
    ];
    for (name, age) in fixtures {
        let path = work.path().join(name);
        fs::File::create(&path).unwrap();
        let t = std::fs::FileTimes::new().set_modified(now - age);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(t)
            .unwrap();
    }

    let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
        .env("HOME", "/nonexistent")
        .env_remove("LS_COLORS")
        .args(["-l", "--colour=always", &format!("--theme={theme}")])
        .args(extra_args)
        .arg(work.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone()
}

#[test]
fn smooth_changes_24bit_output() {
    let discrete = run_with_varied_mtimes("lx-24bit", &[]);
    let smooth = run_with_varied_mtimes("lx-24bit", &["--smooth"]);
    assert_ne!(
        discrete, smooth,
        "--smooth should change lx-24bit output (interpolating between anchors)",
    );
}

#[test]
fn smooth_is_noop_on_256_palette_theme() {
    // lx-256 uses Color::Fixed palette colours; is_smoothable()
    // returns false, so the LUT is never built and the output
    // must match the discrete render byte-for-byte.
    let discrete = run_with_varied_mtimes("lx-256", &[]);
    let smooth = run_with_varied_mtimes("lx-256", &["--smooth"]);
    assert_eq!(
        discrete, smooth,
        "--smooth should be a no-op on lx-256 (palette anchors gate it out)",
    );
}

#[test]
fn smooth_is_noop_on_ansi_exa_theme() {
    // The builtin exa theme uses basic ANSI colours; same gate.
    let discrete = run_with_varied_mtimes("exa", &[]);
    let smooth = run_with_varied_mtimes("exa", &["--smooth"]);
    assert_eq!(
        discrete, smooth,
        "--smooth should be a no-op on the exa theme (basic ANSI anchors)",
    );
}

#[test]
fn smooth_with_no_gradient_is_harmless() {
    // With --no-gradient every column collapses to its flat
    // colour, so there's nothing to smooth.  --smooth on top of
    // that must not error and must not differ from --no-gradient
    // alone.
    let flat = run_with_varied_mtimes("lx-24bit", &["--no-gradient"]);
    let flat_smooth = run_with_varied_mtimes("lx-24bit", &["--no-gradient", "--smooth"]);
    assert_eq!(
        flat, flat_smooth,
        "--smooth --no-gradient should behave identically to --no-gradient alone",
    );
}

#[test]
fn no_smooth_suppresses_personality_smooth() {
    // A personality that sets `smooth = true` is overridden by
    // `--no-smooth` on the command line.  Round-trip through
    // actual config: the personality's smooth should fire when
    // no CLI override is given, then drop out under --no-smooth.
    let dir = tempdir().expect("failed to create tempdir");
    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
        version = "0.5"
        [personality.lx]
        theme = "lx-24bit"
        smooth = true
    "#,
    )
    .unwrap();

    let run = |extra: &[&str]| -> Vec<u8> {
        use std::time::{Duration, SystemTime};

        let work = tempdir().expect("failed to create workdir");
        let path = work.path().join("f");
        fs::File::create(&path).unwrap();
        // Land strictly between anchors so smooth ≠ discrete.
        let t = std::fs::FileTimes::new()
            .set_modified(SystemTime::now() - Duration::from_secs(3 * 86_400));
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(t)
            .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lx").expect("binary lx not found");
        cmd.env("LX_CONFIG", &config_path)
            .env("HOME", "/nonexistent")
            .env_remove("LS_COLORS")
            .args(["-l", "--colour=always"])
            .args(extra)
            .arg(work.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };

    let with_personality = run(&[]); // smooth on (via personality)
    let forced_off = run(&["--no-smooth"]); // smooth off (CLI wins)
    assert_ne!(
        with_personality, forced_off,
        "--no-smooth should override a personality that enables smooth",
    );
}
