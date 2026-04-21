//! File-colour style sets: `[style.NAME]` resolution and the
//! `--dump-style` output.

use std::collections::HashMap;

use super::error::ConfigError;
use super::schema::StyleDef;
use super::store::config;

/// Return the compiled-in "exa" style definition.
///
/// This maps the built-in file-type classes to their default colours,
/// matching the hard-coded values in `src/info/filetype.rs`.
pub fn compiled_exa_style() -> StyleDef {
    StyleDef {
        classes: HashMap::from([
            ("temp".into(), "38;5;244".into()),
            ("immediate".into(), "bold underline yellow".into()),
            ("image".into(), "38;5;133".into()),
            ("video".into(), "38;5;135".into()),
            ("music".into(), "38;5;92".into()),
            ("lossless".into(), "38;5;93".into()),
            ("crypto".into(), "38;5;109".into()),
            ("document".into(), "38;5;105".into()),
            ("compressed".into(), "red".into()),
            ("compiled".into(), "38;5;137".into()),
        ]),
        patterns: HashMap::new(),
    }
}

/// Look up a style by name: config first, then compiled-in "exa".
pub fn resolve_style(name: &str) -> Option<StyleDef> {
    if let Some(cfg) = config()
        && let Some(s) = cfg.style.get(name)
    {
        return Some(s.clone());
    }
    match name {
        "exa" => Some(compiled_exa_style()),
        _ => None,
    }
}

// ── --dump-style output ─────────────────────────────────────────

/// Names of all known styles (compiled-in + config).
pub fn all_style_names() -> Vec<String> {
    let mut names = vec!["exa".to_string()];
    if let Some(cfg) = config() {
        for name in cfg.style.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names
}

/// Format a style definition as TOML.
fn format_style_toml(name: &str) -> Option<String> {
    let style = resolve_style(name)?;
    let mut lines = vec![format!("[style.{name}]")];

    // Class references.
    let mut keys: Vec<_> = style.classes.keys().collect();
    keys.sort();
    for key in keys {
        lines.push(format!("class.{key} = \"{}\"", style.classes[key]));
    }

    // File patterns.
    let mut keys: Vec<_> = style.patterns.keys().collect();
    keys.sort();
    for key in keys {
        lines.push(format!("\"{key}\" = \"{}\"", style.patterns[key]));
    }

    Some(lines.join("\n"))
}

/// Print a single style definition as copy-pasteable TOML.
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if `name` does not match any
/// built-in or user-defined style.
pub fn dump_style(name: &str) -> Result<(), ConfigError> {
    if let Some(toml) = format_style_toml(name) {
        println!("{toml}");
        Ok(())
    } else {
        Err(ConfigError::NotFound {
            kind: "style",
            kind_plural: "styles",
            name: name.to_string(),
            candidates: all_style_names().join(", "),
        })
    }
}

/// Print all style definitions as copy-pasteable TOML.
pub fn dump_style_all() {
    let names = all_style_names();
    let mut first = true;
    for name in &names {
        if let Some(toml) = format_style_toml(name) {
            if !first {
                println!();
            }
            println!("{toml}");
            first = false;
        }
    }
}
