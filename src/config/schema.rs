//! Data types deserialised from a `lxconfig.toml` file.
//!
//! These are the on-disk schema: the top-level `Config`, plus the
//! per-section types (`PersonalityDef`, `ThemeDef`, `StyleDef`,
//! `ConditionalOverride`).  None of the resolution or rendering logic
//! lives here — see `personality.rs`, `themes.rs`, etc.

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use serde::Deserialize;

use super::settings::settings_to_args;

// ── Config schema versioning ────────────────────────────────────

/// The current config schema version.
pub const CONFIG_VERSION: &str = "0.6";

/// Accepted config versions.  0.3, 0.4, and 0.5 are forward-compatible
/// subsets of 0.6: any config file from these versions loads fine in
/// the current parser.  0.6 adds glob and array support to `[[when]]`
/// env conditions — both purely additive.  The only ever-removed
/// setting is `time = "..."` (gone in 0.5), which triggers a warning
/// and is ignored if found in 0.3/0.4 files.
pub(super) const ACCEPTED_VERSIONS: &[&str] = &["0.3", "0.4", "0.5", "0.6"];

// ── Top-level config ────────────────────────────────────────────

/// Top-level config file structure.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Config schema version.  `None` means a legacy (pre-0.2) config.
    pub version: Option<String>,

    #[serde(default)]
    pub format: HashMap<String, Vec<String>>,

    #[serde(default)]
    pub personality: HashMap<String, PersonalityDef>,

    /// Named theme definitions: `[theme.NAME]`.
    #[serde(default)]
    pub theme: HashMap<String, ThemeDef>,

    /// Named file colour style sets: `[style.NAME]`.
    #[serde(default)]
    pub style: HashMap<String, StyleDef>,

    /// File-type class definitions: `[class]`.
    /// Each key is a class name, each value a list of glob patterns.
    #[serde(default)]
    pub class: HashMap<String, Vec<String>>,

    /// Paths of loaded drop-in fragments (for `--show-config`).
    #[serde(skip)]
    pub drop_in_paths: Vec<PathBuf>,
}

impl Config {
    /// Merge a drop-in fragment into this config.  Each named entry
    /// in the fragment overrides the same-named entry in `self`.
    pub(super) fn merge(&mut self, other: Config) {
        for (k, v) in other.format {
            self.format.insert(k, v);
        }
        for (k, v) in other.personality {
            self.personality.insert(k, v);
        }
        for (k, v) in other.theme {
            self.theme.insert(k, v);
        }
        for (k, v) in other.style {
            self.style.insert(k, v);
        }
        for (k, v) in other.class {
            self.class.insert(k, v);
        }
    }
}

// ── ThemeDef ────────────────────────────────────────────────────

/// A named theme definition under `[theme.NAME]`.
///
/// UI element keys are captured via `serde(flatten)` into a flat map.
/// File colour styles are referenced by name from `[style.NAME]`
/// sections.
///
/// Theme selection happens through personalities (`theme = "NAME"`)
/// or the `--theme=NAME` CLI flag.
///
/// Themes can inherit from other themes via `inherits = "NAME"`.
/// The special name `"exa"` refers to the compiled-in default theme.
/// Without `inherits`, a theme starts from a blank slate.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct ThemeDef {
    /// Inherit from another theme.  The parent's UI keys are applied
    /// first; this theme's keys override.  The special name `"exa"`
    /// refers to the compiled-in default theme.
    pub inherits: Option<String>,

    /// Reference a named style set from `[style.NAME]`.
    pub use_style: Option<String>,

    /// UI element colour overrides (flat keys like `directory`, `date`, etc.)
    #[serde(flatten)]
    pub ui: HashMap<String, String>,
}

// ── StyleDef ────────────────────────────────────────────────────

/// A named file colour style set under `[style.NAME]`.
///
/// Class references use bare dotted TOML keys (`class.media`),
/// which serde deserialises into the `class` sub-table.  File
/// patterns use quoted TOML keys (`"*.rs"`, `"Makefile"`), which
/// land in the `patterns` map via `serde(flatten)`.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct StyleDef {
    /// Class references: `class.NAME = "colour"` (bare dotted keys).
    #[serde(default, rename = "class")]
    pub classes: HashMap<String, String>,

    /// File patterns: `"*.rs" = "colour"` (quoted keys).
    /// Keys with glob metacharacters are glob patterns; keys without
    /// are exact filename matches.
    #[serde(flatten)]
    pub patterns: HashMap<String, String>,
}

// ── ConditionalOverride ─────────────────────────────────────────

/// A conditional override block: `[[personality.NAME.when]]`.
///
/// Environment conditions use `env.VAR = value` where the value type
/// determines the check:
/// - **String** (`env.TERM_PROGRAM = "ghostty"`) — exact match
/// - **`true`** (`env.SSH_CONNECTION = true`) — variable must be set
///   (to any value, including empty)
/// - **`false`** (`env.DISPLAY = false`) — variable must be truly
///   unset (not just empty)
///
/// All conditions in a block must match (AND logic).
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct ConditionalOverride {
    /// Environment variable conditions.  Values are either strings
    /// (exact match), `true` (must be set), or `false` (must be unset).
    #[serde(default)]
    pub env: HashMap<String, toml::Value>,

    /// Settings to overlay when conditions match.
    #[serde(flatten)]
    pub settings: HashMap<String, toml::Value>,
}

impl ConditionalOverride {
    /// Check whether all `env` conditions are satisfied.
    ///
    /// Each value can be:
    /// - **String** — literal exact match, OR a glob pattern if it
    ///   contains glob metacharacters (`*`, `?`, `[`).
    /// - **Array of strings** — any element matches (each element is
    ///   independently treated as literal-or-glob).
    /// - **`true`** — variable must be set to anything (even empty).
    /// - **`false`** — variable must be unset entirely.
    ///
    /// Globs and arrays were added in config schema 0.6; the existing
    /// literal-string and boolean forms continue to work unchanged.
    pub(super) fn matches(&self) -> bool {
        self.env.iter().all(|(key, condition)| {
            let actual = env::var(key).unwrap_or_default();
            match condition {
                toml::Value::String(expected) => super::load::match_string(&actual, expected),
                toml::Value::Array(items) => items.iter().any(|item| match item {
                    toml::Value::String(s) => super::load::match_string(&actual, s),
                    _ => false,
                }),
                toml::Value::Boolean(true) => env::var(key).is_ok(),
                toml::Value::Boolean(false) => env::var(key).is_err(),
                // Anything else: ignore (treat as always-true).
                _ => true,
            }
        })
    }
}

// ── PersonalityDef ──────────────────────────────────────────────

/// A personality bundles format, columns, and settings.
///
/// `format` and `columns` are structural fields (they define the
/// column layout).  `inherits` controls how personalities compose.
/// All other settings are captured via `serde(flatten)` and
/// converted to CLI args via `SETTING_FLAGS`.
///
/// Conditional overrides (`[[personality.NAME.when]]`) allow settings
/// to vary based on environment variables.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct PersonalityDef {
    /// Inherit from another personality.  The parent's settings
    /// are applied first; this personality's values override per-key.
    /// `format` and `columns` replace (not merge) the parent's.
    pub inherits: Option<String>,

    /// Reference to a named format (looked up in `[format.*]`).
    pub format: Option<String>,

    /// Inline column list (overrides `format` if both given).
    /// Accepts a TOML array or a comma-separated string.
    pub columns: Option<StringOrList>,

    /// Conditional overrides: `[[personality.NAME.when]]` blocks.
    #[serde(default)]
    pub when: Vec<ConditionalOverride>,

    /// All other settings, converted to CLI args via `SETTING_FLAGS`.
    #[serde(flatten)]
    pub settings: HashMap<String, toml::Value>,
}

impl PersonalityDef {
    /// Convert this personality's settings to synthetic CLI arguments.
    /// Order: columns/format first, then named settings.
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();

        // Structural fields.
        if let Some(ref cols) = self.columns {
            args.push(format!("--columns={}", cols.to_csv()).into());
        } else if let Some(ref fmt) = self.format {
            args.push(format!("--format={fmt}").into());
        }

        // Named settings.
        args.extend(settings_to_args(&self.settings, "[personality]"));

        args
    }
}

// ── StringOrList ────────────────────────────────────────────────

/// A value that can be either a TOML string (comma-separated) or a
/// TOML array of strings.  Used for the `columns` field.
#[derive(Debug, Clone)]
pub enum StringOrList {
    Str(String),
    List(Vec<String>),
}

impl StringOrList {
    /// Convert to a comma-separated string suitable for `--columns=`.
    pub fn to_csv(&self) -> String {
        match self {
            Self::Str(s) => s.clone(),
            Self::List(v) => v.join(","),
        }
    }
}

impl<'de> Deserialize<'de> for StringOrList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StringOrList;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a string or array of strings")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<StringOrList, E> {
                Ok(StringOrList::Str(v.to_string()))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<StringOrList, A::Error> {
                let mut v = Vec::new();
                while let Some(s) = seq.next_element::<String>()? {
                    v.push(s);
                }
                Ok(StringOrList::List(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}
