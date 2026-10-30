//! Tests for the compounding -l detail tiers.
//!
//! Post-0.11 semantics (the tier redesign; see "Upgrading to 0.11"
//! in `docs/UPGRADING.md` for context):
//!
//! - `-l`/`-ll`/`-lll` route through compiled-in personalities
//!   `l`/`ll`/`lll`.  `lx -ll`, `lx -p ll`, and the `ll` argv[0]
//!   symlink all activate `[personality.ll]` — one mechanism, three
//!   invocation routes.
//! - **Tier 1 columns**: perms, size, user, modified, **vcs**.
//! - **Tier 2 columns**: same as tier 1.  Differs presentationally:
//!   header row, ISO-style timestamps, count footer.  No new data.
//! - **Tier 3 columns**: full forensic view (perms, links, size,
//!   blocks, user, group, modified, changed, created, accessed, vcs).
//!   Header + ISO timestamps.

mod support;

use predicates::prelude::*;
use support::lx_no_colour;

// ── Tier 1: -l (base long view) ──────────────────────────────────

#[test]
fn tier1_lists_the_file() {
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
        .stdout(predicate::str::contains(support::current_group()).not());
}

#[test]
fn tier1_no_header() {
    lx_no_colour()
        .args(["-l", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions").not());
}

#[test]
fn tier1_has_vcs_column_when_in_repo() {
    // The 0.11 redesign promotes `vcs` to the tier-1 column set —
    // it's the column that genuinely differentiates `lx` from `ls`.
    // We use `-h` to surface column headers; the VCS-column header
    // is "Git" or "JJ" depending on the active backend.  Either is
    // acceptable.
    let output = lx_no_colour()
        .args(["-l", "-h", "Cargo.toml"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Git") || stdout.contains("JJ"),
        "expected VCS column header (Git or JJ) in tier-1 output: {stdout}"
    );
}

// ── Tier 2: -ll (polish on top of tier 1) ───────────────────────

#[test]
fn tier2_no_group_column() {
    // Tier 2 has the SAME columns as tier 1.  Group does not appear
    // in `-ll` output unless explicitly added with `-g` or by an
    // overridden personality.
    lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(support::current_group()).not());
}

#[test]
fn tier2_has_header() {
    // Tier 2's `[personality.ll]` declares `header = true`, so the
    // header row appears without an explicit `-h`.
    lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn tier2_has_long_iso_timestamp() {
    // `[personality.ll]` declares `time-style = "long-iso"`, so the
    // modified column renders as `YYYY-MM-DD HH:MM`.
    lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap());
}

#[test]
fn tier1_and_tier2_have_same_column_set() {
    // The tier redesign's load-bearing simplification: tier 2 adds
    // *presentation polish* on top of tier 1, not new data.  The
    // visible column-name set should match between the two.
    //
    // We compare by extracting which column-header strings appear in
    // each output (using -h on tier 1 to surface its headers, since
    // tier 2 always shows them via personality polish).
    let column_names = [
        "Permissions",
        "Size",
        "User",
        "Date Modified",
        "Group",
        "Links",
        "Blocks",
        "Date Changed",
        "Date Created",
        "Date Accessed",
    ];
    let columns_in = |args: &[&str]| -> std::collections::BTreeSet<String> {
        let output = lx_no_colour()
            .args(args)
            .output()
            .expect("failed to run lx");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        column_names
            .iter()
            .filter(|name| stdout.contains(*name))
            .map(|s| (*s).to_string())
            .collect()
    };
    let t1 = columns_in(&["-l", "-h", "Cargo.toml"]);
    let t2 = columns_in(&["-ll", "Cargo.toml"]);
    assert_eq!(
        t1, t2,
        "tier 1 and tier 2 should have identical column sets"
    );
}

// ── Tier 3: -lll (full forensic view) ───────────────────────────

#[test]
fn tier3_has_header() {
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions"));
}

#[test]
fn tier3_has_group_column() {
    // Tier 3 keeps `group` in its forensic column set — the column
    // earns its width here, where the tier's purpose is "show me
    // everything".
    lx_no_colour()
        .args(["-lll", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(support::current_group()));
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

// ── Tier-flag / personality / argv[0] convergence (0.11 redesign) ─

#[test]
fn tier_flag_and_p_flag_produce_identical_output() {
    // The load-bearing invariant of the 0.11 tier redesign:
    // `lx -ll <path>` and `lx -p ll <path>` must produce
    // byte-identical output.  Both activate `[personality.ll]`.
    let via_tier = lx_no_colour()
        .args(["-ll", "Cargo.toml"])
        .output()
        .expect("failed to run -ll");
    let via_p = lx_no_colour()
        .args(["-p", "ll", "Cargo.toml"])
        .output()
        .expect("failed to run -p ll");
    assert_eq!(
        String::from_utf8_lossy(&via_tier.stdout),
        String::from_utf8_lossy(&via_p.stdout),
        "lx -ll and lx -p ll must produce identical output"
    );
}

// ── Overrides on top of tiers ────────────────────────────────────

#[test]
fn tier1_plus_group_flag() {
    // -l -g should show group even at tier 1.
    lx_no_colour()
        .args(["-l", "-g", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(support::current_group()));
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
fn tier2_no_header_flag_suppresses() {
    // Tier 2 declares `header = true` via its personality, but
    // `--no-header` should still suppress it (CLI flags override
    // personality settings).
    lx_no_colour()
        .args(["-ll", "--no-header", "Cargo.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Permissions").not());
}

#[test]
fn letter_l_in_a_flag_value_is_not_a_tier_flag() {
    // `-Ilocal` is `-I` with the glob `local`.  Its two `l`s used
    // to be counted as `-ll`, activating the tier-2 personality.
    lx_no_colour()
        .args(["-Ilocal", "--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Personality: lx"));
}

#[test]
fn personality_flag_inside_a_cluster_is_honoured() {
    // `-ap tree` is `-a` plus `-p tree`.  The personality used to be
    // ignored because `-p` wasn't at the start of the argument.
    lx_no_colour()
        .args(["-ap", "tree", "--show-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Personality: tree"));
}
