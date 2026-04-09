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
fn columns_unknown_names_error() {
    // Unknown column names are rejected by ColumnsParser, which
    // delegates to clap's PossibleValuesParser for the *first*
    // bad name in the list — so the error focuses on that name
    // (with [possible values: ...] and "did you mean" hints) rather
    // than echoing the whole comma-separated input.
    lx_no_colour()
        .args(["-l", "--columns=perms,bogus,size", "Cargo.toml"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid value 'bogus' for '--columns"))
        .stderr(predicate::str::contains("[possible values:"));
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
        .stdout(predicate::str::contains(support::current_group()));
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


// ── Canonical column insertion order ─────────────────────────────

#[test]
fn blocks_inserts_after_size() {
    // -lS should put blocks right after size, not at the end.
    // Header mode makes column order visible.
    lx_no_colour()
        .args(["-l", "-S", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"Size\s+Blocks\s+User").unwrap());
}

#[test]
fn inode_inserts_before_perms() {
    // -li should put inode first (before permissions).
    lx_no_colour()
        .args(["-l", "-i", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?i)inode\s+Permissions").unwrap());
}

#[test]
fn multiple_adds_canonical_order() {
    // -lSi should give: Inode, Permissions, Size, Blocks, User, Date
    lx_no_colour()
        .args(["-l", "-S", "-i", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?i)inode\s+Permissions").unwrap())
        .stdout(predicate::str::is_match(r"Size\s+Blocks").unwrap());
}

#[test]
fn group_add_between_user_and_timestamp() {
    // -lg should put group after user, before date.
    lx_no_colour()
        .args(["-l", "-g", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"User\s+Group\s+Date").unwrap());
}

#[test]
fn explicit_columns_with_add_inserts_canonically() {
    // --columns in non-canonical order + -S should still put blocks
    // after size (its nearest canonical predecessor).
    lx_no_colour()
        .args(["--columns=modified,user,size", "-S", "-h", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"Size\s+Blocks").unwrap());
}

#[test]
fn no_duplicate_when_already_present() {
    // -llg: group is already in long2, should not duplicate.
    let output = lx_no_colour()
        .args(["-ll", "-g", "-h", "Cargo.toml"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Count occurrences of "Group" in header — should be exactly 1.
    let count = stdout.matches("Group").count();
    assert_eq!(count, 1, "Group should appear once, got {count}: {stdout}");
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
