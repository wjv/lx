//! Tests for timestamp display and time-style options.

mod support;

use std::fs;
use predicates::prelude::*;
use support::lx_no_colour;
use tempfile::tempdir;


#[test]
fn time_style_default() {
    lx_no_colour()
        .args(["-l", "--time-style=default", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_style_iso() {
    lx_no_colour()
        .args(["-l", "--time-style=iso", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_style_long_iso() {
    // long-iso includes a full date like "2026-03-18 15:07"
    lx_no_colour()
        .args(["-l", "--time-style=long-iso", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}

#[test]
fn time_style_full_iso() {
    // full-iso includes seconds and timezone offset
    lx_no_colour()
        .args(["-l", "--time-style=full-iso", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap());
}

#[test]
fn time_style_from_env() {
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .env("TIME_STYLE", "long-iso")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}

#[test]
fn time_style_flag_overrides_env() {
    lx_no_colour()
        .args(["-l", "--time-style=iso", "Cargo.toml"])
        .env("TIME_STYLE", "full-iso")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}


// ── Timestamp field selection ─────────────────────────────────────

#[test]
fn time_modified_flag() {
    lx_no_colour()
        .args(["-lm", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_accessed_flag() {
    lx_no_colour()
        .args(["-lu", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_created_flag() {
    lx_no_colour()
        .args(["-lU", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_param_modified() {
    lx_no_colour()
        .args(["-l", "--time=modified", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_param_accessed() {
    lx_no_colour()
        .args(["-l", "--time=accessed", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn time_param_created() {
    lx_no_colour()
        .args(["-l", "--time=created", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn no_time_suppresses_date() {
    let dir = tempdir().expect("failed to create tempdir");
    fs::write(dir.path().join("file.txt"), "content").unwrap();

    // With --no-time, the date column should not appear.
    // We check that a year-like pattern is absent.
    lx_no_colour()
        .args(["-l", "--no-time", "--time-style=long-iso"])
        .arg(dir.path().join("file.txt"))
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2}").unwrap().not());
}

#[test]
fn multiple_time_flags_combine() {
    // -m and -u together should show both modified and accessed
    lx_no_colour()
        .args(["-lmu", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}
