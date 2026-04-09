//! `--upgrade-config`: migrate older config files to the current schema.
//!
//! Detects the source version and applies the appropriate migration:
//! - 0.1 (unversioned): convert `[defaults]`, flatten formats, stamp current
//! - 0.2: flatten `[format.NAME]` sub-tables, stamp current
//! - 0.3 or 0.4: bump version string to current (no structural changes;
//!   the `time = "..."` setting, removed in 0.5, warns at load time)
//! - 0.5: bump version string to current (no structural changes; 0.6
//!   adds glob and array support to `[[when]]` env conditions, both
//!   purely additive — old configs work unchanged)

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use super::error::{ConfigError, IoResultExt};
use super::load::{detect_config_version, inject_auto_select_blocks};
use super::schema::CONFIG_VERSION;


/// Upgrade an older config file to the current format.
pub fn upgrade_config(path: &PathBuf) -> Result<(), ConfigError> {
    let contents = fs::read_to_string(path).with_path(path)?;
    let version = detect_config_version(&contents);

    if version == CONFIG_VERSION {
        return Err(ConfigError::AlreadyCurrent { path: path.clone() });
    }

    // Warn about settings removed in 0.5.
    if version == "0.3" || version == "0.4" {
        let mut warned_time = false;
        let mut warned_numeric = false;
        for line in contents.lines() {
            let trimmed = line.trim();
            if !trimmed.contains('=') {
                continue;
            }
            if !warned_time
                && trimmed.starts_with("time")
                && !trimmed.starts_with("time-style")
            {
                eprintln!(
                    "lx: warning: `time = \"...\"` is removed in config \
                     version 0.5; use `modified`, `changed`, `accessed`, \
                     or `created` booleans instead. Upgrading anyway."
                );
                warned_time = true;
            }
            if !warned_numeric && trimmed.starts_with("numeric") {
                eprintln!(
                    "lx: warning: `numeric = ...` is removed in config \
                     version 0.5; UID and GID are first-class columns now. \
                     Use `uid = true, gid = true, no-user = true, \
                     no-group = true` for the old behaviour. Upgrading anyway."
                );
                warned_numeric = true;
            }
            if warned_time && warned_numeric {
                break;
            }
        }
    }

    // 0.3, 0.4, or 0.5 → current: bump the version string in place.
    // For 0.5 → 0.6, also inject auto-selection [[when]] blocks into
    // [personality.default] (if it exists and doesn't already have
    // terminal-detection conditions).  Older versions don't support
    // [[when]] blocks at all and will pick up the auto-selection
    // when the user re-runs --upgrade-config from 0.5.
    if version == "0.3" || version == "0.4" || version == "0.5" {
        let backup = path.with_extension("toml.bak");
        fs::copy(path, &backup).with_path(&backup)?;

        let old_version_line = format!("version = \"{version}\"");
        let new_version_line = format!("version = \"{CONFIG_VERSION}\"");
        let mut updated = contents.replacen(&old_version_line, &new_version_line, 1);

        // Inject auto-selection blocks if upgrading from 0.5 and the
        // user has [personality.default] without existing terminal
        // detection.  Match on `env.TERM ` and `env.COLORTERM` as
        // tokens (with following space or `=`) so we don't false-match
        // on `env.TERM_PROGRAM` etc.
        let has_term_detection = updated.lines().any(|line| {
            let l = line.trim_start();
            l.starts_with("env.TERM ")
                || l.starts_with("env.TERM=")
                || l.starts_with("env.COLORTERM ")
                || l.starts_with("env.COLORTERM=")
        });
        if version == "0.5"
            && updated.contains("[personality.default]")
            && !has_term_detection
        {
            updated = inject_auto_select_blocks(&updated);
            eprintln!(
                "Note: added auto-selection [[when]] blocks to \
                 [personality.default] so capable terminals get the \
                 lx-256 / lx-24bit themes automatically.  Edit or \
                 delete to opt out."
            );
        }

        fs::write(path, &updated).with_path(path)?;

        eprintln!("Original config saved to {}", backup.display());
        eprintln!("Upgraded {} from version {version} to {CONFIG_VERSION}", path.display());
        return Ok(());
    }

    // Parse with the permissive legacy struct (handles all old formats).
    let legacy: LegacyConfig = toml::from_str(&contents)
        .map_err(|source| ConfigError::Parse { path: path.clone(), source })?;

    let mut out = String::new();
    out.push_str(&format!("version = \"{CONFIG_VERSION}\"\n"));

    // Formats — always emit as flat [format] section.
    if !legacy.format.is_empty() {
        out.push_str("\n[format]\n");
        for (name, columns) in &legacy.format {
            out.push_str(&format!(
                "{name} = [{}]\n",
                columns.iter()
                    .map(|c| format!("\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // 0.1-specific: convert [defaults] → [personality.default].
    if version == "0.1" && !legacy.defaults.is_empty() {
        out.push_str("\n[personality.default]\n");
        for (key, value) in &legacy.defaults {
            out.push_str(&format!("{key} = {value}\n"));
        }

        // Ensure [personality.lx] inherits from "default".
        let has_lx = legacy.personality.contains_key("lx");
        if has_lx {
            out.push_str("\n[personality.lx]\n");
            out.push_str("inherits = \"default\"\n");
            if let Some(lx_p) = legacy.personality.get("lx") {
                for (key, value) in lx_p {
                    if key != "inherits" {
                        out.push_str(&format!("{key} = {value}\n"));
                    }
                }
            }
        } else {
            out.push_str("\n[personality.lx]\ninherits = \"default\"\n");
        }
    }

    // Personalities (preserved as-is, except lx handled above for 0.1).
    for (name, settings) in &legacy.personality {
        if version == "0.1" && name == "lx" {
            continue;  // already handled above
        }
        out.push_str(&format!("\n[personality.{name}]\n"));
        for (key, value) in settings {
            out.push_str(&format!("{key} = {value}\n"));
        }
    }

    // Back up the original.
    let backup = path.with_extension("toml.bak");
    fs::copy(path, &backup).with_path(&backup)?;

    // Write the new config.
    fs::write(path, &out).with_path(path)?;

    eprintln!("Original config saved to {}", backup.display());
    eprintln!("Upgraded {} from version {version} to {CONFIG_VERSION}", path.display());
    eprintln!("Note: comments were not preserved. You may want to review the result.");

    Ok(())
}

/// Legacy config structure for migration.  Uses raw TOML tables
/// so we can round-trip key-value pairs without losing data.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyConfig {
    #[serde(default)]
    defaults: HashMap<String, toml::Value>,

    #[serde(default)]
    format: HashMap<String, Vec<String>>,

    #[serde(default)]
    personality: HashMap<String, HashMap<String, toml::Value>>,
}
