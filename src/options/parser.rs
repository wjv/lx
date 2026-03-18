//! CLI argument parsing using Clap 4.5.
//!
//! Clap handles validation, help text, and version display. After Clap
//! validates the arguments, we reconstruct an ordered list of flags for
//! the "last flag wins" and strict-mode duplicate-detection semantics
//! that the deduce functions rely on.

use std::ffi::{OsStr, OsString};
use std::fmt;

use crate::options::error::OptionsError;
use crate::options::flags;


/// A **short argument** is a single ASCII character.
pub type ShortArg = u8;

/// A **long argument** is a static string.
pub type LongArg = &'static str;

/// A **list of values** that an option can have, for display in error messages.
pub type Values = &'static [&'static str];

/// A **flag** is either a short or long argument form, as the user typed it.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Flag {
    Short(ShortArg),
    Long(LongArg),
}

impl Flag {
    pub fn matches(&self, arg: &Arg) -> bool {
        match self {
            Self::Short(short)  => arg.short == Some(*short),
            Self::Long(long)    => arg.long == *long,
        }
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Short(short)  => write!(f, "-{}", *short as char),
            Self::Long(long)    => write!(f, "--{}", long),
        }
    }
}


/// Whether redundant arguments should be considered a problem.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Strictness {
    /// Throw an error when an argument doesn't do anything, either because
    /// it requires another argument to be specified, or because two conflict.
    ComplainAboutRedundantArguments,

    /// Search the arguments list back-to-front, giving ones specified later
    /// in the list priority over earlier ones.
    UseLastArguments,
}


/// Whether a flag takes a value.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TakesValue {
    /// This flag must be followed by a value.
    Necessary(Option<Values>),

    /// This flag must not be followed by a value.
    Forbidden,

    /// This flag may optionally be followed by a value.
    Optional(Option<Values>),
}


/// An **argument** definition, used for flag identity and error messages.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Arg {
    /// The short argument that matches it, if any.
    pub short: Option<ShortArg>,

    /// The long argument that matches it.
    pub long: LongArg,

    /// Whether this flag takes a value or not.
    pub takes_value: TakesValue,
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "--{}", self.long)?;
        if let Some(short) = self.short {
            write!(f, " (-{})", short as char)?;
        }
        Ok(())
    }
}


// ---- Clap command builder ----

/// Build the Clap command from the flag definitions.
pub fn build_command() -> clap::Command {
    use clap::{Arg as ClapArg, ArgAction};

    let mut cmd = clap::Command::new("lx")
        .version(include_str!(concat!(env!("OUT_DIR"), "/version_string.txt")))
        .about("list extended (but call me Alex!)")
        .disable_help_flag(true)
        .disable_version_flag(true);

    for arg_def in flags::ALL_ARGS {
        // Help and version are handled by dedicated Clap actions below.
        if arg_def.long == "help" || arg_def.long == "version" {
            continue;
        }

        let mut clap_arg = ClapArg::new(arg_def.long)
            .long(arg_def.long);

        if let Some(short) = arg_def.short {
            clap_arg = clap_arg.short(short as char);
        }

        match arg_def.takes_value {
            TakesValue::Forbidden => {
                clap_arg = clap_arg.action(ArgAction::Count);
            }
            TakesValue::Necessary(_) => {
                clap_arg = clap_arg
                    .action(ArgAction::Append)
                    .value_parser(clap::value_parser!(OsString))
                    .num_args(1);
            }
            TakesValue::Optional(_) => {
                clap_arg = clap_arg
                    .action(ArgAction::Append)
                    .value_parser(clap::value_parser!(OsString))
                    .num_args(0..=1);
            }
        }

        cmd = cmd.arg(clap_arg);
    }

    // Help flag — Clap handles display and exit.
    cmd = cmd.arg(
        ClapArg::new("help")
            .short('?')
            .long("help")
            .action(ArgAction::HelpShort)
    );

    // Version flag — Clap handles display and exit.
    cmd = cmd.arg(
        ClapArg::new("version")
            .short('v')
            .long("version")
            .action(ArgAction::Version)
    );

    // Positional file arguments — everything that isn't a flag.
    cmd = cmd.arg(
        ClapArg::new("FILE")
            .action(ArgAction::Append)
            .value_parser(clap::value_parser!(OsString))
            .num_args(0..)
    );

    cmd
}


// ---- Matches and MatchedFlags ----

/// The result of parsing the user's command-line strings.
#[derive(Debug)]
pub struct Matches {
    /// The flags that were parsed from the user's input.
    pub flags: MatchedFlags,

    /// All the strings that weren't matched as arguments, as well as anything
    /// after the special "--" string.
    pub frees: Vec<OsString>,
}

#[derive(Debug)]
pub struct MatchedFlags {
    /// The individual flags from the user's input, in the order they were
    /// originally given. Long and short arguments are kept in the same
    /// vector because we usually want the one nearest the end to count.
    flags: Vec<(Flag, Option<OsString>)>,

    /// Whether to check for duplicate or redundant arguments.
    strictness: Strictness,
}

impl MatchedFlags {
    /// Whether the given argument was specified.
    pub fn has(&self, arg: &'static Arg) -> Result<bool, OptionsError> {
        self.has_where(|flag| flag.matches(arg))
            .map(|flag| flag.is_some())
    }

    /// Returns the last found argument that satisfies the predicate (among
    /// boolean flags only), or nothing if none is found, or an error in
    /// strict mode if multiple arguments satisfy the predicate.
    pub fn has_where<P>(&self, predicate: P) -> Result<Option<Flag>, OptionsError>
    where P: Fn(&Flag) -> bool {
        if self.is_strict() {
            let all = self.flags.iter()
                          .filter(|tuple| tuple.1.is_none() && predicate(&tuple.0))
                          .collect::<Vec<_>>();

            if all.len() < 2 { Ok(all.first().map(|t| t.0)) }
                        else { Err(OptionsError::Duplicate(all[0].0, all[1].0)) }
        }
        else {
            Ok(self.has_where_any(predicate))
        }
    }

    /// Returns the last found argument that satisfies the predicate (among
    /// boolean flags only), ignoring strict mode.
    pub fn has_where_any<P>(&self, predicate: P) -> Option<Flag>
    where P: Fn(&Flag) -> bool {
        self.flags.iter().rev()
            .find(|tuple| tuple.1.is_none() && predicate(&tuple.0))
            .map(|tuple| tuple.0)
    }

    /// Returns the value of the given argument if it was specified, nothing
    /// if it wasn't, and an error in strict mode if it was specified more
    /// than once.
    pub fn get(&self, arg: &'static Arg) -> Result<Option<&OsStr>, OptionsError> {
        self.get_where(|flag| flag.matches(arg))
    }

    /// Returns the value of the argument that matches the predicate.
    pub fn get_where<P>(&self, predicate: P) -> Result<Option<&OsStr>, OptionsError>
    where P: Fn(&Flag) -> bool {
        if self.is_strict() {
            let those = self.flags.iter()
                            .filter(|tuple| tuple.1.is_some() && predicate(&tuple.0))
                            .collect::<Vec<_>>();

            if those.len() < 2 { Ok(those.first().map(|t| t.1.as_deref().unwrap())) }
                          else { Err(OptionsError::Duplicate(those[0].0, those[1].0)) }
        }
        else {
            let found = self.flags.iter().rev()
                            .find(|tuple| tuple.1.is_some() && predicate(&tuple.0))
                            .and_then(|tuple| tuple.1.as_deref());
            Ok(found)
        }
    }

    /// Counts the number of occurrences of the given argument.
    pub fn count(&self, arg: &Arg) -> usize {
        self.flags.iter()
            .filter(|tuple| tuple.0.matches(arg))
            .count()
    }

    /// Checks whether strict mode is on.
    pub fn is_strict(&self) -> bool {
        self.strictness == Strictness::ComplainAboutRedundantArguments
    }

    /// Constructor for tests that build MatchedFlags directly.
    #[cfg(test)]
    pub fn new_for_test(flags: Vec<(Flag, Option<OsString>)>, strictness: Strictness) -> Self {
        Self { flags, strictness }
    }
}


// ---- Flag reconstruction from raw args ----
//
// After Clap validates the arguments, we scan the raw args to reconstruct
// the ordered flag list that the deduce functions rely on. Since Clap has
// already validated everything, this scan needs no error handling.

/// Parse validated command-line arguments into an ordered flag list plus free args.
///
/// Call this only after `build_command().try_get_matches_from()` has succeeded,
/// so all arguments are known to be valid.
pub fn reconstruct_matches(raw_args: &[OsString], strictness: Strictness) -> Matches {
    let mut result_flags = Vec::new();
    let mut frees = Vec::new();
    let mut parsing = true;
    let mut iter = raw_args.iter().peekable();

    while let Some(arg) = iter.next() {
        let bytes = os_str_to_bytes(arg);

        if !parsing {
            frees.push(arg.clone());
        }
        else if arg == "--" {
            parsing = false;
        }
        else if bytes.starts_with(b"--") {
            let long_arg = &bytes[2..];

            if let Some(eq_pos) = long_arg.iter().position(|&b| b == b'=') {
                let name_bytes = &long_arg[..eq_pos];
                let value_bytes = &long_arg[eq_pos + 1..];
                if let Some(arg_def) = lookup_long(name_bytes) {
                    let flag = Flag::Long(arg_def.long);
                    let value = bytes_to_os_str(value_bytes).to_os_string();
                    result_flags.push((flag, Some(value)));
                }
            }
            else if let Some(arg_def) = lookup_long(long_arg) {
                let flag = Flag::Long(arg_def.long);
                match arg_def.takes_value {
                    TakesValue::Forbidden => {
                        result_flags.push((flag, None));
                    }
                    TakesValue::Necessary(_) => {
                        // Clap guarantees the next arg exists.
                        let value = iter.next().unwrap().clone();
                        result_flags.push((flag, Some(value)));
                    }
                    TakesValue::Optional(_) => {
                        // Take the next arg as a value if it exists and doesn't
                        // look like a flag. Since Clap validated, we trust it.
                        if let Some(next) = iter.peek() {
                            let next_bytes = os_str_to_bytes(next);
                            if !next_bytes.starts_with(b"-") || next_bytes == b"-" {
                                let value = iter.next().unwrap().clone();
                                result_flags.push((flag, Some(value)));
                            } else {
                                result_flags.push((flag, None));
                            }
                        } else {
                            result_flags.push((flag, None));
                        }
                    }
                }
            }
        }
        else if bytes.starts_with(b"-") && bytes != b"-" {
            // Short args: one or more in a cluster.
            for (index, &byte) in bytes.iter().enumerate().skip(1) {
                if let Some(arg_def) = lookup_short(byte) {
                    let flag = Flag::Short(byte);
                    match arg_def.takes_value {
                        TakesValue::Forbidden => {
                            result_flags.push((flag, None));
                        }
                        TakesValue::Necessary(_) | TakesValue::Optional(_) => {
                            // Check for =value or remaining bytes as value.
                            if index + 1 < bytes.len() {
                                let rest = if bytes[index + 1] == b'=' {
                                    &bytes[index + 2..]
                                } else {
                                    &bytes[index + 1..]
                                };
                                let value = bytes_to_os_str(rest).to_os_string();
                                result_flags.push((flag, Some(value)));
                            }
                            else if let Some(next) = iter.next() {
                                result_flags.push((flag, Some(next.clone())));
                            }
                            else {
                                // Optional with no value.
                                result_flags.push((flag, None));
                            }
                            break; // Rest consumed as value.
                        }
                    }
                }
            }
        }
        else {
            frees.push(arg.clone());
        }
    }

    Matches {
        frees,
        flags: MatchedFlags { flags: result_flags, strictness },
    }
}


/// Look up a long argument name in the known flags.
fn lookup_long(name_bytes: &[u8]) -> Option<&'static Arg> {
    // All our long arg names are ASCII, so this comparison is safe.
    flags::ALL_ARGS.iter().copied().find(|a| a.long.as_bytes() == name_bytes)
}

/// Look up a short argument character in the known flags.
fn lookup_short(byte: u8) -> Option<&'static Arg> {
    flags::ALL_ARGS.iter().copied().find(|a| a.short == Some(byte))
}


#[cfg(unix)]
fn os_str_to_bytes(s: &OsStr) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes()
}

#[cfg(unix)]
fn bytes_to_os_str(b: &[u8]) -> &OsStr {
    use std::os::unix::ffi::OsStrExt;
    OsStr::from_bytes(b)
}

#[cfg(windows)]
fn os_str_to_bytes(s: &OsStr) -> &[u8] {
    s.to_str().unwrap().as_bytes()
}

#[cfg(windows)]
fn bytes_to_os_str(b: &[u8]) -> &OsStr {
    use std::str;
    OsStr::new(str::from_utf8(b).unwrap())
}


#[cfg(test)]
mod matches_test {
    use super::*;

    static VERBOSE: Arg = Arg { short: Some(b'v'), long: "verbose", takes_value: TakesValue::Forbidden };
    static COUNT:   Arg = Arg { short: Some(b'c'), long: "count",   takes_value: TakesValue::Necessary(None) };

    fn make_flags(flags: Vec<(Flag, Option<OsString>)>) -> MatchedFlags {
        MatchedFlags {
            flags,
            strictness: Strictness::UseLastArguments,
        }
    }

    #[test]
    fn short_never() {
        let flags = make_flags(vec![]);
        assert_eq!(flags.has(&VERBOSE), Ok(false));
    }

    #[test]
    fn short_once() {
        let flags = make_flags(vec![(Flag::Short(b'v'), None)]);
        assert_eq!(flags.has(&VERBOSE), Ok(true));
    }

    #[test]
    fn short_twice() {
        let flags = make_flags(vec![(Flag::Short(b'v'), None), (Flag::Short(b'v'), None)]);
        assert_eq!(flags.has(&VERBOSE), Ok(true));
    }

    #[test]
    fn long_once() {
        let flags = make_flags(vec![(Flag::Long("verbose"), None)]);
        assert_eq!(flags.has(&VERBOSE), Ok(true));
    }

    #[test]
    fn long_twice() {
        let flags = make_flags(vec![(Flag::Long("verbose"), None), (Flag::Long("verbose"), None)]);
        assert_eq!(flags.has(&VERBOSE), Ok(true));
    }

    #[test]
    fn long_mixed() {
        let flags = make_flags(vec![(Flag::Long("verbose"), None), (Flag::Short(b'v'), None)]);
        assert_eq!(flags.has(&VERBOSE), Ok(true));
    }

    #[test]
    fn only_count() {
        let flags = make_flags(vec![(Flag::Short(b'c'), Some(OsString::from("everything")))]);
        assert_eq!(flags.get(&COUNT), Ok(Some(OsStr::new("everything"))));
    }

    #[test]
    fn rightmost_count() {
        let flags = make_flags(vec![
            (Flag::Short(b'c'), Some(OsString::from("everything"))),
            (Flag::Short(b'c'), Some(OsString::from("nothing"))),
        ]);
        assert_eq!(flags.get(&COUNT), Ok(Some(OsStr::new("nothing"))));
    }

    #[test]
    fn no_count() {
        let flags = make_flags(vec![]);
        assert!(!flags.has(&COUNT).unwrap());
    }
}
