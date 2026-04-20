//! Named column-format definitions: `[format]` resolution and the
//! `--show-format` output.

use std::collections::HashMap;

use super::error::ConfigError;
use super::store::config;

/// The compiled-in format definitions as column name strings.
fn compiled_formats() -> HashMap<String, Vec<String>> {
    HashMap::from([
        (
            "long".into(),
            vec![
                "permissions".into(),
                "size".into(),
                "user".into(),
                "modified".into(),
            ],
        ),
        (
            "long2".into(),
            vec![
                "permissions".into(),
                "size".into(),
                "user".into(),
                "group".into(),
                "modified".into(),
                "vcs".into(),
            ],
        ),
        (
            "long3".into(),
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

/// Resolve all format definitions: compiled-in + config overrides.
/// Returns a map of format name → list of column name strings.
pub fn resolve_formats() -> HashMap<String, Vec<String>> {
    let mut formats = compiled_formats();

    // Config overrides.
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
pub fn show_format_all() {
    let formats = resolve_formats();
    let mut names: Vec<_> = formats.keys().collect();
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
