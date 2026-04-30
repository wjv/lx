//! Basic CLI smoke tests: help, version, error handling, simple listings.

mod support;

use predicates::prelude::*;
use support::{lx, lx_no_colour};

// ── Help and version ──────────────────────────────────────────────

#[test]
fn help_flag() {
    lx().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("personality"));
}

#[test]
fn help_short_flag() {
    lx().arg("-?")
        .assert()
        .success()
        .stdout(predicate::str::contains("--oneline"));
}

#[test]
fn version_flag() {
    lx().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_short_flag() {
    lx().arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("lx"));
}

// ── Invalid options ───────────────────────────────────────────────

#[test]
fn unknown_short_flag() {
    lx().arg("-Y")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn uppercase_f_groups_dirs_first() {
    // -F is now short for --group-dirs=first (was --classify in exa)
    lx().arg("-F").assert().success();
}

#[test]
fn unknown_long_flag() {
    lx().arg("--ternary")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn invalid_sort_value() {
    lx().arg("--sort=colour")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn removed_time_flag_is_rejected() {
    // --time=X was removed in 0.8; it is no longer a valid flag.
    lx().arg("--time=modified").assert().failure().stderr(
        predicate::str::contains("unexpected argument")
            .or(predicate::str::contains("unrecognized")),
    );
}

#[test]
fn invalid_colour_value() {
    lx().arg("--colour=upstream")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn unknown_time_style_errors() {
    // Unknown --time-style values are rejected by clap's value_parser
    // (via the `TimeStyleParser` TypedValueParser), so the error is
    // formatted natively with exit code 2 and a [possible values: ...]
    // hint.
    lx_no_colour()
        .args(["--time-style=24-hour", "-l", "Cargo.toml"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "invalid value '24-hour' for '--time-style",
        ))
        .stderr(predicate::str::contains(
            "[possible values: default, iso, long-iso, full-iso, relative, +FORMAT]",
        ));
}

#[test]
fn invalid_level_not_a_number() {
    lx().arg("--level=abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn time_tier_compounds() {
    // -t is now a compounding flag like -l; -tt adds changed on top
    // of modified and should succeed.
    lx_no_colour()
        .args(["-ltt", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn tree_all_all_error() {
    lx().args(["-Taa"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--tree"));
}

// ── Exit codes ────────────────────────────────────────────────────

#[test]
fn exit_code_success() {
    lx().arg(".").assert().success();
}

#[test]
fn exit_code_options_error() {
    lx().arg("--sort=nope").assert().code(2); // Clap uses exit code 2 for usage errors
}

#[test]
fn nonexistent_path_still_exits() {
    // lx should print an error and exit non-zero for missing paths
    lx().arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .failure();
}

// ── Basic listing ─────────────────────────────────────────────────

#[test]
fn list_current_directory() {
    // Should produce some output and succeed
    lx_no_colour()
        .arg(".")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn list_specific_file() {
    lx_no_colour()
        .arg("Cargo.toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn oneline_view() {
    lx_no_colour()
        .args(["-1", "Cargo.toml", "Cargo.lock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"))
        .stdout(predicate::str::contains("Cargo.lock"));
}

#[test]
fn long_view() {
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn long_view_with_header() {
    lx_no_colour()
        .args(["-lh", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"))
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn grid_view() {
    lx_no_colour()
        .args(["-G", "."])
        .env("COLUMNS", "80")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── View mode combinations ────────────────────────────────────────

#[test]
fn tree_view() {
    lx_no_colour()
        .args(["-T", "--level=1", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn recurse_view() {
    lx_no_colour()
        .args(["-R", "--level=1", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn long_grid_view() {
    lx_no_colour()
        .args(["-lG", "."])
        .env("COLUMNS", "200")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── Filtering ─────────────────────────────────────────────────────

#[test]
fn sort_by_name() {
    lx_no_colour()
        .args(["-1", "--sort=name", "."])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn sort_by_size() {
    lx_no_colour()
        .args(["-1", "--sort=size", "."])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn reverse_sort() {
    lx_no_colour()
        .args(["-1r", "--sort=name", "."])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn ignore_glob() {
    lx_no_colour()
        .args(["-1", "-I=*.lock", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.lock").not());
}

#[test]
fn only_dirs() {
    lx_no_colour()
        .args(["-1D", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("src"))
        .stdout(predicate::str::contains("Cargo.toml").not());
}

#[test]
fn only_dirs_filters_cli_arguments() {
    // Regression: `--only-dirs` previously applied only to files
    // discovered inside a directory, never to CLI-named files
    // themselves.  With `-d` (treat directories as files), every
    // listed file goes through the argument-filter path, exposing
    // the gap.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::write(root.join("a_file.txt"), "x").unwrap();
    std::fs::create_dir(root.join("a_dir")).unwrap();

    lx_no_colour()
        .current_dir(root)
        .args(["-1dD", "a_file.txt", "a_dir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a_dir"))
        .stdout(predicate::str::contains("a_file.txt").not());
}

#[test]
fn only_files_filters_cli_arguments() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::write(root.join("a_file.txt"), "x").unwrap();
    std::fs::create_dir(root.join("a_dir")).unwrap();

    lx_no_colour()
        .current_dir(root)
        .args(["-1df", "a_file.txt", "a_dir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a_file.txt"))
        .stdout(predicate::str::contains("a_dir").not());
}

// ── Long-view columns ─────────────────────────────────────────────

#[test]
fn binary_sizes() {
    lx_no_colour()
        .args(["-lb", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn byte_sizes() {
    lx_no_colour()
        .args(["-lB", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn uid_and_gid_columns() {
    // --uid and --gid are first-class columns as of 0.8 (batch C);
    // the old `-n`/`--numeric` shortcut is gone.
    lx_no_colour()
        .args(["-l", "--uid", "--gid", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn group_column() {
    lx_no_colour()
        .args(["-lg", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn inode_column() {
    lx_no_colour()
        .args(["-li", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

// ── Suppressed columns ───────────────────────────────────────────

#[test]
fn no_permissions() {
    lx_no_colour()
        .args(["-l", "--no-permissions", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn no_filesize() {
    lx_no_colour()
        .args(["-l", "--no-filesize", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn no_user() {
    lx_no_colour()
        .args(["-l", "--no-user", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn no_time() {
    lx_no_colour()
        .args(["-l", "--no-time", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

// ── Colour control ────────────────────────────────────────────────

#[test]
fn colour_always() {
    lx().args(["--colour=always", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn colour_never() {
    lx().args(["--colour=never", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn colour_auto() {
    lx().args(["--colour=auto", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn no_color_env() {
    lx().args(["-1", "Cargo.toml"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
}

// ── Shell completions ─────────────────────────────────────────────

#[test]
fn completions_bash() {
    lx().arg("--completions=bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_lx"));
}

#[test]
fn completions_zsh() {
    lx().arg("--completions=zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef lx"));
}

#[test]
fn completions_fish() {
    lx().arg("--completions=fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c lx"));
}

// ── Environment variables ─────────────────────────────────────────

#[test]
fn columns_env_controls_width() {
    // Should succeed with a narrow terminal width
    lx_no_colour()
        .args(["-G", "."])
        .env("COLUMNS", "40")
        .assert()
        .success();
}

#[test]
fn invalid_columns_env() {
    lx_no_colour()
        .args(["-G", "."])
        .env("COLUMNS", "abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("COLUMNS"));
}

#[test]
fn invalid_lx_grid_rows_env() {
    lx_no_colour()
        .args(["-lG", "."])
        .env("LX_GRID_ROWS", "not-a-number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("LX_GRID_ROWS"));
}

// ── Long-view flags without --long (silently ignored) ─────────────

#[test]
fn binary_without_long_is_fine() {
    lx_no_colour().args(["--binary", "."]).assert().success();
}

#[test]
fn header_without_long_is_fine() {
    lx_no_colour().args(["--header", "."]).assert().success();
}

#[test]
fn level_without_recurse_is_fine() {
    lx_no_colour().args(["--level=3", "."]).assert().success();
}

// ── Regression tests for wjv/lx#33 and wjv/lx#34 ────────────────

/// `-R -L<N>` with a positional path should descend N levels,
/// exactly as if invoked from inside that path.  Previously, depth
/// was measured against absolute path components, so an absolute
/// positional argument always exceeded the limit and recursion
/// stopped after the first level.
#[test]
fn recurse_with_level_and_positional_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("a/aa/aaa")).unwrap();
    std::fs::write(root.join("a/aa/aaa/leaf"), b"").unwrap();

    // -RL2 should recurse one level deep: lists `a` then its
    // contents (`aa`), and stops there.
    let assert = lx_no_colour()
        .args(["-RL2", root.to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(stdout.contains("aa"), "depth-2 should reach `aa`: {stdout}");
    assert!(
        !stdout.contains("aaa"),
        "depth-2 should NOT reach `aaa`: {stdout}"
    );

    // -RL3 should reach one level deeper.
    let assert = lx_no_colour()
        .args(["-RL3", root.to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        stdout.contains("aaa"),
        "depth-3 should reach `aaa`: {stdout}"
    );
}

/// `-T` on a symlinked directory passed as a positional argument
/// should follow the symlink and render its contents as a tree,
/// matching the bare and `-R` behaviour.
#[cfg(unix)]
#[test]
fn tree_follows_symlinked_positional_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("target");
    let link = dir.path().join("link");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("leaf"), b"").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    lx_no_colour()
        .args(["-T", link.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("leaf"));
}
