//! Parsing command-line strings into lx options.
//!
//! This module imports lx's configuration types, such as `View` (the details
//! of displaying multiple files) and `DirAction` (what to do when encountering
//! a directory), and implements `deduce` methods on them so they can be
//! configured using command-line options.
//!
//!
//! ## Useless and overridden options
//!
//! Options are resolved right-to-left: the last specified flag wins.  This
//! supports shell aliases that set defaults which the user can then override
//! on the command line.  In strict mode (`EXA_STRICT` set), duplicate or
//! redundant flags are reported as errors instead.

use std::ffi::OsString;

use crate::fs::dir_action::DirAction;
use crate::fs::filter::{FileFilter, GitIgnore};
use crate::output::{View, Mode, details, grid_details};
use crate::theme::Options as ThemeOptions;

mod dir_action;
mod file_name;
mod filter;
mod flags;
mod theme;
mod view;

mod error;
pub use self::error::{OptionsError, NumberSource};

pub mod parser;
use self::parser::MatchedFlags;

pub mod vars;
pub use self::vars::Vars;


/// These **options** represent a parsed, error-checked versions of the
/// user's command-line options.
#[derive(Debug)]
pub struct Options {

    /// The action to perform when encountering a directory rather than a
    /// regular file.
    pub dir_action: DirAction,

    /// How to sort and filter files before outputting them.
    pub filter: FileFilter,

    /// The user's preference of view to use (lines, grid, details, or
    /// grid-details) along with the options on how to render file names.
    /// If the view requires the terminal to have a width, and there is no
    /// width, then the view will be downgraded.
    pub view: View,

    /// The options to make up the styles of the UI and file names.
    pub theme: ThemeOptions,
}

impl Options {

    /// Parse the given iterator of command-line strings into an Options
    /// struct and a list of free filenames, using the environment variables
    /// for extra options.
    #[allow(unused_results)]
    pub fn parse<V>(args: &[OsString], vars: &V) -> OptionsResult
    where V: Vars,
    {
        use crate::options::parser::Strictness;

        let strictness = match vars.get(vars::EXA_STRICT) {
            None                         => Strictness::UseLastArguments,
            Some(ref t) if t.is_empty()  => Strictness::UseLastArguments,
            Some(_)                      => Strictness::ComplainAboutRedundantArguments,
        };

        // Use Clap for validation, help, and version.
        // try_get_matches_from expects the binary name as the first argument.
        let mut clap_args = vec![OsString::from("lx")];
        clap_args.extend_from_slice(args);
        let cmd = parser::build_command();
        match cmd.try_get_matches_from(&clap_args) {
            Err(e) => {
                use clap::error::ErrorKind;
                match e.kind() {
                    ErrorKind::DisplayHelp => {
                        return OptionsResult::HelpOrVersion(e);
                    }
                    ErrorKind::DisplayVersion => {
                        // Use our own version string instead of Clap's.
                        return OptionsResult::Version;
                    }
                    _ => {
                        return OptionsResult::InvalidOptionsClap(e);
                    }
                }
            }
            Ok(_clap_matches) => {
                // Clap validated — now reconstruct ordered flags for deduce.
            }
        }

        // Reconstruct the flag list from raw args for ordering and strict mode.
        let parser::Matches { flags, frees } =
            parser::reconstruct_matches(args, strictness);

        match Self::deduce(&flags, vars) {
            Ok(options)  => OptionsResult::Ok(options, frees),
            Err(oe)      => OptionsResult::InvalidOptions(oe),
        }
    }

    /// Whether the View specified in this set of options includes a Git
    /// status column. It's only worth trying to discover a repository if the
    /// results will end up being displayed.
    pub fn should_scan_for_git(&self) -> bool {
        if self.filter.git_ignore == GitIgnore::CheckAndIgnore {
            return true;
        }

        match self.view.mode {
            Mode::Details(details::Options { table: Some(ref table), .. }) |
            Mode::GridDetails(grid_details::Options { details: details::Options { table: Some(ref table), .. }, .. }) => table.columns.git,
            _ => false,
        }
    }

    /// Determines the complete set of options based on the given command-line
    /// arguments, after they've been parsed.
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        if cfg!(not(feature = "git")) &&
                matches.has_where_any(|f| f.matches(&flags::GIT) || f.matches(&flags::GIT_IGNORE)).is_some() {
            return Err(OptionsError::Unsupported(String::from(
                "Options --git and --git-ignore can't be used because `git` feature was disabled in this build of lx"
            )));
        }

        let view = View::deduce(matches, vars)?;
        let dir_action = DirAction::deduce(matches, matches!(view.mode, Mode::Details(_)))?;
        let filter = FileFilter::deduce(matches)?;
        let theme = ThemeOptions::deduce(matches, vars)?;

        Ok(Self { dir_action, filter, view, theme })
    }
}


/// The result of the `Options::parse` function.
#[derive(Debug)]
pub enum OptionsResult {

    /// The options were parsed successfully.
    Ok(Options, Vec<OsString>),

    /// There was an error in the deduce phase.
    InvalidOptions(OptionsError),

    /// Clap wants to display help (normal exit).
    HelpOrVersion(clap::Error),

    /// Display our custom version string.
    Version,

    /// Clap detected an error in the arguments.
    InvalidOptionsClap(clap::Error),
}


#[cfg(test)]
pub mod test {
    use crate::options::parser::{Arg, Strictness, MatchedFlags, Flag};
    use std::ffi::OsString;

    #[derive(PartialEq, Eq, Debug)]
    pub enum Strictnesses {
        Last,
        Complain,
        Both,
    }

    /// This function gets used by the other testing modules.
    /// It can run with one or both strictness values: if told to run with
    /// both, then both should resolve to the same result.
    ///
    /// It returns a vector with one or two elements in.
    /// These elements can then be tested with `assert_eq` or what have you.
    pub fn parse_for_test<T, F>(inputs: &[&str], args: &'static [&'static Arg], strictnesses: Strictnesses, get: F) -> Vec<T>
    where F: Fn(&MatchedFlags) -> T
    {
        use self::Strictnesses::*;
        use crate::options::parser::TakesValue;

        let bits: Vec<OsString> = inputs.iter().map(|s| OsString::from(s)).collect();
        let mut result = Vec::new();

        // Build a mini Clap command for just these test args, to avoid
        // depending on the full ALL_ARGS set in unit tests.
        // But actually it's simpler to just reconstruct directly, since
        // reconstruct_matches does the right thing and only looks up args
        // it recognises.  We create a temporary ALL_ARGS-like lookup by
        // just using the reconstruct logic with our test arg definitions.
        //
        // For test isolation, we bypass Clap validation entirely and call
        // the reconstruction directly.  This is safe because tests only
        // supply well-formed inputs.

        if strictnesses == Last || strictnesses == Both {
            let mf = parse_test_flags(&bits, args, Strictness::UseLastArguments);
            result.push(get(&mf));
        }

        if strictnesses == Complain || strictnesses == Both {
            let mf = parse_test_flags(&bits, args, Strictness::ComplainAboutRedundantArguments);
            result.push(get(&mf));
        }

        result
    }

    /// Parse test flags using a mini flag reconstruction that only knows
    /// about the given arg definitions.
    fn parse_test_flags(args: &[OsString], arg_defs: &[&'static Arg], strictness: Strictness) -> MatchedFlags {
        use crate::options::parser::TakesValue;

        let mut flags: Vec<(Flag, Option<OsString>)> = Vec::new();
        let mut parsing = true;
        let mut iter = args.iter().peekable();

        fn lookup_long_in<'a>(name: &[u8], defs: &[&'a Arg]) -> Option<&'a Arg> {
            defs.iter().find(|a| a.long.as_bytes() == name).copied()
        }

        fn lookup_short_in<'a>(byte: u8, defs: &[&'a Arg]) -> Option<&'a Arg> {
            defs.iter().find(|a| a.short == Some(byte)).copied()
        }

        #[cfg(unix)]
        fn to_bytes(s: &OsString) -> &[u8] {
            use std::os::unix::ffi::OsStrExt;
            s.as_bytes()
        }

        #[cfg(windows)]
        fn to_bytes(s: &OsString) -> &[u8] {
            s.to_str().unwrap().as_bytes()
        }

        while let Some(arg) = iter.next() {
            let bytes = to_bytes(arg);

            if !parsing {
                // free arg — ignored in tests
            }
            else if arg == "--" {
                parsing = false;
            }
            else if bytes.starts_with(b"--") {
                let long_arg = &bytes[2..];

                if let Some(eq_pos) = long_arg.iter().position(|&b| b == b'=') {
                    let name = &long_arg[..eq_pos];
                    let value = &long_arg[eq_pos + 1..];
                    if let Some(def) = lookup_long_in(name, arg_defs) {
                        let flag = Flag::Long(def.long);
                        #[cfg(unix)]
                        let val = {
                            use std::os::unix::ffi::OsStrExt;
                            std::ffi::OsStr::from_bytes(value).to_os_string()
                        };
                        #[cfg(windows)]
                        let val = OsString::from(std::str::from_utf8(value).unwrap());
                        flags.push((flag, Some(val)));
                    }
                }
                else if let Some(def) = lookup_long_in(long_arg, arg_defs) {
                    let flag = Flag::Long(def.long);
                    match def.takes_value {
                        TakesValue::Forbidden => flags.push((flag, None)),
                        TakesValue::Necessary(_) => {
                            let value = iter.next().unwrap().clone();
                            flags.push((flag, Some(value)));
                        }
                        TakesValue::Optional(_) => {
                            if let Some(next) = iter.next() {
                                flags.push((flag, Some(next.clone())));
                            } else {
                                flags.push((flag, None));
                            }
                        }
                    }
                }
            }
            else if bytes.starts_with(b"-") && bytes != b"-" {
                for (index, &byte) in bytes.iter().enumerate().skip(1) {
                    if let Some(def) = lookup_short_in(byte, arg_defs) {
                        let flag = Flag::Short(byte);
                        match def.takes_value {
                            TakesValue::Forbidden => flags.push((flag, None)),
                            TakesValue::Necessary(_) | TakesValue::Optional(_) => {
                                if index + 1 < bytes.len() {
                                    let rest = if bytes[index + 1] == b'=' {
                                        &bytes[index + 2..]
                                    } else {
                                        &bytes[index + 1..]
                                    };
                                    #[cfg(unix)]
                                    let val = {
                                        use std::os::unix::ffi::OsStrExt;
                                        std::ffi::OsStr::from_bytes(rest).to_os_string()
                                    };
                                    #[cfg(windows)]
                                    let val = OsString::from(std::str::from_utf8(rest).unwrap());
                                    flags.push((flag, Some(val)));
                                }
                                else if let Some(next) = iter.next() {
                                    flags.push((flag, Some(next.clone())));
                                }
                                else {
                                    flags.push((flag, None));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        MatchedFlags::new_for_test(flags, strictness)
    }
}
