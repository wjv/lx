//! Tests for --columns, --format, and their interaction with -l tiers.

mod support;

use predicates::prelude::*;
use support::lx_no_colour;


// ── --columns flag ───────────────────────────────────────────────

#[test]
fn columns_explicit_set() {
    lx_no_colour()
        .args(["-l", "--columns=perms,size,modified", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn columns_with_inode() {
    lx_no_colour()
        .args(["-l", "--columns=inode,perms,size", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn columns_overrides_tier() {
    // --columns should override -lll tier columns — no group
    // (but header remains because -lll sets header independently)
    lx_no_colour()
        .args(["-lll", "--columns=perms,size,modified", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Group").not())
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn columns_suppression_still_works() {
    // --no-permissions should remove perms even from explicit --columns
    lx_no_colour()
        .args(["-l", "--columns=perms,size,modified", "--no-permissions", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn columns_unknown_names_ignored() {
    // Unknown column names should be silently ignored
    lx_no_colour()
        .args(["-l", "--columns=perms,bogus,size", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}


// ── --format flag ────────────────────────────────────────────────

#[test]
fn format_long() {
    lx_no_colour()
        .args(["-l", "--format=long", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn format_long2_has_group() {
    lx_no_colour()
        .args(["-l", "--format=long2", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").or(predicate::str::contains("wheel")));
}

#[test]
fn format_long3_has_multiple_timestamps() {
    // long3 has four timestamp columns — check for "Date Changed"
    // (which only appears in long3, not long or long2)
    lx_no_colour()
        .args(["-l", "--format=long3", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date Changed"));
}

#[test]
fn format_overrides_tier() {
    // --format=long should override -lll
    lx_no_colour()
        .args(["-lll", "--format=long", "Cargo.toml"])
        .assert()
        .success()
        // long format has no group column
        .stdout(predicate::str::contains("staff").not());
}

#[test]
fn format_invalid_rejected() {
    lx_no_colour()
        .args(["-l", "--format=bogus", "Cargo.toml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn format_with_individual_add() {
    // --format=long plus -i should add inode
    lx_no_colour()
        .args(["-l", "--format=long", "-i", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inode"));
}

#[test]
fn format_with_suppression() {
    // --format=long2 with --no-group should hide group
    lx_no_colour()
        .args(["-l", "--format=long2", "--no-group", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").not());
}


// ── --columns overrides --format ─────────────────────────────────

#[test]
fn columns_overrides_format() {
    // --columns should override --format
    lx_no_colour()
        .args(["-l", "--format=long3", "--columns=perms,size", "Cargo.toml"])
        .assert()
        .success()
        // No group (not in --columns), despite long3
        .stdout(predicate::str::contains("staff").not());
}
