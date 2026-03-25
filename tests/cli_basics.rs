//! Basic CLI smoke tests: help, version, error handling, simple listings.

mod support;

use predicates::prelude::*;
use support::{lx, lx_no_colour};


// ── Help and version ──────────────────────────────────────────────

#[test]
fn help_flag() {
    lx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List eXtended"));
}

#[test]
fn help_short_flag() {
    lx()
        .arg("-?")
        .assert()
        .success()
        .stdout(predicate::str::contains("--oneline"));
}

#[test]
fn version_flag() {
    lx()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_short_flag() {
    lx()
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("lx"));
}


// ── Invalid options ───────────────────────────────────────────────

#[test]
fn unknown_short_flag() {
    lx()
        .arg("-Y")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn uppercase_f_groups_dirs_first() {
    // -F is now short for --group-dirs=first (was --classify in exa)
    lx()
        .arg("-F")
        .assert()
        .success();
}

#[test]
fn unknown_long_flag() {
    lx()
        .arg("--ternary")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn invalid_sort_value() {
    lx()
        .arg("--sort=colour")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn invalid_time_value() {
    lx()
        .arg("--time=tea")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn invalid_colour_value() {
    lx()
        .arg("--colour=upstream")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn invalid_time_style_value() {
    lx()
        .arg("--time-style=24-hour")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn invalid_level_not_a_number() {
    lx()
        .arg("--level=abc")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn time_conflicts_with_modified() {
    lx()
        .args(["-ltmod", "-m"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn tree_all_all_error() {
    lx()
        .args(["-Taa"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--tree"));
}


// ── Exit codes ────────────────────────────────────────────────────

#[test]
fn exit_code_success() {
    lx()
        .arg(".")
        .assert()
        .success();
}

#[test]
fn exit_code_options_error() {
    lx()
        .arg("--sort=nope")
        .assert()
        .code(2);  // Clap uses exit code 2 for usage errors
}

#[test]
fn nonexistent_path_still_exits() {
    // lx should print an error and exit non-zero for missing paths
    lx()
        .arg("/nonexistent/path/that/does/not/exist")
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
fn numeric_ids() {
    lx_no_colour()
        .args(["-ln", "Cargo.toml"])
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
    lx()
        .args(["--colour=always", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn colour_never() {
    lx()
        .args(["--colour=never", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn colour_auto() {
    lx()
        .args(["--colour=auto", "-1", "Cargo.toml"])
        .assert()
        .success();
}

#[test]
fn no_color_env() {
    lx()
        .args(["-1", "Cargo.toml"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
}


// ── Shell completions ─────────────────────────────────────────────

#[test]
fn completions_bash() {
    lx()
        .arg("--completions=bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_lx"));
}

#[test]
fn completions_zsh() {
    lx()
        .arg("--completions=zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef lx"));
}

#[test]
fn completions_fish() {
    lx()
        .arg("--completions=fish")
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
    lx_no_colour()
        .args(["--binary", "."])
        .assert()
        .success();
}

#[test]
fn header_without_long_is_fine() {
    lx_no_colour()
        .args(["--header", "."])
        .assert()
        .success();
}

#[test]
fn level_without_recurse_is_fine() {
    lx_no_colour()
        .args(["--level=3", "."])
        .assert()
        .success();
}
