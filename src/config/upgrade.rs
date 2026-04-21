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
use std::fmt::Write as _;
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
            if !warned_time && trimmed.starts_with("time") && !trimmed.starts_with("time-style") {
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
        if version == "0.5" && updated.contains("[personality.default]") && !has_term_detection {
            updated = inject_auto_select_blocks(&updated);
            eprintln!(
                "Note: added auto-selection [[when]] blocks to \
                 [personality.default] so capable terminals get the \
                 lx-256 / lx-24bit themes automatically.  Edit or \
                 delete to opt out."
            );
        }

        // Rewrite `colour-scale = "..."` (and the `color-scale`
        // alias) to the new `gradient = "..."` form.  See
        // rewrite_colour_scale_to_gradient for the value mapping.
        let (rewritten, rewrites) = rewrite_colour_scale_to_gradient(&updated);
        updated = rewritten;
        if rewrites > 0 {
            eprintln!(
                "Note: rewrote {rewrites} `colour-scale = \"...\"` line{} \
                 to `gradient = \"...\"`.  See `man lx` and \
                 `docs/UPGRADING.md` for the new vocabulary.",
                if rewrites == 1 { "" } else { "s" },
            );
        }

        fs::write(path, &updated).with_path(path)?;

        eprintln!("Original config saved to {}", backup.display());
        eprintln!(
            "Upgraded {} from version {version} to {CONFIG_VERSION}",
            path.display()
        );
        return Ok(());
    }

    // Parse with the permissive legacy struct (handles all old formats).
    let legacy: LegacyConfig = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.clone(),
        source,
    })?;

    let mut out = String::new();
    writeln!(out, "version = \"{CONFIG_VERSION}\"").unwrap();

    // Formats — always emit as flat [format] section.
    if !legacy.format.is_empty() {
        out.push_str("\n[format]\n");
        for (name, columns) in &legacy.format {
            let cols = columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{name} = [{cols}]").unwrap();
        }
    }

    // 0.1-specific: convert [defaults] → [personality.default].
    if version == "0.1" && !legacy.defaults.is_empty() {
        out.push_str("\n[personality.default]\n");
        for (key, value) in &legacy.defaults {
            writeln!(out, "{key} = {value}").unwrap();
        }

        // Ensure [personality.lx] inherits from "default".
        let has_lx = legacy.personality.contains_key("lx");
        if has_lx {
            out.push_str("\n[personality.lx]\n");
            out.push_str("inherits = \"default\"\n");
            if let Some(lx_p) = legacy.personality.get("lx") {
                for (key, value) in lx_p {
                    if key != "inherits" {
                        writeln!(out, "{key} = {value}").unwrap();
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
            continue; // already handled above
        }
        writeln!(out, "\n[personality.{name}]").unwrap();
        for (key, value) in settings {
            writeln!(out, "{key} = {value}").unwrap();
        }
    }

    // Back up the original.
    let backup = path.with_extension("toml.bak");
    fs::copy(path, &backup).with_path(&backup)?;

    // Write the new config.
    fs::write(path, &out).with_path(path)?;

    eprintln!("Original config saved to {}", backup.display());
    eprintln!(
        "Upgraded {} from version {version} to {CONFIG_VERSION}",
        path.display()
    );
    eprintln!("Note: comments were not preserved. You may want to review the result.");

    Ok(())
}

/// Rewrite `colour-scale = "..."` (and the `color-scale` alias)
/// lines to the new `gradient = "..."` form.
///
/// Operates as a per-line text rewrite — the setting only ever
/// appears inside `[personality.NAME]` or `[[personality.NAME.when]]`
/// blocks, so we don't need to track section context.  Whitespace,
/// comments, and surrounding lines are preserved.
///
/// Value mapping:
/// - `"none"` → `"none"`   (still no gradients)
/// - `"16"`   → `"all"`    (the depth distinction is gone)
/// - `"256"`  → `"all"`    (ditto)
///
/// Returns `(rewritten_content, rewrite_count)`.
fn rewrite_colour_scale_to_gradient(contents: &str) -> (String, usize) {
    let mut out = String::with_capacity(contents.len());
    let mut count = 0;
    for line in contents.split_inclusive('\n') {
        // Strip the trailing newline for matching, then add it back.
        let (body, newline) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        let trimmed = body.trim_start();
        let indent_len = body.len() - trimmed.len();
        let indent = &body[..indent_len];

        let key_match = trimmed
            .strip_prefix("colour-scale")
            .or_else(|| trimmed.strip_prefix("color-scale"));

        if let Some(rest) = key_match {
            // Must be followed by whitespace + `=` to be a TOML
            // key/value line; otherwise it's part of something else
            // (e.g. a comment, a longer key name).
            let after = rest.trim_start();
            if let Some(value_part) = after.strip_prefix('=') {
                let value_part = value_part.trim_start();
                // Strip an optional trailing comment.
                let (val_with_quotes, comment) = match value_part.find('#') {
                    Some(i) => (value_part[..i].trim_end(), &value_part[i..]),
                    None => (value_part.trim_end(), ""),
                };
                let stripped = val_with_quotes
                    .trim_start_matches(['"', '\''])
                    .trim_end_matches(['"', '\'']);
                let new_value = match stripped {
                    "none" => Some("none"),
                    "16" | "256" => Some("all"),
                    _ => None,
                };
                if let Some(v) = new_value {
                    out.push_str(indent);
                    out.push_str("gradient = \"");
                    out.push_str(v);
                    out.push('"');
                    if !comment.is_empty() {
                        out.push(' ');
                        out.push_str(comment);
                    }
                    out.push_str(newline);
                    count += 1;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    (out, count)
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

#[cfg(test)]
mod rewrite_test {
    use super::rewrite_colour_scale_to_gradient;

    #[test]
    fn rewrites_none() {
        let (out, n) =
            rewrite_colour_scale_to_gradient("[personality.default]\ncolour-scale = \"none\"\n");
        assert_eq!(n, 1);
        assert!(out.contains("gradient = \"none\""));
        assert!(!out.contains("colour-scale"));
    }

    #[test]
    fn rewrites_16_to_all() {
        let (out, n) =
            rewrite_colour_scale_to_gradient("[personality.default]\ncolour-scale = \"16\"\n");
        assert_eq!(n, 1);
        assert!(out.contains("gradient = \"all\""));
    }

    #[test]
    fn rewrites_256_to_all() {
        let (out, n) =
            rewrite_colour_scale_to_gradient("[personality.default]\ncolour-scale = \"256\"\n");
        assert_eq!(n, 1);
        assert!(out.contains("gradient = \"all\""));
    }

    #[test]
    fn rewrites_color_scale_alias() {
        let (out, n) =
            rewrite_colour_scale_to_gradient("[personality.default]\ncolor-scale = \"none\"\n");
        assert_eq!(n, 1);
        assert!(out.contains("gradient = \"none\""));
    }

    #[test]
    fn preserves_indent_and_trailing_comment() {
        let (out, n) =
            rewrite_colour_scale_to_gradient("  colour-scale = \"16\"   # nice gradient\n");
        assert_eq!(n, 1);
        assert!(out.contains("  gradient = \"all\" # nice gradient"));
    }

    #[test]
    fn does_not_rewrite_unknown_value() {
        let (out, n) = rewrite_colour_scale_to_gradient("colour-scale = \"surprise\"\n");
        assert_eq!(n, 0);
        assert!(out.contains("colour-scale"));
    }

    #[test]
    fn rewrites_inside_when_block() {
        let input = "[[personality.lx.when]]\nenv.SSH_CONNECTION = true\ncolour-scale = \"none\"\n";
        let (out, n) = rewrite_colour_scale_to_gradient(input);
        assert_eq!(n, 1);
        assert!(out.contains("gradient = \"none\""));
    }
}
