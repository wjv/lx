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
use crate::output::{Mode, View, details, grid_details};
use crate::theme::Options as ThemeOptions;

mod dir_action;
mod file_name;
mod filter;
pub(crate) mod flags;
mod theme;
mod view;

mod error;
pub use self::error::{NumberSource, OptionsError};

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

    /// Whether to print the total count of items listed (to stderr).
    pub count: bool,
}

impl Options {
    /// Parse the given iterator of command-line strings into an Options
    /// struct and a list of free filenames, using the environment variables
    /// for extra options.
    #[allow(unused_results)]
    pub fn parse<V>(args: &[OsString], vars: &V) -> OptionsResult
    where
        V: Vars,
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
                    _ => OptionsResult::InvalidOptionsClap(e),
                }
            }
            Ok(clap_matches) => {
                if let Some(shell) = clap_matches.get_one::<clap_complete::Shell>("completions") {
                    return OptionsResult::Completions(*shell);
                }

                if clap_matches.contains_id("show-config")
                    && clap_matches.value_source("show-config")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let mode = match clap_matches
                        .get_one::<String>("show-config")
                        .map(String::as_str)
                    {
                        Some("full") => ShowConfigMode::Full,
                        Some("available") => ShowConfigMode::Available,
                        _ => ShowConfigMode::Active,
                    };
                    // Detect the implicit `-l` tier so `--show-config`
                    // can show the effective format when no personality
                    // declares one.  An explicit `--columns` or
                    // `--format` overrides the tier in `deduce_columns`,
                    // so we suppress the hint when either is present.
                    let long_count = clap_matches.get_count("long");
                    let has_columns = clap_matches
                        .value_source(flags::COLUMNS)
                        .is_some_and(|src| src == clap::parser::ValueSource::CommandLine);
                    let has_format = clap_matches
                        .value_source(flags::FORMAT)
                        .is_some_and(|src| src == clap::parser::ValueSource::CommandLine);
                    let implicit_format = if long_count > 0 && !has_columns && !has_format {
                        Some(
                            match long_count {
                                1 => "long",
                                2 => "long2",
                                _ => "long3",
                            }
                            .to_string(),
                        )
                    } else {
                        None
                    };
                    return OptionsResult::ShowConfig {
                        mode,
                        implicit_format,
                    };
                }

                if clap_matches.contains_id("dump-class")
                    && clap_matches.value_source("dump-class")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches
                        .get_one::<String>("dump-class")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpClass(name);
                }

                if clap_matches.contains_id("dump-format")
                    && clap_matches.value_source("dump-format")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches
                        .get_one::<String>("dump-format")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpFormat(name);
                }

                if clap_matches.contains_id("dump-personality")
                    && clap_matches.value_source("dump-personality")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches
                        .get_one::<String>("dump-personality")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpPersonality(name);
                }

                if clap_matches.contains_id("dump-theme")
                    && clap_matches.value_source("dump-theme")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches
                        .get_one::<String>("dump-theme")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpTheme(name);
                }

                if clap_matches.contains_id("dump-style")
                    && clap_matches.value_source("dump-style")
                        == Some(clap::parser::ValueSource::CommandLine)
                {
                    let name = clap_matches
                        .get_one::<String>("dump-style")
                        .cloned()
                        .unwrap_or_default();
                    return OptionsResult::DumpStyle(name);
                }

                if let Some(name) = clap_matches.get_one::<String>("save-as") {
                    let settings = Self::extract_cli_settings(&clap_matches);
                    return OptionsResult::SaveAs(
                        name.clone(),
                        None, // inherits — set by main.rs from active personality
                        settings,
                    );
                }

                // Was `--show-as`/`--show` given at all?  With
                // `num_args(0..=1)`, get_one returns None both when
                // the flag is absent *and* when it's present without
                // a value, so we have to check the value source
                // separately.
                let show_present = clap_matches
                    .value_source("show-as")
                    .is_some_and(|s| s == clap::parser::ValueSource::CommandLine);
                if show_present {
                    let raw_name = clap_matches.get_one::<String>("show-as").cloned();
                    // Empty-string value (`--show-as=`) is treated
                    // as no name: keep-calm-and-carry-on rather than
                    // erroring on an obvious typo.
                    let name = raw_name.filter(|s| !s.is_empty());
                    let settings = Self::extract_cli_settings(&clap_matches);
                    return OptionsResult::ShowAs(
                        name, None, // inherits — set by main.rs from active personality
                        settings,
                    );
                }

                if clap_matches.get_flag("init-config") {
                    return OptionsResult::InitConfig;
                }

                if clap_matches.get_flag("upgrade-config") {
                    return OptionsResult::UpgradeConfig;
                }

                let frees = clap_matches
                    .get_many::<OsString>("FILE")
                    .map(|vals| vals.cloned().collect())
                    .unwrap_or_default();
                let flags = MatchedFlags::new(clap_matches);

                match Self::deduce(&flags, vars) {
                    Ok(options) => OptionsResult::Ok(Box::new(options), frees),
                    Err(oe) => OptionsResult::InvalidOptions(oe),
                }
            }
        }
    }

    /// Extract settings that were explicitly passed on the CLI, as a
    /// config-key → TOML-value map suitable for writing a personality file.
    /// Only captures flags the user actually typed, not personality defaults.
    fn extract_cli_settings(
        matches: &clap::ArgMatches,
    ) -> std::collections::HashMap<String, toml::Value> {
        let mut settings = collect_flag_values(matches);
        expand_time_tiers(matches, &mut settings);
        expand_all_count(matches, &mut settings);
        expand_extended_count(matches, &mut settings);
        rewrite_negations(&mut settings);
        dedup_aliases(&mut settings);
        settings
    }
}

/// Iterate `SETTING_FLAGS` and collect values the user explicitly
/// passed on the command line.
fn collect_flag_values(
    matches: &clap::ArgMatches,
) -> std::collections::HashMap<String, toml::Value> {
    use crate::config::{SETTING_FLAGS, SettingKind};
    use std::collections::HashMap;

    let mut settings: HashMap<String, toml::Value> = HashMap::new();

    for def in SETTING_FLAGS {
        let clap_id = match def.key {
            "permissions" => flags::SHOW_PERMISSIONS,
            "size" | "filesize" => flags::SHOW_SIZE,
            "user" => flags::SHOW_USER,
            "colour" | "color" => flags::COLOR,
            "colour-scale" | "color-scale" => flags::COLOR_SCALE,
            "ignore" => flags::IGNORE_GLOB,
            _ => def.flag.strip_prefix("--").unwrap_or(def.flag),
        };

        if settings.contains_key(def.key) {
            continue;
        }
        if matches.value_source(clap_id) != Some(clap::parser::ValueSource::CommandLine) {
            continue;
        }

        let value = match def.kind {
            SettingKind::Bool => toml::Value::Boolean(true),
            SettingKind::Str => {
                if let Some(v) = matches.get_one::<String>(clap_id) {
                    toml::Value::String(v.clone())
                } else {
                    continue;
                }
            }
            SettingKind::Int => {
                if let Some(v) = matches.get_one::<i64>(clap_id) {
                    toml::Value::Integer(*v)
                } else if let Some(v) = matches.get_one::<String>(clap_id) {
                    if let Ok(n) = v.parse::<i64>() {
                        toml::Value::Integer(n)
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        };
        settings.insert(def.key.to_string(), value);
    }

    settings
}

/// Expand compounding `-a` (`ALL`) count >= 2 into `dot-entries = true`.
fn expand_all_count(
    matches: &clap::ArgMatches,
    settings: &mut std::collections::HashMap<String, toml::Value>,
) {
    if matches.value_source(flags::ALL) != Some(clap::parser::ValueSource::CommandLine) {
        return;
    }
    if matches.get_count(flags::ALL) >= 2 {
        settings.insert("dot-entries".into(), toml::Value::Boolean(true));
    }
}

/// Expand compounding `-@` (`EXTENDED`) count.
///
/// Count 1 (`-@`) sets `xattr-indicator = true`.  Count 2 (`-@@`)
/// also sets `extended = true`.  Mirrors `expand_all_count`.
fn expand_extended_count(
    matches: &clap::ArgMatches,
    settings: &mut std::collections::HashMap<String, toml::Value>,
) {
    if matches.value_source(flags::EXTENDED) != Some(clap::parser::ValueSource::CommandLine) {
        return;
    }
    let count = matches.get_count(flags::EXTENDED);
    if count >= 1 {
        settings.insert("xattr-indicator".into(), toml::Value::Boolean(true));
    }
    if count < 2 {
        // Count 1 means *indicator only* — strip the `extended`
        // entry that `collect_flag_values` would have inserted
        // on the basis of "value-source is command-line".
        settings.remove("extended");
    }
}

/// Expand compounding `-t` (`TIME_TIER`) into individual timestamp booleans.
fn expand_time_tiers(
    matches: &clap::ArgMatches,
    settings: &mut std::collections::HashMap<String, toml::Value>,
) {
    if matches.value_source(flags::TIME_TIER) != Some(clap::parser::ValueSource::CommandLine) {
        return;
    }
    let count = matches.get_count(flags::TIME_TIER);
    if count >= 1 {
        settings.insert("modified".into(), toml::Value::Boolean(true));
    }
    if count >= 2 {
        settings.insert("changed".into(), toml::Value::Boolean(true));
    }
    if count >= 3 {
        settings.insert("created".into(), toml::Value::Boolean(true));
        settings.insert("accessed".into(), toml::Value::Boolean(true));
    }
}

/// Rewrite `no-X = true` as `X = false` when the positive counterpart
/// is a Bool setting.  With three-state Bool semantics, `X = false`
/// emits `--no-X` at load time, so the saved personality is correct
/// and reads more naturally.
fn rewrite_negations(settings: &mut std::collections::HashMap<String, toml::Value>) {
    use crate::config::{SettingKind, find_setting};

    let neg_keys: Vec<String> = settings
        .keys()
        .filter(|k| k.starts_with("no-"))
        .cloned()
        .collect();
    for neg_key in neg_keys {
        let pos_key = neg_key.strip_prefix("no-").unwrap().to_string();
        if let Some(pos_def) = find_setting(&pos_key)
            && matches!(pos_def.kind, SettingKind::Bool)
        {
            settings.remove(&neg_key);
            settings.insert(pos_key, toml::Value::Boolean(false));
        }
    }
}

/// Remove backward-compat alias keys when the canonical key for the
/// same setting is also present.
fn dedup_aliases(settings: &mut std::collections::HashMap<String, toml::Value>) {
    for (canonical, alias) in [
        ("size", "filesize"),
        ("total", "total-size"),
        ("octal", "octal-permissions"),
    ] {
        if settings.contains_key(canonical) {
            settings.remove(alias);
        }
    }
}

impl Options {
    /// Whether the View specified in this set of options includes a Git
    /// status column. It's only worth trying to discover a repository if the
    /// results will end up being displayed.
    pub fn should_scan_for_vcs(&self) -> bool {
        use crate::output::table::Column;

        if self.filter.vcs_ignore == VcsIgnore::CheckAndIgnore {
            return true;
        }

        match self.view.mode {
            Mode::Details(details::Options {
                table: Some(ref table),
                ..
            })
            | Mode::GridDetails(grid_details::Options {
                details:
                    details::Options {
                        table: Some(ref table),
                        ..
                    },
                ..
            }) => table.columns.contains(&Column::VcsStatus),
            _ => false,
        }
    }

    /// Determines the complete set of options based on the given command-line
    /// arguments, after they've been parsed.
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let wants_git = matches.has(flags::VCS_STATUS)
            || matches.has(flags::VCS_IGNORE)
            || matches.get(flags::VCS).is_some_and(|v| v != "none");

        if cfg!(not(feature = "git")) && wants_git {
            return Err(OptionsError::Unsupported(String::from(
                "VCS options can't be used because the `git` feature was disabled in this build of lx",
            )));
        }

        if cfg!(not(feature = "jj")) && matches.get(flags::VCS) == Some("jj") {
            return Err(OptionsError::Unsupported(String::from(
                "--vcs=jj can't be used because the `jj` feature was disabled in this build of lx",
            )));
        }

        let vcs_backend = match matches.get(flags::VCS) {
            Some("git") => VcsBackend::Git,
            Some("jj") => VcsBackend::Jj,
            Some("none") => VcsBackend::None,
            _ => VcsBackend::Auto,
        };

        let view = View::deduce(matches, vars)?;
        let dir_action = DirAction::deduce(matches, matches!(view.mode, Mode::Details(_)));
        let filter = FileFilter::deduce(matches)?;
        let theme = ThemeOptions::deduce(matches, vars);
        let count = matches.has(flags::COUNT) && !matches.has(flags::NO_COUNT);

        Ok(Self {
            dir_action,
            filter,
            view,
            theme,
            vcs_backend,
            count,
        })
    }
}

/// Which sections of `--show-config` to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowConfigMode {
    /// Just the active half: what's currently running.  The default
    /// when `--show-config` is given without an explicit mode.
    Active,
    /// Both halves: active + the available catalogue, separated by
    /// a horizontal rule.
    Full,
    /// Just the catalogue half: every defined personality, format,
    /// theme, style, and class.
    Available,
}

/// The result of the `Options::parse` function.
#[derive(Debug)]
pub enum OptionsResult {
    /// The options were parsed successfully.
    Ok(Box<Options>, Vec<OsString>),

    /// There was an error in the deduce phase.
    InvalidOptions(OptionsError),

    /// Clap wants to display help or version (normal exit).
    HelpOrVersion(clap::Error),

    /// Clap detected an error in the arguments.
    InvalidOptionsClap(clap::Error),

    /// The user requested shell completions.
    Completions(clap_complete::Shell),

    /// The user wants to see the active configuration.
    ///
    /// `mode` selects how much to show: just the active half
    /// ("what's running"), the full output including the
    /// available catalogue, or the catalogue alone.
    ///
    /// `implicit_format` carries the `-l` tier name (`"long"`,
    /// `"long2"`, `"long3"`) when the user invoked `--show-config`
    /// alongside `-l` and neither `--columns` nor `--format` would
    /// override it.  `--show-config` displays this as the effective
    /// format when the active personality declares neither
    /// `format` nor `columns`.
    ShowConfig {
        mode: ShowConfigMode,
        implicit_format: Option<String>,
    },

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

    /// The user wants to save CLI flags as a personality.
    /// Contains: (name, inherits, settings as TOML key/value pairs).
    SaveAs(
        String,
        Option<String>,
        std::collections::HashMap<String, toml::Value>,
    ),

    /// The user wants to preview CLI flags as a personality on stdout.
    /// Contains: (name, inherits, settings as TOML key/value pairs).
    /// `name` is `None` when `--show` or bare `--show-as` was used
    /// (anonymous preview); the emitter renders the snippet
    /// fully commented out under `[personality.UNNAMED]`.
    ShowAs(
        Option<String>,
        Option<String>,
        std::collections::HashMap<String, toml::Value>,
    ),
}

#[cfg(test)]
pub mod test {
    use crate::options::parser::MatchedFlags;
    use std::ffi::OsString;

    /// Parse test inputs through the full Clap command and call the given
    /// function on the resulting `MatchedFlags`.  Returns a single-element
    /// `Vec` so existing test macros that iterate over results still work.
    pub fn parse_for_test<T, F>(inputs: &[&str], get: F) -> Vec<T>
    where
        F: Fn(&MatchedFlags) -> T,
    {
        let args: Vec<OsString> = std::iter::once(OsString::from("lx"))
            .chain(inputs.iter().map(|s| OsString::from(s)))
            .collect();
        let cmd = crate::options::parser::build_command();
        let clap_matches = cmd
            .try_get_matches_from(&args)
            .expect("Clap parse error in test");
        let mf = MatchedFlags::new(clap_matches);
        vec![get(&mf)]
    }
}
