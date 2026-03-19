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
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use log::*;
use serde::Deserialize;


/// Global config, loaded once at startup.
pub static CONFIG: LazyLock<Option<Config>> = LazyLock::new(load_config);


/// Top-level config file structure.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub defaults: Defaults,

    #[serde(default)]
    pub formats: HashMap<String, FormatDef>,

    #[serde(default)]
    pub personalities: HashMap<String, PersonalityDef>,

    // theme: deferred to a later iteration
}

/// Default settings applied to every invocation unless overridden by
/// CLI flags or environment variables.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Defaults {
    pub colour: Option<String>,
    pub colour_scale: Option<String>,
    pub time_style: Option<String>,
    pub group_dirs: Option<String>,
    pub icons: Option<String>,
    pub classify: Option<String>,
}

impl Defaults {
    /// Convert defaults into CLI args that can be prepended before
    /// the real arguments.  Clap's `args_override_self` ensures that
    /// any explicit CLI flag overrides these.
    pub fn to_args(&self) -> Vec<std::ffi::OsString> {
        let mut args = Vec::new();

        if let Some(ref v) = self.colour {
            args.push(format!("--colour={v}").into());
        }
        if let Some(ref v) = self.colour_scale {
            args.push(format!("--colour-scale={v}").into());
        }
        if let Some(ref v) = self.time_style {
            args.push(format!("--time-style={v}").into());
        }
        if let Some(ref v) = self.group_dirs {
            args.push(format!("--group-dirs={v}").into());
        }
        if let Some(ref v) = self.icons {
            args.push(format!("--icons={v}").into());
        }
        // classify is long-form only, no ArgAction::Set currently —
        // defer until =WHEN vocabulary is standardised (#11)

        args
    }
}


/// A named column layout.
#[derive(Debug, Deserialize)]
pub struct FormatDef {
    pub columns: Vec<String>,
}

/// A personality bundles columns, flags, and settings.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PersonalityDef {
    /// Reference to a named format (looked up in `[formats]`).
    pub format: Option<String>,

    /// Inline column list (overrides `format` if both given).
    pub columns: Option<Vec<String>>,

    /// CLI flags to prepend (e.g. `["--group-dirs=first", "--header"]`).
    pub flags: Option<Vec<String>>,

    /// Override time-style for this personality.
    pub time_style: Option<String>,

    /// Override header setting for this personality.
    pub header: Option<bool>,
}


/// Search for a config file and return its path, or `None`.
pub fn find_config_path() -> Option<PathBuf> {
    // 1. Explicit env var
    if let Ok(path) = env::var("LX_CONFIG") {
        let p = PathBuf::from(path);
        if p.is_file() {
            debug!("Config from LX_CONFIG: {}", p.display());
            return Some(p);
        }
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
/// Returns `None` if no config file exists.
/// Prints an error and returns `None` if the file exists but can't be parsed.
pub fn load_config() -> Option<Config> {
    let path = find_config_path()?;

    match fs::read_to_string(&path) {
        Ok(contents) => {
            match toml::from_str(&contents) {
                Ok(config) => {
                    info!("Loaded config from {}", path.display());
                    Some(config)
                }
                Err(e) => {
                    eprintln!("lx: error parsing {}: {e}", path.display());
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("lx: error reading {}: {e}", path.display());
            None
        }
    }
}


/// The default config as a commented TOML string, for `--init-config`.
pub fn default_config_toml() -> &'static str {
    include_str!("../lxconfig.default.toml")
}


/// Look up a personality by name: config first, then compiled-in defaults.
pub fn resolve_personality(name: &str) -> Option<PersonalityDef> {
    // Config-defined personalities take priority.
    if let Some(ref cfg) = *CONFIG {
        if let Some(p) = cfg.personalities.get(name) {
            return Some(PersonalityDef {
                format: p.format.clone(),
                columns: p.columns.clone(),
                flags: p.flags.clone(),
                time_style: p.time_style.clone(),
                header: p.header,
            });
        }
    }

    // Compiled-in personalities.
    match name {
        "ll" => Some(PersonalityDef {
            format: Some("long2".into()),
            flags: Some(vec!["--group-dirs=first".into()]),
            ..Default::default()
        }),
        "lll" => Some(PersonalityDef {
            format: Some("long3".into()),
            flags: Some(vec!["--group-dirs=first".into(), "--header".into()]),
            time_style: Some("long-iso".into()),
            ..Default::default()
        }),
        "la" => Some(PersonalityDef {
            format: Some("long2".into()),
            flags: Some(vec!["--all".into(), "--group-dirs=first".into()]),
            ..Default::default()
        }),
        "tree" => Some(PersonalityDef {
            format: Some("long2".into()),
            flags: Some(vec!["--tree".into(), "--group-dirs=first".into()]),
            ..Default::default()
        }),
        "ls" => Some(PersonalityDef {
            flags: Some(vec!["--grid".into(), "--across".into()]),
            ..Default::default()
        }),
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


/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
