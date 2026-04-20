//! Compiled-in theme registry, theme name lookup, and `--dump-theme`
//! output.
//!
//! Resolution of a theme's UI keys (the chain of `inherits = "..."`
//! parents) lives in `src/theme/mod.rs` because it operates on
//! `UiStyles` rather than the on-disk `ThemeDef`.

use super::error::ConfigError;
use super::store::config;

/// Names of the compiled-in themes that don't live in any config
/// file.  These are always resolvable by `--theme=NAME` and appear
/// in the `--dump-theme` listing.
pub const BUILTIN_THEMES: &[&str] = &["exa", "lx-256", "lx-24bit"];

/// Check if a theme name refers to a compiled-in builtin.
pub fn is_builtin_theme(name: &str) -> bool {
    BUILTIN_THEMES.contains(&name)
}

// ── --dump-theme output ─────────────────────────────────────────

/// Names of all known themes (compiled-in + config).
pub fn all_theme_names() -> Vec<String> {
    let mut names: Vec<String> = BUILTIN_THEMES.iter().map(|s| (*s).to_string()).collect();
    if let Some(cfg) = config() {
        for name in cfg.theme.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names
}

/// Format a theme definition as TOML.
fn format_theme_toml(name: &str) -> Option<String> {
    if is_builtin_theme(name) {
        // Compiled-in themes from default_theme.rs can't be round-tripped
        // to TOML.  Show a helpful comment instead.
        return Some(format!(
            "# [theme.{name}] is compiled-in and cannot be dumped as TOML.\n\
             # To customise, create a new theme that inherits from it:\n\
             #\n\
             # [theme.custom]\n\
             # inherits = \"{name}\"\n\
             # directory = \"bold dodgerblue\"\n\
             # date = \"steelblue\""
        ));
    }

    let cfg = config()?;
    let theme = cfg.theme.get(name)?;
    let mut lines = vec![format!("[theme.{name}]")];

    if let Some(ref inherits) = theme.inherits {
        lines.push(format!("inherits = \"{inherits}\""));
    }
    if let Some(ref use_style) = theme.use_style {
        lines.push(format!("use-style = \"{use_style}\""));
    }

    // Partition UI keys into the `date-*` family and everything else.
    // Alphabetical ordering interleaves bulk `date-*` keys with per-
    // column `date-<col>-*` keys, which destroys the "baseline +
    // overrides" structure a theme author almost always wrote.  We
    // pull the date keys out, sort them by (bulk vs per-column, then
    // tier index), and re-insert them as a contiguous block at the
    // alphabetical position where plain `date` would fall.
    let (mut date_keys, mut other_keys): (Vec<_>, Vec<_>) = theme
        .ui
        .keys()
        .partition(|k| k.as_str() == "date" || k.starts_with("date-"));
    other_keys.sort();
    date_keys.sort_by(|a, b| {
        date_sort_key(a)
            .cmp(&date_sort_key(b))
            .then_with(|| a.cmp(b))
    });

    // Build the date block with blank-line separators between
    // groups (bulk, then each per-column group that has any keys).
    let mut date_block: Vec<String> = Vec::new();
    let mut last_group: Option<u8> = None;
    for k in &date_keys {
        let group = date_sort_key(k).0;
        if last_group.is_some_and(|g| g != group) {
            date_block.push(String::new());
        }
        date_block.push(format!("{k} = \"{}\"", theme.ui[*k]));
        last_group = Some(group);
    }

    // Splice the date block into the alphabetical position where
    // plain `date` would sort among the remaining keys.
    let insert_at = other_keys.partition_point(|k| k.as_str() < "date");
    for k in &other_keys[..insert_at] {
        lines.push(format!("{k} = \"{}\"", theme.ui[*k]));
    }
    if !date_block.is_empty() {
        if insert_at > 0 {
            lines.push(String::new());
        }
        lines.extend(date_block);
        if insert_at < other_keys.len() {
            lines.push(String::new());
        }
    }
    for k in &other_keys[insert_at..] {
        lines.push(format!("{k} = \"{}\"", theme.ui[*k]));
    }

    Some(lines.join("\n"))
}

/// Return a sort key for a date-family theme key so that
/// `--dump-theme` output groups bulk keys (`date`, `date-<tier>`)
/// together first, then each per-column family (`date-<col>-<tier>`)
/// in canonical column order, each group in canonical tier order.
///
/// The returned pair is `(group, tier_index)`:
///
/// - group 0: bulk — `date`, `date-now`, `date-today`, …, `date-flat`
/// - group 1: `date-modified` and `date-modified-<tier>`
/// - group 2: `date-accessed` and `date-accessed-<tier>`
/// - group 3: `date-changed`  and `date-changed-<tier>`
/// - group 4: `date-created`  and `date-created-<tier>`
/// - group 5: any date-prefixed key we don't recognise (stable,
///   alphabetical via the caller's tiebreaker)
///
/// Tier index 0 is reserved for the "whole column" setter (`date`
/// itself or `date-<col>` without a tier suffix), so it always
/// sorts first within its group.
///
/// Non-date keys return `(u8::MAX, 0)`; the caller is expected not
/// to pass them.
fn date_sort_key(key: &str) -> (u8, u8) {
    const TIERS: [&str; 8] = ["", "now", "today", "week", "month", "year", "old", "flat"];
    const COLS: [&str; 4] = ["modified", "accessed", "changed", "created"];

    if key == "date" {
        return (0, 0);
    }

    let Some(rest) = key.strip_prefix("date-") else {
        return (u8::MAX, 0);
    };

    // Bulk tier: date-now, date-today, …, date-flat
    if let Some(ti) = TIERS.iter().position(|t| *t == rest) {
        return (0, ti as u8);
    }

    // Per-column whole: date-modified, date-accessed, …
    if let Some(ci) = COLS.iter().position(|c| *c == rest) {
        return ((1 + ci) as u8, 0);
    }

    // Per-column tier: date-modified-now, date-accessed-week, …
    for (ci, col) in COLS.iter().enumerate() {
        if let Some(tier) = rest.strip_prefix(col).and_then(|r| r.strip_prefix('-'))
            && let Some(ti) = TIERS.iter().position(|t| *t == tier)
        {
            return ((1 + ci) as u8, ti as u8);
        }
    }

    (5, 0)
}

/// Print a single theme definition as copy-pasteable TOML.
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if `name` does not match any
/// built-in or user-defined theme.
pub fn dump_theme(name: &str) -> Result<(), ConfigError> {
    if let Some(toml) = format_theme_toml(name) {
        println!("{toml}");
        Ok(())
    } else {
        Err(ConfigError::NotFound {
            kind: "theme",
            kind_plural: "themes",
            name: name.to_string(),
            candidates: all_theme_names().join(", "),
        })
    }
}

/// Print all theme definitions as copy-pasteable TOML.
pub fn dump_theme_all() {
    let names = all_theme_names();
    let mut first = true;
    for name in &names {
        if let Some(toml) = format_theme_toml(name) {
            if !first {
                println!();
            }
            println!("{toml}");
            first = false;
        }
    }
}
