use std::ffi::OsString;
use std::fmt;
use std::num::ParseIntError;

use crate::options::flags;


/// Something wrong with the combination of options the user has picked.
#[derive(PartialEq, Eq, Debug)]
pub enum OptionsError {

    /// The user supplied an illegal choice to an argument.
    BadArgument(&'static str, OsString),

    /// The user supplied a set of options that are unsupported.
    Unsupported(String),

    /// Two options were given that conflict with one another.
    Conflict(&'static str, &'static str),

    /// An option was given that does nothing when another one either is or
    /// isn't present.
    Useless(&'static str, bool, &'static str),

    /// An option was given that does nothing when either of two other options
    /// are not present.
    Useless2(&'static str, &'static str, &'static str),

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

    /// It came from a command-line argument.
    Arg(&'static str),

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
            Self::Arg(arg) => write!(f, "option --{}", arg),
            Self::Env(env) => write!(f, "environment variable {}", env),
        }
    }
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadArgument(arg, attempt) => {
                write!(f, "Option --{} has no {:?} setting", arg, attempt)
            }
            Self::Unsupported(e)             => write!(f, "{}", e),
            Self::Conflict(a, b)             => write!(f, "Option --{} conflicts with option --{}", a, b),
            Self::Useless(a, false, b)       => write!(f, "Option --{} is useless without option --{}", a, b),
            Self::Useless(a, true, b)        => write!(f, "Option --{} is useless given option --{}", a, b),
            Self::Useless2(a, b1, b2)        => write!(f, "Option --{} is useless without options --{} or --{}", a, b1, b2),
            Self::TreeAllAll                 => write!(f, "Option --tree is useless given --all --all"),
            Self::FailedParse(s, n, e)       => write!(f, "Value {:?} not valid for {}: {}", s, n, e),
            Self::FailedGlobPattern(e)       => write!(f, "Failed to parse glob pattern: {}", e),
        }
    }
}

impl OptionsError {

    /// Try to second-guess what the user was trying to do, depending on what
    /// went wrong.
    pub fn suggestion(&self) -> Option<&'static str> {
        // 'ls -lt' and 'ls -ltr' are common combinations
        match self {
            Self::BadArgument(time, r) if *time == flags::TIME && r == "r" => {
                Some("To sort oldest files last, try \"--sort oldest\", or just \"-sold\"")
            }
            _ => {
                None
            }
        }
    }
}
