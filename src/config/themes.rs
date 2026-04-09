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
fn all_theme_names() -> Vec<String> {
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

    let mut keys: Vec<_> = theme.ui.keys().collect();
    keys.sort();
    for key in keys {
        lines.push(format!("{key} = \"{}\"", theme.ui[key]));
    }

    Some(lines.join("\n"))
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
            if !first { println!(); }
            println!("{toml}");
            first = false;
        }
    }
}
