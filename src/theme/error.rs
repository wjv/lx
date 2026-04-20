//! Errors raised while resolving the active theme.

use thiserror::Error;

/// Things that can go wrong when applying a configured theme.
///
/// `apply_style()` and `apply_theme_def()` continue to use `warn!`
/// for best-effort recovery (bad globs, unknown style references) —
/// these are not user errors, just hints we ignore.  Only the
/// "I can't even find the theme" path becomes a hard error.
#[derive(Debug, Error)]
pub enum ThemeError {
    /// `--theme=NAME` (or a name reached via inheritance) does not
    /// match a built-in theme or a `[theme.NAME]` config section.
    #[error("unknown theme '{name}'")]
    Unknown { name: String },

    /// A theme `inherits = "..."` chain forms a cycle.
    ///
    /// Previously these were `warn!`-and-continue.  In 0.9 they are
    /// fatal: a cycle is unambiguously a broken config and silently
    /// dropping it hides real bugs.
    #[error("theme inheritance cycle: {chain}")]
    Cycle { chain: String },
}
