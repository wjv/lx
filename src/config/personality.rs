//! Personality resolution: walking the inheritance chain, applying
//! `[[when]]` overrides, and the `--dump-personality` /
//! `--save-as=NAME` output paths.

use std::collections::HashMap;
use std::path::PathBuf;

use log::*;

use super::error::{ConfigError, IoResultExt};
use super::load::{find_config_path, find_drop_in_dir};
use super::schema::{ConditionalOverride, PersonalityDef};
use super::store::config;


/// Look up a personality by name, resolving inheritance.
///
/// Config-defined personalities take priority over compiled-in ones.
/// If the personality declares `inherits = "NAME"`, the chain is
/// walked to the root and settings are merged (child overrides parent
/// per-key; `format`/`columns` replace entirely).
///
/// Returns `Ok(Some(...))` on success, `Ok(None)` if the personality
/// doesn't exist, or `Err(ConfigError)` on config errors (e.g. cycles).
pub fn resolve_personality(name: &str) -> Result<Option<PersonalityDef>, ConfigError> {
    // Build the inheritance chain: [leaf, ..., root].
    let mut chain: Vec<PersonalityDef> = Vec::new();
    let mut visited: Vec<String> = Vec::new();
    let mut current = Some(name.to_string());

    while let Some(ref pname) = current {
        // Cycle detection.
        if visited.contains(pname) {
            visited.push(pname.clone());
            let chain_str = visited.join(" \u{2192} ");
            return Err(ConfigError::InheritanceCycle { chain: chain_str });
        }
        visited.push(pname.clone());

        // Look up: config first, then compiled-in.
        let Some(def) = lookup_personality(pname) else {
            if chain.is_empty() {
                return Ok(None);  // top-level personality not found
            }
            return Err(ConfigError::MissingParent {
                child: visited[visited.len() - 2].clone(),
                parent: pname.clone(),
            });
        };
        let next = def.inherits.clone();
        chain.push(def);
        current = next;
    }

    // Merge from root (last) to leaf (first).
    let mut effective = PersonalityDef::default();
    for def in chain.into_iter().rev() {
        if def.format.is_some() {
            effective.format = def.format;
        }
        if def.columns.is_some() {
            effective.columns = def.columns;
        }
        // Settings: child values override parent values.
        for (key, value) in def.settings {
            effective.settings.insert(key, value);
        }
        // Collect all when blocks (parent first, child later).
        effective.when.extend(def.when);
    }

    // Apply matching conditional overrides (in order; later wins).
    for cond in &effective.when {
        if cond.matches() {
            debug!("conditional override matched: env = {:?}", cond.env);
            for (key, value) in &cond.settings {
                effective.settings.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(Some(effective))
}

/// Look up a single personality definition by name (no inheritance).
fn lookup_personality(name: &str) -> Option<PersonalityDef> {
    if let Some(cfg) = config()
        && let Some(p) = cfg.personality.get(name) {
            return Some(p.clone());
        }
    compiled_personality(name)
}

/// Return a compiled-in personality definition, if one exists.
///
/// These match the uncommented sections in `lxconfig.default.toml`,
/// so the tool behaves the same with or without a config file.
fn compiled_personality(name: &str) -> Option<PersonalityDef> {
    use toml::Value::{Boolean, String as Str};

    match name {
        // The "default" personality sets theme = "exa" so that
        // file-type colouring is explicit, not a magic fallback.
        // Conditional [[when]] blocks auto-select richer themes
        // when the terminal supports them: lx-256 on 256-colour
        // terminals, lx-24bit on truecolour terminals.  The
        // truecolour block comes second so it wins on terminals
        // that satisfy both conditions (e.g. xterm-256color +
        // COLORTERM=truecolor).
        "default" => Some(PersonalityDef {
            settings: HashMap::from([
                ("theme".into(), toml::Value::String("exa".into())),
            ]),
            when: vec![
                ConditionalOverride {
                    env: HashMap::from([
                        ("TERM".into(), toml::Value::String("*-256color".into())),
                    ]),
                    settings: HashMap::from([
                        ("theme".into(), toml::Value::String("lx-256".into())),
                    ]),
                },
                ConditionalOverride {
                    env: HashMap::from([
                        ("COLORTERM".into(), toml::Value::Array(vec![
                            toml::Value::String("truecolor".into()),
                            toml::Value::String("24bit".into()),
                        ])),
                    ]),
                    settings: HashMap::from([
                        ("theme".into(), toml::Value::String("lx-24bit".into())),
                    ]),
                },
            ],
            ..Default::default()
        }),
        "lx" => Some(PersonalityDef {
            inherits: Some("default".into()),
            ..Default::default()
        }),
        "ll" => Some(PersonalityDef {
            inherits: Some("lx".into()),
            format: Some("long2".into()),
            settings: HashMap::from([
                ("group-dirs".into(), Str("first".into())),
            ]),
            ..Default::default()
        }),
        "lll" => Some(PersonalityDef {
            inherits: Some("lx".into()),
            format: Some("long3".into()),
            settings: HashMap::from([
                ("group-dirs".into(), Str("first".into())),
                ("header".into(), Boolean(true)),
                ("time-style".into(), Str("long-iso".into())),
            ]),
            ..Default::default()
        }),
        "la" => Some(PersonalityDef {
            inherits: Some("ll".into()),
            settings: HashMap::from([
                ("all".into(), Boolean(true)),
            ]),
            ..Default::default()
        }),
        "tree" => Some(PersonalityDef {
            inherits: Some("default".into()),
            format: Some("long2".into()),
            settings: HashMap::from([
                ("tree".into(), Boolean(true)),
                ("group-dirs".into(), Str("first".into())),
            ]),
            ..Default::default()
        }),
        "ls" => Some(PersonalityDef {
            settings: HashMap::from([
                ("grid".into(), Boolean(true)),
                ("across".into(), Boolean(true)),
            ]),
            ..Default::default()
        }),
        _ => None,
    }
}


// ── --dump-personality output ───────────────────────────────────

/// Names of all compiled-in personalities.
const COMPILED_PERSONALITIES: &[&str] = &[
    "default", "lx", "ll", "lll", "la", "tree", "ls",
];

/// Return the names of all known personalities (compiled-in + config).
pub fn all_personality_names() -> Vec<String> {
    let mut names: Vec<String> = COMPILED_PERSONALITIES.iter()
        .map(|s| (*s).into())
        .collect();
    if let Some(cfg) = config() {
        for name in cfg.personality.keys() {
            if !names.iter().any(|n| n == name) {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names
}

/// Format a personality definition as TOML.
fn format_personality_toml(name: &str) -> Option<String> {
    // Look up the *unresolved* definition (without inheritance merging)
    // so the TOML output matches what you'd write in a config file.
    let def = lookup_personality(name)?;
    let mut lines = vec![format!("[personality.{name}]")];

    if let Some(ref inherits) = def.inherits {
        lines.push(format!("inherits = \"{inherits}\""));
    }
    if let Some(ref format) = def.format {
        lines.push(format!("format = \"{format}\""));
    }
    if let Some(ref columns) = def.columns {
        let entries: Vec<String> = columns.to_csv()
            .split(',')
            .map(|s| format!("\"{}\"", s.trim()))
            .collect();
        lines.push(format!("columns = [{}]", entries.join(", ")));
    }

    // Sort settings for stable output.
    let mut keys: Vec<_> = def.settings.keys().collect();
    keys.sort();
    for key in keys {
        let value = &def.settings[key];
        match value {
            toml::Value::String(s) => lines.push(format!("{key} = \"{s}\"")),
            toml::Value::Boolean(b) => lines.push(format!("{key} = {b}")),
            toml::Value::Integer(i) => lines.push(format!("{key} = {i}")),
            toml::Value::Float(f) => lines.push(format!("{key} = {f}")),
            _ => lines.push(format!("{key} = {value}")),
        }
    }

    Some(lines.join("\n"))
}

/// Print a single personality definition as copy-pasteable TOML.
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if `name` does not match any
/// compiled-in or user-defined personality.
pub fn dump_personality(name: &str) -> Result<(), ConfigError> {
    if let Some(toml) = format_personality_toml(name) {
        println!("{toml}");
        Ok(())
    } else {
        Err(ConfigError::NotFound {
            kind: "personality",
            kind_plural: "personalities",
            name: name.to_string(),
            candidates: all_personality_names().join(", "),
        })
    }
}

/// Print all personality definitions as copy-pasteable TOML.
pub fn dump_personality_all() {
    let names = all_personality_names();
    let mut first = true;
    for name in &names {
        if let Some(toml) = format_personality_toml(name) {
            if !first { println!(); }
            println!("{toml}");
            first = false;
        }
    }
}


// ── --save-as=NAME ──────────────────────────────────────────────

/// Save a personality definition to `conf.d/NAME.toml`.
///
/// Builds a TOML `[personality.NAME]` section from the given settings
/// (the CLI flag delta), with an optional `inherits` directive.
/// Creates the `conf.d/` directory if it doesn't exist.  Backs up
/// any existing file to `NAME.toml.bak`.
///
/// # Errors
///
/// Returns `ConfigError::Io` if the `conf.d/` directory cannot be
/// created, the existing file cannot be backed up, or the new file
/// cannot be written.
pub fn save_personality_as(
    name: &str,
    inherits: Option<&str>,
    settings: &HashMap<String, toml::Value>,
) -> Result<(), ConfigError> {
    use chrono::Local;

    // Build TOML lines.
    let mut lines = vec![
        format!("# Generated by lx --save-as on {}", Local::now().format("%Y-%m-%d")),
        String::new(),
        format!("[personality.{name}]"),
    ];

    if let Some(parent) = inherits {
        lines.push(format!("inherits = \"{parent}\""));
    }

    // Sort keys for stable output.
    let mut keys: Vec<_> = settings.keys().collect();
    keys.sort();
    for key in keys {
        let value = &settings[key];
        match value {
            toml::Value::String(s) => lines.push(format!("{key} = \"{s}\"")),
            toml::Value::Boolean(b) => lines.push(format!("{key} = {b}")),
            toml::Value::Integer(i) => lines.push(format!("{key} = {i}")),
            toml::Value::Float(f) => lines.push(format!("{key} = {f}")),
            _ => lines.push(format!("{key} = {value}")),
        }
    }
    lines.push(String::new()); // trailing newline

    let toml_content = lines.join("\n");

    // Find or create the conf.d/ directory.
    let conf_dir = find_drop_in_dir(find_config_path().as_deref())
        .unwrap_or_else(|| {
            // No config file exists; use the XDG default location.
            let xdg = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").expect("HOME not set");
                    PathBuf::from(home).join(".config")
                });
            xdg.join("lx").join("conf.d")
        });

    std::fs::create_dir_all(&conf_dir).with_path(&conf_dir)?;

    let file_path = conf_dir.join(format!("{name}.toml"));

    // Back up any existing file.
    if file_path.exists() {
        let backup = file_path.with_extension("toml.bak");
        std::fs::rename(&file_path, &backup).with_path(&file_path)?;
        eprintln!("lx: backed up {} → {}", file_path.display(), backup.display());
    }

    std::fs::write(&file_path, &toml_content).with_path(&file_path)?;

    eprintln!("lx: saved personality '{name}' to {}", file_path.display());
    Ok(())
}
