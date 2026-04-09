//! Process-wide storage for the loaded configuration.
//!
//! `init_config()` is called once at startup from `try_main()`; from
//! that point on, all read sites use `config()` to get the loaded
//! `Config` (or `None` if no config file exists).

use std::sync::OnceLock;

use super::error::ConfigError;
use super::load::try_load_config;
use super::schema::Config;


/// Storage for the user's configuration.
///
/// Populated once by `init_config()` early in `try_main()`, then read
/// through `config()` for the rest of the process lifetime.  The
/// outer `Option` represents "no config file exists" (which is fine —
/// lx falls back to compiled defaults), while errors during loading
/// surface as `Err` from `init_config()` rather than being silently
/// swallowed.
static CONFIG_STORE: OnceLock<Option<Config>> = OnceLock::new();

/// Load the user's configuration file.
///
/// Called once from `main` before any other config-reading code runs.
/// Errors here are fatal — the pre-0.9 graceful fallback for broken
/// configs (eprintln + continue with compiled defaults) is gone.
///
/// # Errors
///
/// Returns whatever `try_load_config()` returns: `ConfigError::Io`
/// for unreadable files, `ConfigError::Parse` for invalid TOML,
/// `ConfigError::NeedsUpgrade` for old-format files, etc.
pub fn init_config() -> Result<(), ConfigError> {
    let loaded = try_load_config()?;
    // OnceLock::set fails only if it has already been set; we don't
    // need a second init, so silently ignore.
    let _ = CONFIG_STORE.set(loaded);
    Ok(())
}

/// Read the loaded configuration.
///
/// Returns `None` if no config file exists, or if `init_config()`
/// has not yet been called (e.g. during `--upgrade-config`, which
/// loads the file directly).
#[must_use]
pub fn config() -> Option<&'static Config> {
    CONFIG_STORE.get().and_then(Option::as_ref)
}
