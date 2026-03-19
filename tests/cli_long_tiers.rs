//! Tests for compounding -l detail tiers.

mod support;

use predicates::prelude::*;
use support::lx_no_colour;


// ── Tier 1: -l (base long view) ──────────────────────────────────

#[test]
fn tier1_has_permissions_size_user_modified() {
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo.toml"));
}

#[test]
fn tier1_no_group_column() {
    // Tier 1 should not show the group unless -g is given.
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").not());
}

#[test]
fn tier1_no_header() {
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions").not());
}


// ── Tier 2: -ll (+ group, VCS status) ────────────────────────────

#[test]
fn tier2_has_group() {
    lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").or(predicate::str::contains("wheel")));
}

#[test]
fn tier2_no_header() {
    // Tier 2 still doesn't have the header row.
    lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions").not());
}


// ── Tier 3: -lll (+ header, all timestamps, links, blocks) ──────

#[test]
fn tier3_has_header() {
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn tier3_has_links_column() {
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Links"));
}

#[test]
fn tier3_has_blocks_column() {
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocks"));
}

#[test]
fn tier3_has_all_timestamp_headers() {
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date Modified"))
        .stdout(predicate::str::contains("Date Changed"))
        .stdout(predicate::str::contains("Date Accessed"))
        .stdout(predicate::str::contains("Date Created"));
}


// ── Overrides on top of tiers ────────────────────────────────────

#[test]
fn tier1_plus_group_flag() {
    // -l -g should show group even at tier 1.
    lx_no_colour()
        .args(["-l", "-g", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staff").or(predicate::str::contains("wheel")));
}

#[test]
fn tier3_with_no_time() {
    // --no-time suppresses all timestamps even at tier 3.
    lx_no_colour()
        .args(["-lll", "--no-time", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date Modified").not());
}

#[test]
fn tier2_plus_header_flag() {
    // -ll -h should show header even though tier 2 doesn't include it.
    lx_no_colour()
        .args(["-ll", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}
