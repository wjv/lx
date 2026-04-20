//! Errors that can arise while loading or resolving configuration.

use std::path::PathBuf;

use thiserror::Error;

use super::schema::CONFIG_VERSION;

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
    #[error(
        "config file {path} uses version {version} format.\n\
             Run `lx --upgrade-config` to migrate it to version {CONFIG_VERSION}."
    )]
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

    /// `--upgrade-config` invoked with no config file in scope.
    #[error("no config file found to upgrade")]
    NothingToUpgrade,

    /// A `--dump-*` or `--show-class`/`--show-format` lookup that
    /// references a name we don't know.  `kind` is the singular
    /// user-facing noun ("theme", "personality", ...) and
    /// `kind_plural` is its plural form (passed explicitly so
    /// "personality" → "personalities" works); `candidates` is a
    /// pre-joined comma-separated list of the names that *would*
    /// have worked.
    #[error("unknown {kind} '{name}'\nKnown {kind_plural}: {candidates}")]
    NotFound {
        kind: &'static str,
        kind_plural: &'static str,
        name: String,
        candidates: String,
    },
}

/// Extension trait for attaching path context to `io::Result`.
///
/// Used throughout the `config` submodules to wrap raw `std::io::Error`
/// values into `ConfigError::Io { path, source }` so the caller knows
/// *which* file the error came from.
pub(super) trait IoResultExt<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T, ConfigError>;
}

impl<T> IoResultExt<T> for std::io::Result<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> Result<T, ConfigError> {
        self.map_err(|source| ConfigError::Io {
            path: path.into(),
            source,
        })
    }
}
