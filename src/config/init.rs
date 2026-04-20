//! `--init-config`: write the compiled-in default config file.

use std::fs;
use std::path::PathBuf;

use super::error::{ConfigError, IoResultExt};
use super::load::{default_config_toml, home_dir};

/// The default path for `--init-config` to write to.
pub fn init_config_path() -> PathBuf {
    home_dir()
        .map(|h| h.join(".lxconfig.toml"))
        .unwrap_or_else(|| PathBuf::from(".lxconfig.toml"))
}

/// Write the default config file.
///
/// # Errors
///
/// Returns `ConfigError::Io` if the target file already exists or
/// the write fails.  The path is attached as context by
/// `IoResultExt::with_path()` so the caller can produce a useful
/// error message.
pub fn write_init_config(path: &PathBuf) -> Result<(), ConfigError> {
    if path.exists() {
        // The path is added by with_path() — don't repeat it here.
        let err = std::io::Error::other("already exists; remove it first or edit it directly");
        return Err(err).with_path(path);
    }
    fs::write(path, default_config_toml()).with_path(path)
}
