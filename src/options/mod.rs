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
use crate::fs::filter::{FileFilter, VcsIgnore};
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


/// Which VCS backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsBackend {
    /// Detect automatically: prefer jj if `.jj/` exists, else git.
    Auto,
    /// Use git only.
    Git,
    /// Use jj only.
    Jj,
    /// Disable VCS integration.
    None,
}

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

    /// Which VCS backend to use for status display and ignore filtering.
    pub vcs_backend: VcsBackend,
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
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                        OptionsResult::HelpOrVersion(e)
                    }
                    _ => {
                        OptionsResult::InvalidOptionsClap(e)
                    }
                }
            }
            Ok(clap_matches) => {
                if let Some(shell) = clap_matches.get_one::<clap_complete::Shell>("completions") {
                    return OptionsResult::Completions(*shell);
                }

                if clap_matches.get_flag("show-config") {
                    return OptionsResult::ShowConfig;
                }

                if clap_matches.contains_id("dump-class")
                    && clap_matches.value_source("dump-class") == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches.get_one::<String>("dump-class")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpClass(name);
                }

                if clap_matches.contains_id("dump-format")
                    && clap_matches.value_source("dump-format") == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches.get_one::<String>("dump-format")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpFormat(name);
                }

                if clap_matches.contains_id("dump-personality")
                    && clap_matches.value_source("dump-personality") == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches.get_one::<String>("dump-personality")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpPersonality(name);
                }

                if clap_matches.contains_id("dump-theme")
                    && clap_matches.value_source("dump-theme") == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches.get_one::<String>("dump-theme")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpTheme(name);
                }

                if clap_matches.contains_id("dump-style")
                    && clap_matches.value_source("dump-style") == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches.get_one::<String>("dump-style")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpStyle(name);
                }

                if clap_matches.get_flag("init-config") {
                    return OptionsResult::InitConfig;
                }

                if clap_matches.get_flag("upgrade-config") {
                    return OptionsResult::UpgradeConfig;
                }

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
    pub fn should_scan_for_vcs(&self) -> bool {
        if self.filter.vcs_ignore == VcsIgnore::CheckAndIgnore {
            return true;
        }

        use crate::output::table::Column;
        match self.view.mode {
            Mode::Details(details::Options { table: Some(ref table), .. }) |
            Mode::GridDetails(grid_details::Options { details: details::Options { table: Some(ref table), .. }, .. }) => {
                table.columns.contains(&Column::VcsStatus)
            }
            _ => false,
        }
    }

    /// Determines the complete set of options based on the given command-line
    /// arguments, after they've been parsed.
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let wants_git = matches.has(flags::VCS_STATUS) || matches.has(flags::VCS_IGNORE)
            || matches.get(flags::VCS).is_some_and(|v| v != "none");

        if cfg!(not(feature = "git")) && wants_git {
            return Err(OptionsError::Unsupported(String::from(
                "VCS options can't be used because the `git` feature was disabled in this build of lx"
            )));
        }

        if cfg!(not(feature = "jj")) && matches.get(flags::VCS) == Some("jj") {
            return Err(OptionsError::Unsupported(String::from(
                "--vcs=jj can't be used because the `jj` feature was disabled in this build of lx"
            )));
        }

        let vcs_backend = match matches.get(flags::VCS) {
            Some("git")  => VcsBackend::Git,
            Some("jj")   => VcsBackend::Jj,
            Some("none") => VcsBackend::None,
            _            => VcsBackend::Auto,
        };

        let view = View::deduce(matches, vars)?;
        let dir_action = DirAction::deduce(matches, matches!(view.mode, Mode::Details(_)))?;
        let filter = FileFilter::deduce(matches)?;
        let theme = ThemeOptions::deduce(matches, vars)?;

        Ok(Self { dir_action, filter, view, theme, vcs_backend })
    }
}


/// The result of the `Options::parse` function.
#[derive(Debug)]
pub enum OptionsResult {

    /// The options were parsed successfully.
    Ok(Options, Vec<OsString>),

    /// There was an error in the deduce phase.
    InvalidOptions(OptionsError),

    /// Clap wants to display help or version (normal exit).
    HelpOrVersion(clap::Error),

    /// Clap detected an error in the arguments.
    InvalidOptionsClap(clap::Error),

    /// The user requested shell completions.
    Completions(clap_complete::Shell),

    /// The user wants to see the active configuration.
    ShowConfig,

    /// The user wants to see class definitions as TOML.
    /// Empty string means all classes; otherwise a specific class name.
    DumpClass(String),

    /// The user wants to see format definitions as TOML.
    DumpFormat(String),

    /// The user wants to see personality definitions as TOML.
    DumpPersonality(String),

    /// The user wants to see theme definitions as TOML.
    DumpTheme(String),

    /// The user wants to see style definitions as TOML.
    DumpStyle(String),

    /// The user wants to generate a default config file.
    InitConfig,

    /// The user wants to upgrade a legacy config file.
    UpgradeConfig,
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
