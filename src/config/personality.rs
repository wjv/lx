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

// ── Inheritance chain metadata ─────────────────────────────────

/// One link in the personality inheritance chain.
#[derive(Debug, Clone)]
pub struct ChainLink {
    /// Personality name at this level.
    pub name: String,
    /// Where the definition came from.
    pub source: PersonalitySource,
    /// The personality definition declared at *this* level (before
    /// merging with parents/children).  Lets diagnostic surfaces
    /// show each link's direct contributions — its own
    /// `format`/`columns`/`settings`, plus the `[[when]]` blocks.
    pub def: PersonalityDef,
    /// When `source == ConfigOverridesBuiltin`, the compiled-in
    /// definition that the user's config shadows.  Used by
    /// `--show-config` to surface the silent-shadowing case (keys
    /// and `[[when]]` blocks present in the builtin but absent
    /// from the override).  `None` for `Builtin` and pure
    /// `Config` links.
    pub shadowed_builtin: Option<PersonalityDef>,
}

/// Where a personality definition comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonalitySource {
    /// Compiled into the lx binary.
    Builtin,
    /// Defined in the user's config file (or drop-in).
    Config,
    /// Config-defined, but a compiled-in personality of the same
    /// name also exists.
    ConfigOverridesBuiltin,
}

/// The resolved personality plus metadata about the resolution
/// process.  Used by `--show-config` to display the inheritance
/// chain and `[[when]]` match status.
#[derive(Debug, Clone)]
pub struct ResolvedPersonality {
    /// The merged personality definition (settings, format, etc.).
    pub def: PersonalityDef,
    /// Inheritance chain, leaf first (the requested personality)
    /// to root (the final ancestor with no `inherits`).
    pub chain: Vec<ChainLink>,
}

// ── Resolution ─────────────────────────────────────────────────

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
    Ok(resolve_personality_full(name)?.map(|r| r.def))
}

/// Like `resolve_personality`, but also returns the inheritance
/// chain with source and `[[when]]` match metadata.
pub fn resolve_personality_full(name: &str) -> Result<Option<ResolvedPersonality>, ConfigError> {
    // Build the inheritance chain: [leaf, ..., root].
    let mut defs: Vec<(String, PersonalityDef)> = Vec::new();
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
            if defs.is_empty() {
                return Ok(None); // top-level personality not found
            }
            return Err(ConfigError::MissingParent {
                child: visited[visited.len() - 2].clone(),
                parent: pname.clone(),
            });
        };
        let next = def.inherits.clone();
        defs.push((pname.clone(), def));
        current = next;
    }

    // Build chain metadata before merging consumes the defs.
    let cfg = config();
    let chain: Vec<ChainLink> = defs
        .iter()
        .map(|(pname, def)| {
            let in_config = cfg.is_some_and(|c| c.personality.contains_key(pname));
            let in_builtin = compiled_personality(pname).is_some();
            let source = match (in_config, in_builtin) {
                (true, true) => PersonalitySource::ConfigOverridesBuiltin,
                (true, false) => PersonalitySource::Config,
                _ => PersonalitySource::Builtin,
            };
            let shadowed_builtin = if source == PersonalitySource::ConfigOverridesBuiltin {
                compiled_personality(pname)
            } else {
                None
            };
            ChainLink {
                name: pname.clone(),
                source,
                def: def.clone(),
                shadowed_builtin,
            }
        })
        .collect();

    // Merge from root (last) to leaf (first).
    let mut effective = PersonalityDef::default();
    for (_name, def) in defs.into_iter().rev() {
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
            debug!(
                "conditional override matched: env = {:?}, platform = {:?}",
                cond.env, cond.platform
            );
            for (key, value) in &cond.settings {
                effective.settings.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(Some(ResolvedPersonality {
        def: effective,
        chain,
    }))
}

/// Look up a single personality definition by name (no inheritance).
fn lookup_personality(name: &str) -> Option<PersonalityDef> {
    if let Some(cfg) = config()
        && let Some(p) = cfg.personality.get(name)
    {
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
            description: Some(
                "Shared base; auto-selects a richer theme on capable terminals".into(),
            ),
            settings: HashMap::from([
                ("gradient".into(), Str("all".into())),
                ("group-dirs".into(), Str("none".into())),
                ("icons".into(), Str("never".into())),
                ("classify".into(), Str("never".into())),
                ("theme".into(), Str("exa".into())),
                // Default-on for the `@` indicator probe; flipped
                // off on macOS in the [[when]] block below because
                // listxattr there is disproportionately expensive
                // (see CHANGELOG for the 0.10 perf details).
                ("xattr-indicator".into(), Boolean(true)),
            ]),
            when: vec![
                ConditionalOverride {
                    env: HashMap::from([("TERM".into(), toml::Value::String("*-256color".into()))]),
                    platform: None,
                    settings: HashMap::from([(
                        "theme".into(),
                        toml::Value::String("lx-256".into()),
                    )]),
                },
                ConditionalOverride {
                    env: HashMap::from([(
                        "COLORTERM".into(),
                        toml::Value::Array(vec![
                            toml::Value::String("truecolor".into()),
                            toml::Value::String("24bit".into()),
                        ]),
                    )]),
                    platform: None,
                    settings: HashMap::from([(
                        "theme".into(),
                        toml::Value::String("lx-24bit".into()),
                    )]),
                },
                ConditionalOverride {
                    env: HashMap::new(),
                    platform: Some(toml::Value::String("macos".into())),
                    settings: HashMap::from([("xattr-indicator".into(), Boolean(false))]),
                },
            ],
            ..Default::default()
        }),
        "lx" => Some(PersonalityDef {
            description: Some("Default for the `lx` binary; inherits `default`".into()),
            inherits: Some("default".into()),
            ..Default::default()
        }),
        "ll" => Some(PersonalityDef {
            description: Some("Two-tier long view; directories grouped first".into()),
            inherits: Some("lx".into()),
            format: Some("long2".into()),
            settings: HashMap::from([("group-dirs".into(), Str("first".into()))]),
            ..Default::default()
        }),
        "lll" => Some(PersonalityDef {
            description: Some("Three-tier long view with header and ISO timestamps".into()),
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
            description: Some("Like `ll`, but includes hidden files".into()),
            inherits: Some("ll".into()),
            settings: HashMap::from([("all".into(), Boolean(true))]),
            ..Default::default()
        }),
        "tree" => Some(PersonalityDef {
            description: Some("Long-view tree; directories grouped first".into()),
            inherits: Some("default".into()),
            format: Some("long2".into()),
            settings: HashMap::from([
                ("tree".into(), Boolean(true)),
                ("group-dirs".into(), Str("first".into())),
            ]),
            ..Default::default()
        }),
        "ls" => Some(PersonalityDef {
            description: Some(
                "Plain `ls`-style: across-the-rows grid, no colours, no decorations".into(),
            ),
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

/// Format a single TOML key = value pair.
fn format_toml_kv(key: &str, value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("{key} = \"{s}\""),
        toml::Value::Boolean(b) => format!("{key} = {b}"),
        toml::Value::Integer(i) => format!("{key} = {i}"),
        toml::Value::Float(f) => format!("{key} = {f}"),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    toml::Value::String(s) => format!("\"{s}\""),
                    other => other.to_string(),
                })
                .collect();
            format!("{key} = [{}]", items.join(", "))
        }
        other => format!("{key} = {other}"),
    }
}

/// Names of all compiled-in personalities.
const COMPILED_PERSONALITIES: &[&str] = &["default", "lx", "ll", "lll", "la", "tree", "ls"];

/// True iff `name` is a compiled-in personality.  A user-defined
/// personality with the same name shadows the compiled-in one but
/// is still considered to "exist as a builtin" by this predicate.
pub fn is_compiled_personality(name: &str) -> bool {
    COMPILED_PERSONALITIES.contains(&name)
}

/// Look up a personality's `description` field, preferring the
/// user's config when both shadow.  Returns `None` if the
/// personality has no description (or no definition at all).
pub fn personality_description(name: &str) -> Option<String> {
    lookup_personality(name).and_then(|d| d.description)
}

/// Return the names of all known personalities (compiled-in + config).
pub fn all_personality_names() -> Vec<String> {
    let mut names: Vec<String> = COMPILED_PERSONALITIES.iter().map(|s| (*s).into()).collect();
    if let Some(cfg) = config() {
        for name in cfg.personality.keys() {
            if !names.iter().any(|n| n == name) {
                names.push(name.clone());
            }
        }
    }
    // Topological sort: parents before children.  Within each depth
    // level, alphabetical order is preserved.
    names.sort();
    let mut ordered = Vec::with_capacity(names.len());
    let mut remaining = names;
    while !remaining.is_empty() {
        let (ready, rest): (Vec<_>, Vec<_>) = remaining.into_iter().partition(|name| {
            lookup_personality(name)
                .and_then(|def| def.inherits)
                .is_none_or(|parent| ordered.contains(&parent))
        });
        if ready.is_empty() {
            // Cycle or missing parent — append the rest alphabetically
            // to avoid an infinite loop.
            ordered.extend(rest);
            break;
        }
        ordered.extend(ready);
        remaining = rest;
    }
    ordered
}

/// Format a personality definition as TOML.
fn format_personality_toml(name: &str) -> Option<String> {
    // Look up the *unresolved* definition (without inheritance merging)
    // so the TOML output matches what you'd write in a config file.
    let def = lookup_personality(name)?;
    let mut lines = vec![format!("[personality.{name}]")];

    if let Some(ref description) = def.description {
        lines.push(format!("description = \"{description}\""));
    }
    if let Some(ref inherits) = def.inherits {
        lines.push(format!("inherits = \"{inherits}\""));
    }
    if let Some(ref format) = def.format {
        lines.push(format!("format = \"{format}\""));
    }
    if let Some(ref columns) = def.columns {
        let entries: Vec<String> = columns
            .to_csv()
            .split(',')
            .map(|s| format!("\"{}\"", s.trim()))
            .collect();
        lines.push(format!("columns = [{}]", entries.join(", ")));
    }

    // Sort settings for stable output.
    let mut keys: Vec<_> = def.settings.keys().collect();
    keys.sort();
    for key in keys {
        lines.push(format_toml_kv(key, &def.settings[key]));
    }

    // Emit [[when]] blocks.
    for cond in &def.when {
        lines.push(String::new());
        lines.push(format!("[[personality.{name}.when]]"));

        if let Some(p) = &cond.platform {
            lines.push(format_toml_kv("platform", p));
        }

        let mut env_keys: Vec<_> = cond.env.keys().collect();
        env_keys.sort();
        for ek in env_keys {
            lines.push(format_toml_kv(&format!("env.{ek}"), &cond.env[ek]));
        }

        let mut setting_keys: Vec<_> = cond.settings.keys().collect();
        setting_keys.sort();
        for sk in setting_keys {
            lines.push(format_toml_kv(sk, &cond.settings[sk]));
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
            if !first {
                println!();
            }
            println!("{toml}");
            first = false;
        }
    }
}

// ── --save-as=NAME / --show-as=NAME ─────────────────────────────

/// Build a TOML `[personality.NAME]` snippet from CLI settings.
///
/// `flag_name` is the originating CLI flag (`save-as` or `show-as`),
/// used only in the generated header comment.  When `name` is
/// `None`, every line of the body (including the
/// `[personality.UNNAMED]` header) is commented out — the snippet
/// is then a preview only, and an accidental redirect into a
/// config file produces something TOML parses as empty.
fn build_personality_toml(
    name: Option<&str>,
    inherits: Option<&str>,
    settings: &HashMap<String, toml::Value>,
    flag_name: &str,
) -> String {
    use chrono::Local;

    let resolved_name = name.unwrap_or("UNNAMED");
    let prefix = if name.is_some() { "" } else { "# " };

    let mut lines = Vec::new();
    lines.push(format!(
        "# Generated by lx --{flag_name} on {}",
        Local::now().format("%Y-%m-%d")
    ));
    if name.is_none() {
        lines.push(String::from(
            "# Anonymous preview — uncomment, rename, or redirect into a [personality.NAME] block.",
        ));
    }
    lines.push(String::new());
    lines.push(format!("{prefix}[personality.{resolved_name}]"));

    if let Some(parent) = inherits {
        lines.push(format!("{prefix}inherits = \"{parent}\""));
    }

    let mut keys: Vec<_> = settings.keys().collect();
    keys.sort();
    for key in keys {
        lines.push(format!("{prefix}{}", format_toml_kv(key, &settings[key])));
    }
    lines.push(String::new());

    lines.join("\n")
}

/// Print a personality definition to stdout (no file written).
///
/// Mirrors `save_personality_as` but emits the TOML to stdout for
/// previewing or piping.  `name` is `None` for the anonymous
/// preview form (`--show` or bare `--show-as`).
pub fn show_personality_as(
    name: Option<&str>,
    inherits: Option<&str>,
    settings: &HashMap<String, toml::Value>,
) {
    print!(
        "{}",
        build_personality_toml(name, inherits, settings, "show-as")
    );
}

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
    let toml_content = build_personality_toml(Some(name), inherits, settings, "save-as");

    // Find or create the conf.d/ directory.
    let conf_dir = find_drop_in_dir(find_config_path().as_deref()).unwrap_or_else(|| {
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
        eprintln!(
            "lx: backed up {} → {}",
            file_path.display(),
            backup.display()
        );
    }

    std::fs::write(&file_path, &toml_content).with_path(&file_path)?;

    eprintln!("lx: saved personality '{name}' to {}", file_path.display());
    Ok(())
}
