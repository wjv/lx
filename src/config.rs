//! Configuration file discovery, loading, and data model.
//!
//! lx looks for a TOML config file in these locations (first found wins):
//!
//! 1. `$LX_CONFIG` — explicit path override
//! 2. `~/.lxconfig.toml` — simple home directory location
//! 3. `$XDG_CONFIG_HOME/lx/config.toml` (default `~/.config/lx/config.toml`)
//! 4. `~/Library/Application Support/lx/config.toml` (macOS only)

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use log::*;
use serde::Deserialize;
use thiserror::Error;


// ── Error types ─────────────────────────────────────────────────

/// Errors that can occur when loading or resolving configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// I/O error accessing a config file.
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// TOML parsing failed.
    #[error("error parsing {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// The config file uses an older format and needs upgrading.
    #[error("config file {path} uses version {version} format.\n\
             Run `lx --upgrade-config` to migrate it to version {CONFIG_VERSION}.")]
    NeedsUpgrade { path: PathBuf, version: String },

    /// Personality inheritance forms a cycle.
    #[error("personality inheritance cycle: {chain}")]
    InheritanceCycle { chain: String },

    /// A personality inherits from a name that doesn't exist.
    #[error("personality '{child}' inherits from '{parent}', which does not exist")]
    MissingParent { child: String, parent: String },

    /// `--upgrade-config` on a config that is already current.
    #[error("{path} is already at version {CONFIG_VERSION}; no upgrade needed")]
    AlreadyCurrent { path: PathBuf },
}

/// Extension trait for attaching path context to `io::Result`.
trait IoResultExt<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T, ConfigError>;
}

impl<T> IoResultExt<T> for std::io::Result<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T, ConfigError> {
        self.map_err(|source| ConfigError::Io { path: path.into(), source })
    }
}


/// Global config, loaded once at startup.
pub static CONFIG: LazyLock<Option<Config>> = LazyLock::new(load_config);


// ── Setting-to-flag mapping ─────────────────────────────────────

/// How a TOML value maps to a CLI argument.
enum SettingKind {
    /// String value: `--flag=VALUE`
    Str,
    /// Boolean: `true` → `--flag`, `false` → omitted
    Bool,
    /// Integer: `--flag=N`
    Int,
}

/// Maps a config key name to its CLI flag and value kind.
struct SettingDef {
    key: &'static str,
    flag: &'static str,
    kind: SettingKind,
}

/// Master table of all config keys that map to CLI flags.
///
/// Adding a new flag to lx = adding one entry here.
static SETTING_FLAGS: &[SettingDef] = &[
    // display options
    SettingDef { key: "oneline",       flag: "--oneline",        kind: SettingKind::Bool },
    SettingDef { key: "long",          flag: "--long",           kind: SettingKind::Bool },
    SettingDef { key: "grid",          flag: "--grid",           kind: SettingKind::Bool },
    SettingDef { key: "across",        flag: "--across",         kind: SettingKind::Bool },
    SettingDef { key: "recurse",       flag: "--recurse",        kind: SettingKind::Bool },
    SettingDef { key: "tree",          flag: "--tree",           kind: SettingKind::Bool },
    SettingDef { key: "classify",      flag: "--classify",       kind: SettingKind::Str },
    SettingDef { key: "colour",        flag: "--colour",         kind: SettingKind::Str },
    SettingDef { key: "color",         flag: "--colour",         kind: SettingKind::Str },
    SettingDef { key: "colour-scale",  flag: "--colour-scale",   kind: SettingKind::Str },
    SettingDef { key: "color-scale",   flag: "--colour-scale",   kind: SettingKind::Str },
    SettingDef { key: "icons",         flag: "--icons",          kind: SettingKind::Str },

    // filtering and sorting
    SettingDef { key: "all",           flag: "--all",            kind: SettingKind::Bool },
    SettingDef { key: "list-dirs",     flag: "--list-dirs",      kind: SettingKind::Bool },
    SettingDef { key: "level",         flag: "--level",          kind: SettingKind::Int },
    SettingDef { key: "reverse",       flag: "--reverse",        kind: SettingKind::Bool },
    SettingDef { key: "sort",          flag: "--sort",           kind: SettingKind::Str },
    SettingDef { key: "group-dirs",    flag: "--group-dirs",     kind: SettingKind::Str },
    SettingDef { key: "only-dirs",     flag: "--only-dirs",      kind: SettingKind::Bool },
    SettingDef { key: "only-files",    flag: "--only-files",     kind: SettingKind::Bool },

    // long view options
    SettingDef { key: "binary",        flag: "--binary",         kind: SettingKind::Bool },
    SettingDef { key: "bytes",         flag: "--bytes",          kind: SettingKind::Bool },
    SettingDef { key: "header",        flag: "--header",         kind: SettingKind::Bool },
    SettingDef { key: "inode",         flag: "--inode",          kind: SettingKind::Bool },
    SettingDef { key: "links",         flag: "--links",          kind: SettingKind::Bool },
    SettingDef { key: "blocks",        flag: "--blocks",         kind: SettingKind::Bool },
    SettingDef { key: "group",         flag: "--group",          kind: SettingKind::Bool },
    SettingDef { key: "numeric",       flag: "--numeric",        kind: SettingKind::Bool },
    SettingDef { key: "time-style",    flag: "--time-style",     kind: SettingKind::Str },
    SettingDef { key: "time",          flag: "--time",           kind: SettingKind::Str },
    SettingDef { key: "modified",      flag: "--modified",       kind: SettingKind::Bool },
    SettingDef { key: "changed",       flag: "--changed",        kind: SettingKind::Bool },
    SettingDef { key: "accessed",      flag: "--accessed",       kind: SettingKind::Bool },
    SettingDef { key: "created",       flag: "--created",        kind: SettingKind::Bool },
    SettingDef { key: "total-size",    flag: "--total-size",     kind: SettingKind::Bool },
    SettingDef { key: "extended",      flag: "--extended",       kind: SettingKind::Bool },
    SettingDef { key: "octal-permissions", flag: "--octal-permissions", kind: SettingKind::Bool },

    // VCS
    SettingDef { key: "vcs",           flag: "--vcs",            kind: SettingKind::Str },
    SettingDef { key: "vcs-status",    flag: "--vcs-status",     kind: SettingKind::Bool },
    SettingDef { key: "vcs-ignore",    flag: "--vcs-ignore",     kind: SettingKind::Bool },

    // theme
    SettingDef { key: "theme",         flag: "--theme",           kind: SettingKind::Str },

    // display
    SettingDef { key: "width",         flag: "--width",           kind: SettingKind::Int },
    SettingDef { key: "absolute",      flag: "--absolute",        kind: SettingKind::Bool },
    SettingDef { key: "hyperlink",     flag: "--hyperlink",       kind: SettingKind::Str },
    SettingDef { key: "quotes",        flag: "--quotes",          kind: SettingKind::Str },

    // explicit column enablers
    SettingDef { key: "permissions",   flag: "--permissions",    kind: SettingKind::Bool },
    SettingDef { key: "filesize",      flag: "--filesize",       kind: SettingKind::Bool },
    SettingDef { key: "user",          flag: "--user",           kind: SettingKind::Bool },

    // column suppressors
    SettingDef { key: "no-permissions", flag: "--no-permissions", kind: SettingKind::Bool },
    SettingDef { key: "no-filesize",   flag: "--no-filesize",    kind: SettingKind::Bool },
    SettingDef { key: "no-user",       flag: "--no-user",        kind: SettingKind::Bool },
    SettingDef { key: "no-time",       flag: "--no-time",        kind: SettingKind::Bool },
    SettingDef { key: "no-icons",      flag: "--no-icons",       kind: SettingKind::Bool },
    SettingDef { key: "no-inode",      flag: "--no-inode",       kind: SettingKind::Bool },
    SettingDef { key: "no-group",      flag: "--no-group",       kind: SettingKind::Bool },
    SettingDef { key: "no-links",      flag: "--no-links",       kind: SettingKind::Bool },
    SettingDef { key: "no-blocks",     flag: "--no-blocks",      kind: SettingKind::Bool },
];

/// Look up a setting definition by config key name.
fn find_setting(key: &str) -> Option<&'static SettingDef> {
    SETTING_FLAGS.iter().find(|s| s.key == key)
}

/// Convert a settings map (from `[defaults]` or `[personality.*]`)
/// into synthetic CLI arguments.  Unknown keys are warned about.
fn settings_to_args(settings: &HashMap<String, toml::Value>, context: &str) -> Vec<OsString> {
    let mut args = Vec::new();

    for (key, value) in settings {
        let Some(def) = find_setting(key) else {
            eprintln!("lx: unknown setting '{key}' in {context}");
            continue;
        };

        match def.kind {
            SettingKind::Bool => {
                let truthy = match value {
                    toml::Value::Boolean(b) => *b,
                    toml::Value::String(s) => s == "true",
                    _ => {
                        warn!("Expected boolean for '{key}' in {context}; ignoring");
                        continue;
                    }
                };
                if truthy {
                    args.push(def.flag.into());
                }
            }
            SettingKind::Str => {
                let s = if let toml::Value::String(s) = value { s.as_str() } else {
                    warn!("Expected string for '{key}' in {context}; ignoring");
                    continue;
                };
                args.push(format!("{}={s}", def.flag).into());
            }
            SettingKind::Int => {
                let n = match value {
                    toml::Value::Integer(n) => *n,
                    toml::Value::String(s) => {
                        if let Ok(n) = s.parse::<i64>() { n } else {
                            warn!("Expected integer for '{key}' in {context}; ignoring");
                            continue;
                        }
                    }
                    _ => {
                        warn!("Expected integer for '{key}' in {context}; ignoring");
                        continue;
                    }
                };
                args.push(format!("{}={n}", def.flag).into());
            }
        }
    }

    args
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

    /// Convert to a Vec of individual column names.
    #[allow(dead_code)]  // will be used once view.rs reads columns directly
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Str(s) => s.split(',').map(|s| s.trim().to_string()).collect(),
            Self::List(v) => v.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for StringOrList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
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

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<StringOrList, A::Error> {
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


// ── Config types ────────────────────────────────────────────────

/// The current config schema version.
pub const CONFIG_VERSION: &str = "0.3";

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
}

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


/// A personality bundles format, columns, and settings.
///
/// `format` and `columns` are structural fields (they define the
/// column layout).  `inherits` controls how personalities compose.
/// All other settings are captured via `serde(flatten)` and
/// converted to CLI args via `SETTING_FLAGS`.
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


// ── Config file discovery and loading ───────────────────────────

/// Search for a config file and return its path, or `None`.
pub fn find_config_path() -> Option<PathBuf> {
    // 1. Explicit env var.
    // If the user sets LX_CONFIG, we trust it unconditionally.
    // If it points to a file, use it.  If it doesn't exist or
    // is not a regular file (e.g. /dev/null), use no config.
    // We never fall through to the default search paths.
    if let Ok(path) = env::var("LX_CONFIG") {
        let p = PathBuf::from(&path);
        if p.is_file() {
            debug!("Config from LX_CONFIG: {}", p.display());
            return Some(p);
        }
        debug!("LX_CONFIG={path}: not a file, no config");
        return None;
    }

    // 2. ~/.lxconfig.toml
    if let Some(home) = home_dir() {
        let p = home.join(".lxconfig.toml");
        if p.is_file() {
            debug!("Config from home dir: {}", p.display());
            return Some(p);
        }
    }

    // 3. XDG_CONFIG_HOME/lx/config.toml
    let xdg = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            home_dir()
                .map(|h| h.join(".config"))
                .unwrap_or_default()
        });
    let p = xdg.join("lx").join("config.toml");
    if p.is_file() {
        debug!("Config from XDG: {}", p.display());
        return Some(p);
    }

    // 4. macOS ~/Library/Application Support/lx/config.toml
    #[cfg(target_os = "macos")]
    if let Some(home) = home_dir() {
        let p = home.join("Library/Application Support/lx/config.toml");
        if p.is_file() {
            debug!("Config from macOS Library: {}", p.display());
            return Some(p);
        }
    }

    None
}


/// Load and parse the config file, if one is found.
///
/// Returns `Ok(None)` if no config file exists.  Returns a typed
/// `ConfigError` on I/O failures, parse errors, or legacy format.
fn try_load_config() -> Result<Option<Config>, ConfigError> {
    let Some(path) = find_config_path() else {
        return Ok(None);
    };

    let contents = fs::read_to_string(&path).with_path(&path)?;

    // Check the version before full parsing.
    let version = detect_config_version(&contents);
    if version != CONFIG_VERSION {
        return Err(ConfigError::NeedsUpgrade {
            path,
            version: version.to_string(),
        });
    }

    let config: Config = toml::from_str(&contents)
        .map_err(|source| ConfigError::Parse { path: path.clone(), source })?;

    info!("Loaded config from {}", path.display());
    Ok(Some(config))
}

/// Load config for the `LazyLock` static.  Errors are printed to
/// stderr and result in `None` (lx continues without a config).
fn load_config() -> Option<Config> {
    match try_load_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("lx: {e}");
            None
        }
    }
}

/// Detect the config schema version from raw file contents.
///
/// Returns the version string, or `"0.1"` if no version field
/// is found (legacy config).
fn detect_config_version(contents: &str) -> &str {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") && trimmed.contains('=') {
            // Extract the value after '=', stripping quotes and whitespace.
            if let Some(val) = trimmed.split('=').nth(1) {
                let val = val.trim().trim_matches('"');
                return match val {
                    "0.2" => "0.2",
                    "0.3" => "0.3",
                    _ => val,  // unknown version — will fail the check
                };
            }
        }
    }
    "0.1"  // no version field → legacy
}


/// The default config as a commented TOML string, for `--init-config`.
pub fn default_config_toml() -> &'static str {
    include_str!("../lxconfig.default.toml")
}


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
    }

    Ok(Some(effective))
}

/// Look up a single personality definition by name (no inheritance).
fn lookup_personality(name: &str) -> Option<PersonalityDef> {
    if let Some(ref cfg) = *CONFIG
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
        "default" => Some(PersonalityDef {
            settings: HashMap::from([
                ("theme".into(), toml::Value::String("exa".into())),
            ]),
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


/// Return the compiled-in file-type class definitions.
///
/// These correspond to the categories in `src/info/filetype.rs`.
/// Config-defined `[class]` entries override these.
pub fn compiled_classes() -> HashMap<String, Vec<String>> {
    fn gl(exts: &[&str]) -> Vec<String> {
        exts.iter().map(|e| format!("*.{e}")).collect()
    }

    HashMap::from([
        ("image".into(), gl(&[
            "png", "jfi", "jfif", "jif", "jpe", "jpeg", "jpg", "gif", "bmp",
            "tiff", "tif", "ppm", "pgm", "pbm", "pnm", "webp", "raw", "arw",
            "svg", "stl", "eps", "dvi", "ps", "cbr", "jpf", "cbz", "xpm",
            "ico", "cr2", "orf", "nef", "heif", "avif", "jxl", "j2k", "jp2",
            "j2c", "jpx",
        ])),
        ("video".into(), gl(&[
            "avi", "flv", "m2v", "m4v", "mkv", "mov", "mp4", "mpeg",
            "mpg", "ogm", "ogv", "vob", "wmv", "webm", "m2ts", "heic",
        ])),
        ("music".into(), gl(&[
            "aac", "m4a", "mp3", "ogg", "wma", "mka", "opus",
        ])),
        ("lossless".into(), gl(&[
            "alac", "ape", "flac", "wav",
        ])),
        ("crypto".into(), gl(&[
            "asc", "enc", "gpg", "pgp", "sig", "signature", "pfx", "p12",
        ])),
        ("document".into(), gl(&[
            "djvu", "doc", "docx", "dvi", "eml", "eps", "fotd", "key",
            "keynote", "numbers", "odp", "odt", "pages", "pdf", "ppt",
            "pptx", "rtf", "xls", "xlsx",
        ])),
        ("compressed".into(), gl(&[
            "zip", "tar", "Z", "z", "gz", "bz2", "a", "ar", "7z",
            "iso", "dmg", "tc", "rar", "par", "tgz", "xz", "txz",
            "lz", "tlz", "lzma", "deb", "rpm", "zst", "lz4", "cpio",
        ])),
        ("compiled".into(), gl(&[
            "class", "elc", "hi", "o", "pyc", "zwc", "ko",
        ])),
        ("temp".into(), gl(&[
            "tmp", "swp", "swo", "swn", "bak", "bkp", "bk",
        ])),
        ("immediate".into(), vec![
            "Makefile".into(), "Cargo.toml".into(), "SConstruct".into(),
            "CMakeLists.txt".into(), "build.gradle".into(), "pom.xml".into(),
            "Rakefile".into(), "package.json".into(), "Gruntfile.js".into(),
            "Gruntfile.coffee".into(), "BUILD".into(), "BUILD.bazel".into(),
            "WORKSPACE".into(), "build.xml".into(), "Podfile".into(),
            "webpack.config.js".into(), "meson.build".into(),
            "composer.json".into(), "RoboFile.php".into(), "PKGBUILD".into(),
            "Justfile".into(), "Procfile".into(), "Dockerfile".into(),
            "Containerfile".into(), "Vagrantfile".into(), "Brewfile".into(),
            "Gemfile".into(), "Pipfile".into(), "build.sbt".into(),
            "mix.exs".into(), "bsconfig.json".into(), "tsconfig.json".into(),
        ]),
    ])
}

/// Resolve class definitions: config overrides compiled-in defaults.
pub fn resolve_classes() -> HashMap<String, Vec<String>> {
    let mut classes = compiled_classes();
    if let Some(ref cfg) = *CONFIG {
        for (name, patterns) in &cfg.class {
            classes.insert(name.clone(), patterns.clone());
        }
    }
    classes
}

/// Return the compiled-in "exa" style definition.
///
/// This maps the built-in file-type classes to their default colours,
/// matching the hard-coded values in `src/info/filetype.rs`.
pub fn compiled_exa_style() -> StyleDef {
    StyleDef {
        classes: HashMap::from([
            ("temp".into(),       "38;5;244".into()),
            ("immediate".into(),  "bold underline yellow".into()),
            ("image".into(),      "38;5;133".into()),
            ("video".into(),      "38;5;135".into()),
            ("music".into(),      "38;5;92".into()),
            ("lossless".into(),   "38;5;93".into()),
            ("crypto".into(),     "38;5;109".into()),
            ("document".into(),   "38;5;105".into()),
            ("compressed".into(), "red".into()),
            ("compiled".into(),   "38;5;137".into()),
        ]),
        patterns: HashMap::new(),
    }
}

/// Look up a style by name: config first, then compiled-in "exa".
pub fn resolve_style(name: &str) -> Option<StyleDef> {
    if let Some(ref cfg) = *CONFIG
        && let Some(s) = cfg.style.get(name) {
            return Some(s.clone());
        }
    match name {
        "exa" => Some(compiled_exa_style()),
        _ => None,
    }
}


/// The default path for `--init-config` to write to.
pub fn init_config_path() -> PathBuf {
    home_dir()
        .map(|h| h.join(".lxconfig.toml"))
        .unwrap_or_else(|| PathBuf::from(".lxconfig.toml"))
}

/// Write the default config file.
pub fn write_init_config(path: &PathBuf) -> std::io::Result<()> {
    if path.exists() {
        return Err(std::io::Error::other(
            format!("{} already exists; remove it first or edit it directly", path.display())
        ));
    }
    fs::write(path, default_config_toml())
}


/// Display the active configuration to stdout.
///
/// Shows the resolved personality, format, theme, style, and classes,
/// indicating for each whether it's compiled-in or from the config file.
pub fn show_config(personality_name: &str) {
    use nu_ansi_term::{Color, Style};

    // Styling consistent with --help: yellow bold headers, cyan bold
    // literals/names, green values/paths, dimmed for source annotations.
    let heading = Style::new().bold().fg(Color::Yellow);
    let label   = Style::new().bold();
    let name    = Style::new().bold().fg(Color::Cyan);
    let value   = Style::new().fg(Color::Green);
    let dimmed  = Style::new().dimmed();

    let config_path = find_config_path();
    let has_config = CONFIG.is_some();

    println!("{}", heading.paint("lx configuration"));
    println!();

    // Config file.
    match &config_path {
        Some(p) => println!("{} {}", label.paint("Config file:"), value.paint(p.display().to_string())),
        None    => println!("{} {}", label.paint("Config file:"), dimmed.paint("(none)")),
    }
    println!("{} {}", label.paint("Config version:"), value.paint(CONFIG_VERSION));
    println!();

    // Personality.
    println!("{} {}", label.paint("Personality:"), name.paint(personality_name));
    let source = if has_config
        && CONFIG.as_ref().unwrap().personality.contains_key(personality_name)
    {
        "config"
    } else {
        "compiled-in"
    };
    println!("  {} {}", label.paint("source:"), dimmed.paint(source));

    if let Ok(Some(p)) = resolve_personality(personality_name) {
        if let Some(ref inherits) = p.inherits {
            println!("  {} {}", label.paint("inherits:"), name.paint(inherits));
        }
        if let Some(ref fmt) = p.format {
            println!("  {} {}", label.paint("format:"), name.paint(fmt));
        }
        if let Some(ref cols) = p.columns {
            println!("  {} {}", label.paint("columns:"), value.paint(cols.to_csv()));
        }
        if !p.settings.is_empty() {
            println!("  {}",label.paint("settings:"));
            let mut keys: Vec<_> = p.settings.keys().collect();
            keys.sort();
            for key in keys {
                println!("    {} = {}", name.paint(key), value.paint(p.settings[key].to_string()));
            }
        }
    }
    println!();

    // Theme.
    let theme_name = resolve_personality(personality_name)
        .ok()
        .flatten()
        .and_then(|p| p.settings.get("theme").and_then(|v| {
            if let toml::Value::String(s) = v { Some(s.clone()) } else { None }
        }));

    if let Some(ref tname) = theme_name {
        println!("{} {}", label.paint("Theme:"), name.paint(tname));
        let source = if tname == "exa" {
            "compiled-in"
        } else if has_config && CONFIG.as_ref().unwrap().theme.contains_key(tname) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", label.paint("source:"), dimmed.paint(source));

        if tname == "exa" {
            println!("  {} {} {}", label.paint("use-style:"), name.paint("exa"), dimmed.paint("(implicit)"));
        } else {
            if let Some(ref cfg) = *CONFIG
                && let Some(theme) = cfg.theme.get(tname) {
                    if let Some(ref inherits) = theme.inherits {
                        println!("  {} {}", label.paint("inherits:"), name.paint(inherits));
                    }
                    if let Some(ref style) = theme.use_style {
                        println!("  {} {}", label.paint("use-style:"), name.paint(style));
                    }
                }
        }
    } else {
        println!("{} {}", label.paint("Theme:"), dimmed.paint("(none)"));
    }
    println!();

    // Style.
    let style_name = theme_name.as_deref().and_then(|tn| {
        if tn == "exa" {
            Some("exa".to_string())
        } else if let Some(ref cfg) = *CONFIG {
            cfg.theme.get(tn).and_then(|t| t.use_style.clone())
        } else {
            None
        }
    });

    if let Some(ref sname) = style_name {
        println!("{} {}", label.paint("Style:"), name.paint(sname));
        let source = if sname == "exa" {
            "compiled-in"
        } else if has_config && CONFIG.as_ref().unwrap().style.contains_key(sname) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", label.paint("source:"), dimmed.paint(source));

        if let Some(style) = resolve_style(sname) {
            if !style.classes.is_empty() {
                println!("  {}", label.paint("class references:"));
                let mut keys: Vec<_> = style.classes.keys().collect();
                keys.sort();
                for key in keys {
                    println!("    {} = {}", name.paint(key), value.paint(format!("\"{}\"", style.classes[key])));
                }
            }
            if !style.patterns.is_empty() {
                println!("  {}", label.paint("file patterns:"));
                let mut keys: Vec<_> = style.patterns.keys().collect();
                keys.sort();
                for key in keys {
                    println!("    {} = {}", name.paint(format!("\"{key}\"")), value.paint(format!("\"{}\"", style.patterns[key])));
                }
            }
        }
    } else {
        println!("{} {}", label.paint("Style:"), dimmed.paint("(none)"));
    }
    println!();

    // Classes.
    let classes = resolve_classes();
    println!("{} {} defined", label.paint("Classes:"), value.paint(classes.len().to_string()));
    let mut names: Vec<_> = classes.keys().collect();
    names.sort();
    for cname in names {
        let source = if has_config
            && CONFIG.as_ref().unwrap().class.contains_key(cname)
        {
            "config"
        } else {
            "compiled-in"
        };
        let patterns = &classes[cname];
        println!("  {} {}: {} patterns",
            name.paint(cname), dimmed.paint(format!("({source})")),
            value.paint(patterns.len().to_string()));
    }
    println!();

    // Formats.
    println!("{}", label.paint("Formats:"));
    let compiled = vec!["long", "long2", "long3"];
    for fname in &compiled {
        let source = if has_config
            && CONFIG.as_ref().unwrap().format.contains_key(*fname)
        {
            "config (overrides compiled-in)"
        } else {
            "compiled-in"
        };
        println!("  {}: {}", name.paint(*fname), dimmed.paint(source));
    }
    if let Some(ref cfg) = *CONFIG {
        for fname in cfg.format.keys() {
            if !compiled.contains(&fname.as_str()) {
                println!("  {}: {}", name.paint(fname), dimmed.paint("config"));
            }
        }
    }
}


// ── dump-theme ──────────────────────────────────────────────────────

/// Names of all known themes (compiled-in + config).
fn all_theme_names() -> Vec<String> {
    let mut names = vec!["exa".to_string()];
    if let Some(ref cfg) = *CONFIG {
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
    if name == "exa" {
        // The "exa" theme is compiled-in from default_theme.rs and can't
        // be round-tripped to TOML.  Show a helpful comment instead.
        return Some("# [theme.exa] is compiled-in and cannot be dumped as TOML.\n\
             # To customise, create a new theme that inherits from it:\n\
             #\n\
             # [theme.custom]\n\
             # inherits = \"exa\"\n\
             # directory = \"bold dodgerblue\"\n\
             # date = \"steelblue\"".to_string());
    }

    let cfg = CONFIG.as_ref()?;
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
pub fn dump_theme(name: &str) {
    if let Some(toml) = format_theme_toml(name) { println!("{toml}") } else {
        eprintln!("lx: unknown theme '{name}'");
        eprintln!("Known themes: {}", all_theme_names().join(", "));
        std::process::exit(3);
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

// ── dump-style ──────────────────────────────────────────────────────

/// Names of all known styles (compiled-in + config).
fn all_style_names() -> Vec<String> {
    let mut names = vec!["exa".to_string()];
    if let Some(ref cfg) = *CONFIG {
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
pub fn dump_style(name: &str) {
    if let Some(toml) = format_style_toml(name) { println!("{toml}") } else {
        eprintln!("lx: unknown style '{name}'");
        eprintln!("Known styles: {}", all_style_names().join(", "));
        std::process::exit(3);
    }
}

/// Print all style definitions as copy-pasteable TOML.
pub fn dump_style_all() {
    let names = all_style_names();
    let mut first = true;
    for name in &names {
        if let Some(toml) = format_style_toml(name) {
            if !first { println!(); }
            println!("{toml}");
            first = false;
        }
    }
}

// ── dump-personality ────────────────────────────────────────────────

/// Names of all compiled-in personalities.
const COMPILED_PERSONALITIES: &[&str] = &[
    "default", "lx", "ll", "lll", "la", "tree", "ls",
];

/// Return the names of all known personalities (compiled-in + config).
fn all_personality_names() -> Vec<String> {
    let mut names: Vec<String> = COMPILED_PERSONALITIES.iter()
        .map(|s| (*s).into())
        .collect();
    if let Some(ref cfg) = *CONFIG {
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
pub fn dump_personality(name: &str) {
    if let Some(toml) = format_personality_toml(name) { println!("{toml}") } else {
        eprintln!("lx: unknown personality '{name}'");
        eprintln!("Known personalities: {}", all_personality_names().join(", "));
        std::process::exit(3);
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

/// Format a single class definition as TOML.
fn format_class_toml(name: &str, patterns: &[String]) -> String {
    // Format as a TOML array that's readable — wrap at ~72 chars.
    let indent = " ".repeat(name.len() + 4); // align continuation lines
    let mut lines = vec![format!("{name} = [")];

    for (i, pat) in patterns.iter().enumerate() {
        let entry = format!("\"{pat}\"");
        let last = lines.last_mut().unwrap();

        if i == 0 {
            last.push_str(&entry);
        } else {
            // Would adding ", entry" exceed 72 chars?
            let trial_len = last.len() + 2 + entry.len();
            if trial_len > 72 {
                last.push(',');
                lines.push(format!("{indent}{entry}"));
            } else {
                last.push_str(", ");
                last.push_str(&entry);
            }
        }
    }
    lines.last_mut().unwrap().push(']');
    lines.join("\n")
}

/// Print a single class definition as copy-pasteable TOML.
pub fn show_class(name: &str) {
    let classes = resolve_classes();
    if let Some(patterns) = classes.get(name) {
        println!("[class]");
        println!("{}", format_class_toml(name, patterns));
    } else {
        eprintln!("lx: unknown class '{name}'");
        eprintln!("Known classes: {}", {
            let mut names: Vec<_> = classes.keys().collect();
            names.sort();
            names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        });
        std::process::exit(3);
    }
}

/// Print all class definitions as copy-pasteable TOML.
pub fn show_class_all() {
    let classes = resolve_classes();
    let mut names: Vec<_> = classes.keys().collect();
    names.sort();

    println!("[class]");
    for name in names {
        println!("{}", format_class_toml(name, &classes[name]));
    }
}

/// The compiled-in format definitions as column name strings.
fn compiled_formats() -> HashMap<String, Vec<String>> {
    HashMap::from([
        ("long".into(), vec![
            "perms".into(), "size".into(), "user".into(), "modified".into(),
        ]),
        ("long2".into(), vec![
            "perms".into(), "size".into(), "user".into(), "group".into(),
            "modified".into(), "vcs".into(),
        ]),
        ("long3".into(), vec![
            "perms".into(), "links".into(), "size".into(), "blocks".into(),
            "user".into(), "group".into(), "modified".into(), "changed".into(),
            "created".into(), "accessed".into(), "vcs".into(),
        ]),
    ])
}

/// Resolve all format definitions: compiled-in + config overrides.
/// Returns a map of format name → list of column name strings.
pub fn resolve_formats() -> HashMap<String, Vec<String>> {
    let mut formats = compiled_formats();

    // Config overrides.
    if let Some(ref cfg) = *CONFIG {
        for (name, columns) in &cfg.format {
            formats.insert(name.clone(), columns.clone());
        }
    }

    formats
}

/// Print a single format definition as copy-pasteable TOML.
pub fn show_format(name: &str) {
    let formats = resolve_formats();
    if let Some(columns) = formats.get(name) {
        println!("[format]");
        println!("{}", format_format_toml(name, columns));
    } else {
        eprintln!("lx: unknown format '{name}'");
        eprintln!("Known formats: {}", {
            let mut names: Vec<_> = formats.keys().collect();
            names.sort();
            names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        });
        std::process::exit(3);
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

/// Upgrade an older config file to the current format.
///
/// Detects the source version and applies the appropriate migration:
/// - 0.1 (unversioned): convert `[defaults]`, flatten formats, stamp 0.3
/// - 0.2: flatten `[format.NAME]` sub-tables, stamp 0.3
/// - 0.3: already current, return error
pub fn upgrade_config(path: &PathBuf) -> Result<(), ConfigError> {
    let contents = fs::read_to_string(path).with_path(path)?;
    let version = detect_config_version(&contents);

    if version == CONFIG_VERSION {
        return Err(ConfigError::AlreadyCurrent { path: path.clone() });
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


/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
