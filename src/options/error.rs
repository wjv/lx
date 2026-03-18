use std::fmt;
use std::num::ParseIntError;


/// Something wrong with the combination of options the user has picked.
#[derive(PartialEq, Eq, Debug)]
pub enum OptionsError {

    /// The user supplied a set of options that are unsupported.
    Unsupported(String),

    /// A very specific edge case where --tree can't be used with --all twice.
    TreeAllAll,

    /// A numeric option was given that failed to be parsed as a number.
    FailedParse(String, NumberSource, ParseIntError),

    /// A glob ignore was given that failed to be parsed as a pattern.
    FailedGlobPattern(String),
}

/// The source of a string that failed to be parsed as a number.
#[derive(PartialEq, Eq, Debug)]
pub enum NumberSource {

    /// It came from an environment variable.
    Env(&'static str),
}

impl From<glob::PatternError> for OptionsError {
    fn from(error: glob::PatternError) -> Self {
        Self::FailedGlobPattern(error.to_string())
    }
}

impl fmt::Display for NumberSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env(env) => write!(f, "environment variable {}", env),
        }
    }
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(e)             => write!(f, "{}", e),
            Self::TreeAllAll                 => write!(f, "Option --tree is useless given --all --all"),
            Self::FailedParse(s, n, e)       => write!(f, "Value {:?} not valid for {}: {}", s, n, e),
            Self::FailedGlobPattern(e)       => write!(f, "Failed to parse glob pattern: {}", e),
        }
    }
}
