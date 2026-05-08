//! Named column-format definitions: `[format]` resolution and the
//! `--show-format` output.

use std::collections::HashMap;

use super::error::ConfigError;
use super::store::config;

/// Hidden aliases for compiled-in format names.
///
/// These map old format names (carried in pre-0.11 configs and
/// muscle-memory `--format=...` invocations) to the canonical names
/// that replace them.  They resolve to the same column list as their
/// canonical name and are accepted on the CLI and in config, but they
/// are not advertised in `--help` or enumerated by `show_format_all`.
///
/// `--upgrade-config` rewrites these to the canonical names so that
/// new config files don't carry the deprecated spellings forward.
const HIDDEN_ALIASES: &[(&str, &str)] = &[
    // 0.11 tier redesign: `long2` and `long3` became aliases.
    ("long2", "long"),
    ("long3", "full"),
];

/// Returns true if `name` is a hidden alias for a canonical format.
fn is_hidden_alias(name: &str) -> bool {
    HIDDEN_ALIASES.iter().any(|&(alias, _)| alias == name)
}

/// The compiled-in format definitions as column name strings.
///
/// Returns canonical names only; hidden aliases (see
/// `HIDDEN_ALIASES`) are added by `resolve_formats` so that they
/// resolve to the same column list as their canonical name.
fn compiled_formats() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "long".into(),
            vec![
                "permissions".into(),
                "size".into(),
                "user".into(),
                "modified".into(),
                "vcs".into(),
            ],
        ),
        (
            "full".into(),
            vec![
                "permissions".into(),
                "links".into(),
                "size".into(),
                "blocks".into(),
                "user".into(),
                "group".into(),
                "modified".into(),
                "changed".into(),
                "created".into(),
                "accessed".into(),
                "vcs".into(),
            ],
        ),
    ])
}

/// Resolve all format definitions: compiled-in canonical names,
/// hidden aliases pointing at the same column lists, and config
/// overrides (which take precedence over both).
///
/// Returns a map of format name → list of column name strings.
pub fn resolve_formats() -> HashMap<String, Vec<String>> {
    let mut formats = compiled_formats();

    // Hidden aliases: each alias resolves to the same column list as
    // its canonical target.  Inserted here (not in `compiled_formats`)
    // so the alias mapping has one point of truth.
    for &(alias, canonical) in HIDDEN_ALIASES {
        if let Some(columns) = formats.get(canonical) {
            formats.insert(alias.into(), columns.clone());
        }
    }

    // Config overrides (canonical or alias name — user wins either way).
    if let Some(cfg) = config() {
        for (name, columns) in &cfg.format {
            formats.insert(name.clone(), columns.clone());
        }
    }

    formats
}

// ── --show-format output ────────────────────────────────────────

/// Print a single format definition as copy-pasteable TOML.
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if `name` does not match any
/// compiled-in or user-defined column format.
pub fn show_format(name: &str) -> Result<(), ConfigError> {
    let formats = resolve_formats();
    if let Some(columns) = formats.get(name) {
        println!("[format]");
        println!("{}", format_format_toml(name, columns));
        Ok(())
    } else {
        let mut names: Vec<_> = formats.keys().collect();
        names.sort();
        let candidates = names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Err(ConfigError::NotFound {
            kind: "format",
            kind_plural: "formats",
            name: name.to_string(),
            candidates,
        })
    }
}

/// Print all format definitions as copy-pasteable TOML.
///
/// Hidden aliases are skipped — they resolve to the same column list
/// as their canonical name, so listing both would produce a
/// confusing duplicate entry.
pub fn show_format_all() {
    let formats = resolve_formats();
    let mut names: Vec<_> = formats.keys().filter(|n| !is_hidden_alias(n)).collect();
    names.sort();

    println!("[format]");
    for name in names {
        println!("{}", format_format_toml(name, &formats[name]));
    }
}

/// Format a single format definition as TOML.
fn format_format_toml(name: &str, columns: &[String]) -> String {
    let entries: Vec<String> = columns.iter().map(|c| format!("\"{c}\"")).collect();
    let body = entries.join(", ");
    let line = format!("{name} = [{body}]");
    if line.len() <= 72 {
        line
    } else {
        // Wrap like class definitions.
        let indent = " ".repeat(name.len() + 4);
        let mut lines = vec![format!("{name} = [")];
        for (i, entry) in entries.iter().enumerate() {
            let last = lines.last_mut().unwrap();
            if i == 0 {
                last.push_str(entry);
            } else {
                let trial_len = last.len() + 2 + entry.len();
                if trial_len > 72 {
                    last.push(',');
                    lines.push(format!("{indent}{entry}"));
                } else {
                    last.push_str(", ");
                    last.push_str(entry);
                }
            }
        }
        lines.last_mut().unwrap().push(']');
        lines.join("\n")
    }
}
