//! Compiled-in theme registry, theme name lookup, and `--dump-theme`
//! output.
//!
//! Resolution of a theme's UI keys (the chain of `inherits = "..."`
//! parents) lives in `src/theme/mod.rs` because it operates on
//! `UiStyles` rather than the on-disk `ThemeDef`.

use crate::theme::key_registry::{StyleAccess, THEME_KEY_REGISTRY, ThemeFamily, ThemeKeyDef};
use crate::theme::{UiStyles, render_style_to_lx};

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
        return Some(format_builtin_theme_toml(name));
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

    let pairs: Vec<(&str, String)> = theme
        .ui
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    append_grouped_pairs(&mut lines, pairs);

    Some(lines.join("\n"))
}

/// Format a compiled-in theme as TOML by walking the theme key
/// registry and rendering each `Direct` entry's resolved style
/// back to a `parse_style`-compatible string.  Bulk keys are not
/// emitted — they were never set as named fields, only as
/// fan-out shortcuts.
fn format_builtin_theme_toml(name: &str) -> String {
    let ui = UiStyles::compiled(name).expect("is_builtin_theme guards this call");

    let mut lines = vec![format!("[theme.{name}]")];

    let pairs: Vec<(&str, String)> = ThemeKeyDef::dumpable()
        .filter_map(|def| match def.access {
            StyleAccess::Direct { get, .. } => Some((def.name, render_style_to_lx(get(&ui)))),
            StyleAccess::Bulk { .. } => None,
        })
        .collect();

    append_grouped_pairs(&mut lines, pairs);

    lines.join("\n")
}

/// Append `key = "value"` lines to `out`, grouped by registry
/// family with blank-line separators between families.  Within a
/// family, lines come out in registry-declaration order (so date
/// tiers stay in canonical now → today → … → flat order).  Keys
/// not in the registry are placed at the end in alphabetical
/// order, after a final blank-line separator.
fn append_grouped_pairs(out: &mut Vec<String>, pairs: Vec<(&str, String)>) {
    // Index each pair by its (family, registry position) so we can
    // sort with one key.  Unknown keys get a sentinel family of
    // `None` and a registry position past the end.
    let mut indexed: Vec<(Option<ThemeFamily>, usize, &str, String)> = pairs
        .into_iter()
        .map(|(k, v)| {
            let position = THEME_KEY_REGISTRY.iter().position(|d| d.name == k);
            let family = position.map(|i| THEME_KEY_REGISTRY[i].family);
            (family, position.unwrap_or(usize::MAX), k, v)
        })
        .collect();

    // Known keys sort by (family, registry order).  Unknown keys
    // sort alphabetically among themselves (they all share
    // `family = None` and `position = MAX`, so the name is the
    // tiebreaker).
    indexed.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));

    let mut last_family: Option<Option<ThemeFamily>> = None;
    for (family, _pos, key, value) in indexed {
        if last_family.is_some_and(|f| f != family) {
            out.push(String::new());
        }
        out.push(format!("{key} = \"{value}\""));
        last_family = Some(family);
    }
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
