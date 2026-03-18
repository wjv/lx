//! Parsing command-line strings into lx options.
//!
//! This module imports lx's configuration types, such as `View` (the details
//! of displaying multiple files) and `DirAction` (what to do when encountering
//! a directory), and implements `deduce` methods on them so they can be
//! configured using command-line options.
//!
//!
//! ## Overridden options
//!
//! Options are resolved so that the last specified flag wins.  This supports
//! shell aliases that set defaults which the user can then override on the
//! command line.  Clap's `overrides_with` handles conflicting flags natively.

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
                        return OptionsResult::Version;
                    }
                    _ => {
                        return OptionsResult::InvalidOptionsClap(e);
                    }
                }
            }
            Ok(clap_matches) => {
                let frees = clap_matches.get_many::<OsString>("FILE")
                    .map(|vals| vals.cloned().collect())
                    .unwrap_or_default();
                let flags = MatchedFlags::new(clap_matches);

                match Self::deduce(&flags, vars) {
                    Ok(options)  => OptionsResult::Ok(options, frees),
                    Err(oe)      => OptionsResult::InvalidOptions(oe),
                }
            }
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
        if cfg!(not(feature = "git")) && (matches.has(flags::GIT) || matches.has(flags::GIT_IGNORE)) {
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
    use crate::options::parser::MatchedFlags;
    use std::ffi::OsString;

    /// Parse test inputs through the full Clap command and call the given
    /// function on the resulting `MatchedFlags`.  Returns a single-element
    /// `Vec` so existing test macros that iterate over results still work.
    pub fn parse_for_test<T, F>(inputs: &[&str], get: F) -> Vec<T>
    where F: Fn(&MatchedFlags) -> T
    {
        let args: Vec<OsString> = std::iter::once(OsString::from("lx"))
            .chain(inputs.iter().map(|s| OsString::from(s)))
            .collect();
        let cmd = crate::options::parser::build_command();
        let clap_matches = cmd.try_get_matches_from(&args)
            .expect("Clap parse error in test");
        let mf = MatchedFlags::new(clap_matches);
        vec![get(&mf)]
    }
}
