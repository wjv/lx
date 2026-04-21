//! Config file discovery, loading, and the [[when]] support functions.
//!
//! lx looks for a TOML config file in these locations (first found wins):
//!
//! 1. `$LX_CONFIG` — explicit path override
//! 2. `~/.lxconfig.toml` — simple home directory location
//! 3. `$XDG_CONFIG_HOME/lx/config.toml` (default `~/.config/lx/config.toml`)
//! 4. `~/Library/Application Support/lx/config.toml` (macOS only)
//!
//! Drop-in fragments live in `conf.d/` next to the main file and are
//! merged on top in alphabetical order.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use log::*;

use super::error::{ConfigError, IoResultExt};
use super::schema::{ACCEPTED_VERSIONS, Config};

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
        .unwrap_or_else(|_| home_dir().map(|h| h.join(".config")).unwrap_or_default());
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

/// Find the drop-in config directory.
///
/// The drop-in directory sits alongside the main config file:
/// - `~/.lxconfig.toml` → `~/.config/lx/conf.d/` (XDG standard)
/// - `$XDG_CONFIG_HOME/lx/config.toml` → `$XDG_CONFIG_HOME/lx/conf.d/`
/// - macOS: `~/Library/Application Support/lx/conf.d/`
///
/// When `LX_CONFIG` is set, the drop-in directory is its parent's
/// `conf.d/` subdirectory, or the XDG location if the config is a
/// standalone file.
pub(super) fn find_drop_in_dir(main_config: Option<&Path>) -> Option<PathBuf> {
    // If the main config is in a directory, look for conf.d/ there.
    if let Some(config_path) = main_config
        && let Some(parent) = config_path.parent()
    {
        let d = parent.join("conf.d");
        if d.is_dir() {
            return Some(d);
        }
    }

    // Also check the XDG location (covers ~/.lxconfig.toml users).
    let xdg = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().map(|h| h.join(".config")).unwrap_or_default());
    let d = xdg.join("lx").join("conf.d");
    if d.is_dir() {
        return Some(d);
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = home_dir() {
        let d = home.join("Library/Application Support/lx/conf.d");
        if d.is_dir() {
            return Some(d);
        }
    }

    None
}

/// Load sorted `*.toml` fragments from a drop-in directory.
fn load_drop_ins(dir: &Path) -> Vec<(PathBuf, Config)> {
    let mut entries: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
            .collect(),
        Err(e) => {
            warn!("conf.d: failed to read {}: {e}", dir.display());
            return Vec::new();
        }
    };
    entries.sort();

    let mut fragments = Vec::new();
    for path in entries {
        match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(cfg) => {
                    debug!("conf.d: loaded {}", path.display());
                    fragments.push((path, cfg));
                }
                Err(e) => {
                    warn!("conf.d: parse error in {}: {e}", path.display());
                }
            },
            Err(e) => {
                warn!("conf.d: failed to read {}: {e}", path.display());
            }
        }
    }
    fragments
}

/// Load and parse the config file, if one is found.
///
/// Returns `Ok(None)` if no config file exists.  Returns a typed
/// `ConfigError` on I/O failures, parse errors, or legacy format.
pub(super) fn try_load_config() -> Result<Option<Config>, ConfigError> {
    let config_path = find_config_path();

    // Load the main config file, if any.
    let mut config = if let Some(ref path) = config_path {
        let contents = fs::read_to_string(path).with_path(path)?;

        let version = detect_config_version(&contents);
        if !ACCEPTED_VERSIONS.contains(&version) {
            return Err(ConfigError::NeedsUpgrade {
                path: path.clone(),
                version: version.to_string(),
            });
        }

        let cfg: Config = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.clone(),
            source,
        })?;

        // Warn if when blocks are used but version is still 0.3.
        if version == "0.3" {
            let has_when = cfg.personality.values().any(|p| !p.when.is_empty());
            if has_when {
                eprintln!("lx: config has [[personality.*.when]] blocks but version is \"0.3\".");
                eprintln!("    Change version to \"0.4\" to enable conditional config.");
            }
        }

        info!("Loaded config from {}", path.display());
        Some(cfg)
    } else {
        None
    };

    // Load drop-in fragments from conf.d/.
    if let Some(drop_in_dir) = find_drop_in_dir(config_path.as_deref()) {
        let fragments = load_drop_ins(&drop_in_dir);
        if !fragments.is_empty() {
            let config = config.get_or_insert_with(Config::default);
            for (path, fragment) in fragments {
                config.drop_in_paths.push(path);
                config.merge(fragment);
            }
        }
    }

    Ok(config)
}

/// Detect the config schema version from raw file contents.
///
/// Returns the version string, or `"0.1"` if no version field
/// is found (legacy config).
pub(super) fn detect_config_version(contents: &str) -> &str {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") && trimmed.contains('=') {
            // Extract the value after '=', stripping quotes and whitespace.
            if let Some(val) = trimmed.split('=').nth(1) {
                let val = val.trim().trim_matches('"');
                return match val {
                    "0.2" => "0.2",
                    "0.3" => "0.3",
                    "0.4" => "0.4",
                    _ => val, // unknown version — will fail the check
                };
            }
        }
    }
    "0.1" // no version field → legacy
}

/// The default config as a commented TOML string, for `--init-config`.
pub fn default_config_toml() -> &'static str {
    include_str!("../../lxconfig.default.toml")
}

// ── [[when]] block support ──────────────────────────────────────

/// Inject the auto-selection `[[when]]` blocks into a config file's
/// `[personality.default]` section.  Used by `--upgrade-config` from
/// 0.5 to 0.6.  Idempotent: callers should check that the blocks
/// are not already present before calling.
pub(super) fn inject_auto_select_blocks(contents: &str) -> String {
    const BLOCKS: &str = "\n\
## Auto-select a richer theme on capable terminals (added by\n\
## lx --upgrade-config to 0.6).  Delete or edit to opt out.\n\
\n\
[[personality.default.when]]\n\
env.TERM = \"*-256color\"\n\
theme = \"lx-256\"\n\
\n\
[[personality.default.when]]\n\
env.COLORTERM = [\"truecolor\", \"24bit\"]\n\
theme = \"lx-24bit\"\n\
\n";

    // Find the end of the [personality.default] section: the next
    // line that starts with [ (a new section) or end of file.
    let lines: Vec<&str> = contents.lines().collect();
    let mut in_default = false;
    let mut insert_after: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[personality.default]") {
            in_default = true;
            continue;
        }
        if in_default && (trimmed.starts_with('[') && !trimmed.starts_with("[[")) {
            // Next top-level section — end of [personality.default].
            insert_after = Some(i);
            break;
        }
        if in_default && trimmed.starts_with("[[") {
            // Already a [[ block in [personality.default] — bail
            // (we don't want to insert in the middle).
            insert_after = Some(i);
            break;
        }
    }

    let insert_at = match insert_after {
        Some(i) => i,
        None if in_default => lines.len(),
        None => return contents.to_string(), // no [personality.default]
    };

    let mut result = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == insert_at {
            result.push_str(BLOCKS);
        }
        result.push_str(line);
        result.push('\n');
    }
    if insert_at == lines.len() {
        result.push_str(BLOCKS);
    }

    // Preserve trailing newline if present in original.
    if !contents.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Match a single value against an expected string, treating the
/// expected string as a glob pattern if it contains glob metacharacters.
pub(super) fn match_string(actual: &str, expected: &str) -> bool {
    if expected.contains(['*', '?', '[']) {
        glob::Pattern::new(expected).is_ok_and(|pat| pat.matches(actual))
    } else {
        actual == expected
    }
}

// ── home_dir helper ─────────────────────────────────────────────

/// Get the user's home directory.
pub(super) fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod when_match_test {
    use super::match_string;

    #[test]
    fn literal_match() {
        assert!(match_string("truecolor", "truecolor"));
        assert!(!match_string("truecolor", "24bit"));
    }

    #[test]
    fn empty_actual_matches_empty_expected() {
        assert!(match_string("", ""));
    }

    #[test]
    fn glob_star_suffix() {
        assert!(match_string("xterm-256color", "*-256color"));
        assert!(match_string("screen-256color", "*-256color"));
        assert!(match_string("rxvt-unicode-256color", "*-256color"));
        assert!(!match_string("xterm", "*-256color"));
        assert!(!match_string("xterm-direct", "*-256color"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(match_string("foo", "fo?"));
        assert!(!match_string("foobar", "fo?"));
    }

    #[test]
    fn glob_bracket_range() {
        assert!(match_string("file1", "file[0-9]"));
        assert!(!match_string("filea", "file[0-9]"));
    }

    #[test]
    fn invalid_glob_falls_back_to_no_match() {
        // An unmatched [ is invalid as a glob; we don't crash, just
        // fail to match.
        assert!(!match_string("foo[bar", "foo[bar"));
    }
}
