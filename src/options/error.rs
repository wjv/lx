use std::fmt;
use std::num::ParseIntError;

use thiserror::Error;

/// Something wrong with the combination of options the user has picked.
#[derive(PartialEq, Eq, Debug, Error)]
pub enum OptionsError {
    /// The user supplied a set of options that are unsupported.
    #[error("{0}")]
    Unsupported(String),

    /// A very specific edge case where --tree can't be used with --all twice.
    #[error(
        "the argument '--tree' cannot be used with '--all --all' \
             (listing '.' and '..' in tree mode would recurse forever)"
    )]
    TreeAllAll,

    /// A numeric option was given that failed to be parsed as a number.
    #[error("Value {0:?} not valid for {1}: {2}")]
    FailedParse(String, NumberSource, #[source] ParseIntError),

    /// A glob ignore was given that failed to be parsed as a pattern.
    ///
    /// Stored as a `String` rather than the original `glob::PatternError`
    /// so that `OptionsError` can keep its `PartialEq`/`Eq` derives
    /// (the underlying error type does not implement them).
    #[error("Failed to parse glob pattern: {0}")]
    FailedGlobPattern(String),
}

/// The source of a string that failed to be parsed as a number.
#[derive(PartialEq, Eq, Debug)]
pub enum NumberSource {
    /// It came from an environment variable.
    Env(&'static str),

    /// It came from a CLI argument or config setting.
    Arg(&'static str),
}

impl From<glob::PatternError> for OptionsError {
    fn from(error: glob::PatternError) -> Self {
        Self::FailedGlobPattern(error.to_string())
    }
}

impl fmt::Display for NumberSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env(env) => write!(f, "environment variable {env}"),
            Self::Arg(arg) => write!(f, "argument --{arg}"),
        }
    }
}
