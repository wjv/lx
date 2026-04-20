#![warn(deprecated_in_future)]
#![warn(future_incompatible)]
#![warn(nonstandard_style)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused)]

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::non_ascii_literal)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unused_self)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::wildcard_imports)]

use std::env;
use std::ffi::OsString;
use std::io::{self, Write, ErrorKind};
use std::path::{Component, PathBuf};

use nu_ansi_term::{AnsiStrings, Style};

use log::*;

use crate::config::ConfigError;
use crate::fs::{Dir, File};
use crate::fs::feature::git::GitCache;
use crate::fs::feature::jj::JjCache;
use crate::fs::feature::VcsCache;
use crate::fs::filter::VcsIgnore;
use crate::options::{Options, OptionsError, VcsBackend, Vars, vars, OptionsResult};
use crate::output::{escape, lines, grid, grid_details, details, View, Mode};
use crate::theme::{Theme, ThemeError};

mod config;
mod fs;
mod logger;
pub(crate) mod options;
mod output;
mod theme;


/// Top-level error type for the `lx` binary.
///
/// Wraps the per-module error types so that fallible call sites in
/// `try_main()` can use the `?` operator across module boundaries.
/// New module errors are added here as the refactor progresses.
#[derive(Debug, thiserror::Error)]
pub enum LxError {
    #[error("{0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Options(#[from] OptionsError),

    #[error("{0}")]
    Theme(#[from] ThemeError),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl LxError {
    /// Map an `LxError` to the process exit code that should accompany it.
    fn exit_code(&self) -> i32 {
        match self {
            Self::Options(_)
            | Self::Theme(_)
            | Self::Config(ConfigError::InheritanceCycle { .. }
                          | ConfigError::MissingParent { .. }
                          | ConfigError::NotFound { .. }) => exits::OPTIONS_ERROR,
            _ => exits::RUNTIME_ERROR,
        }
    }
}


fn main() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    logger::configure(env::var_os(vars::LX_DEBUG));

    #[cfg(windows)]
    if let Err(e) = nu_ansi_term::enable_ansi_support() {
        warn!("Failed to enable ANSI support: {}", e);
    }

    match try_main() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{} {e}", error_label());
            std::process::exit(e.exit_code());
        }
    }
}


/// Format the "error:" label the way clap does: bold red when stderr is
/// a terminal and `NO_COLOR` is unset, plain otherwise.  Keeps our
/// non-clap fatal errors visually consistent with clap's own output.
fn error_label() -> &'static str {
    use std::io::IsTerminal;

    if env::var_os("NO_COLOR").is_none() && io::stderr().is_terminal() {
        "\x1b[1m\x1b[31merror:\x1b[0m"
    } else {
        "error:"
    }
}


/// Discover personality names for shell-completion alias registration
/// by scanning `$PATH` for symlinks that resolve to the same binary
/// as the running process.  Returns a sorted, deduplicated list;
/// `lx` itself is excluded (it already has completions from the
/// primary `generate()` call).
///
/// No compiled-in fallback: registering completions for names like
/// `tree` or `ls` without checking whether the user has actually
/// symlinked them to `lx` would shadow the *real* `tree(1)` / `ls(1)`
/// completions.  Only names that genuinely resolve to our binary are
/// safe to register.
fn personality_completion_names() -> Vec<String> {
    let (Ok(our_exe), Some(path_var)) = (
        std::env::current_exe().and_then(|p| p.canonicalize()),
        std::env::var_os("PATH"),
    ) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for dir in std::env::split_paths(&path_var) {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            // Only check symlinks — skip regular files and dirs.
            let Ok(meta) = path.symlink_metadata() else { continue };
            if !meta.is_symlink() { continue }
            // Does this symlink resolve to our binary?
            let Ok(target) = path.canonicalize() else { continue };
            if target != our_exe { continue }
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name != "lx"
                && !names.iter().any(|n| n == name)
            {
                names.push(name.to_string());
            }
        }
    }

    names.sort();
    names
}


/// Fallible body of `main()`.  Returns the desired process exit code on
/// success or a top-level `LxError` to be reported by the wrapper.
fn try_main() -> Result<i32, LxError> {
    let cli_args: Vec<OsString> = env::args_os().skip(1).collect();

    // Skip config loading for --upgrade-config — the file may be in
    // an older schema version, which would error out of init_config()
    // and prevent the upgrade from running.  upgrade_config() reads
    // the file directly via try_load_config() and bypasses the
    // CONFIG_STORE altogether.
    let upgrading = cli_args.iter().any(|a| a == "--upgrade-config");
    if !upgrading {
        config::init_config()?;
    }

    // Build the arg list in layers; Clap's args_override_self ensures
    // later flags win: personality < CLI flags.
    let mut args: Vec<OsString> = Vec::new();

    // Layer 1: personality (skip when upgrading — config may be legacy).
    let mut active_personality: Option<String> = None;
    // Tracks how the personality was chosen (for --show-config).
    let mut personality_source = "default";
    if !upgrading {
        // Personality resolution: -p → argv[0] → $LX_PERSONALITY → "lx".
        let explicit_personality = find_personality_arg(&cli_args);
        let from_env = env::var("LX_PERSONALITY").ok().filter(|s| !s.is_empty());
        // Error on unknown name when the user asked explicitly (-p or env var).
        let explicit = explicit_personality.is_some() || from_env.is_some();

        let personality_name = if let Some(name) = explicit_personality {
            personality_source = "-p";
            debug!("-p dispatch: {name}");
            Some(name)
        } else if let Some(bin_name) = env::args().next().and_then(|argv0| {
            let name = std::path::Path::new(&argv0)
                .file_name()?
                .to_string_lossy()
                .to_string();
            // "lx" is the binary's own name, not a deliberate symlink —
            // skip it so $LX_PERSONALITY can take effect.
            if name == "lx" {
                return None;
            }
            match config::resolve_personality(&name) {
                Ok(Some(_)) | Err(_) => Some(name),
                Ok(None) => None,
            }
        }) {
            personality_source = "argv[0]";
            debug!("argv[0] dispatch: {bin_name}");
            Some(bin_name)
        } else if let Some(name) = from_env {
            personality_source = "$LX_PERSONALITY";
            debug!("$LX_PERSONALITY: {name}");
            Some(name)
        } else {
            None
        };

        // Apply the resolved personality, falling back to "lx" if no
        // earlier stage matched.
        let personality_name = personality_name.unwrap_or_else(|| "lx".to_string());
        match config::resolve_personality(&personality_name)? {
            Some(personality) => {
                args.extend(personality.to_args());
                active_personality = Some(personality_name.clone());
            }
            None if personality_source == "$LX_PERSONALITY" => {
                // Env var named a personality that doesn't exist —
                // produce our NotFound error.  (-p / --personality is
                // handled by clap's value_parser on the PERSONALITY
                // flag and produces a clap-native error there.)
                return Err(LxError::Config(config::ConfigError::NotFound {
                    kind: "personality",
                    kind_plural: "personalities",
                    name: personality_name,
                    candidates: config::all_personality_names().join(", "),
                }));
            }
            None => {}  // -p (clap will catch it) or argv[0] (silent default)
        }
        // `explicit` is now only consulted via personality_source above.
        let _ = explicit;
    }

    // Layer 3: actual CLI args (override everything above).
    args.extend(cli_args.iter().cloned());


    match Options::parse(&args, &LiveVars) {
        OptionsResult::Ok(options, mut input_paths) => {

            // List the current directory by default.
            // (This has to be done here, otherwise git_options won't see it.)
            if input_paths.is_empty() {
                input_paths = vec![ OsString::from(".") ];
            }

            let vcs = vcs_cache(&options, &input_paths);
            let writer = io::stdout();

            let console_width = options.view.width.actual_terminal_width();
            let theme = options.theme.to_theme(console_width.is_some())?;

            // Build a locale::Numeric with personality overrides applied.
            let mut numeric = locale::Numeric::load_user_locale()
                .unwrap_or_else(|_| locale::Numeric::english());
            if let Some(ref dp) = options.view.table_options().and_then(|t| t.decimal_point.clone()) {
                numeric.decimal_sep.clone_from(dp);
            }
            if let Some(ref ts) = options.view.table_options().and_then(|t| t.thousands_separator.clone()) {
                numeric.thousands_sep.clone_from(ts);
            }

            let lx = Lx { options, writer, input_paths, theme, console_width, vcs, item_count: 0, size_total: 0, numeric };

            match lx.run() {
                Ok(exit_status) => return Ok(exit_status),

                // Broken pipe is normal for `lx | head` etc. — quietly
                // exit 0 instead of treating it as an error.
                Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                    warn!("Broken pipe error: {e}");
                    return Ok(exits::SUCCESS);
                }

                Err(e) => return Err(LxError::from(e)),
            }
        }

        OptionsResult::HelpOrVersion(clap_err) => {
            clap_err.exit();
        }

        OptionsResult::InvalidOptionsClap(clap_err) => {
            clap_err.exit();
        }

        OptionsResult::Completions(shell) => {
            use std::io::Write;

            let mut cmd = crate::options::parser::build_command();
            let mut out = io::stdout();
            clap_complete::generate(shell, &mut cmd, "lx", &mut out);

            // Register the same completions for personality names that
            // users symlink to `lx`.  Start with the compiled-in names
            // (so completions work before symlinks are created), then
            // discover any extra symlinks pointing to this binary in
            // $PATH (catches user-defined personalities).
            let names = personality_completion_names();
            if !names.is_empty() {
                let joined = names.join(" ");
                match shell {
                    clap_complete::Shell::Bash => {
                        let _ = write!(out, "\n\
                            if [[ \"${{BASH_VERSINFO[0]}}\" -eq 4 && \
                            \"${{BASH_VERSINFO[1]}}\" -ge 4 || \
                            \"${{BASH_VERSINFO[0]}}\" -gt 4 ]]; then\n    \
                            complete -F _lx -o nosort -o bashdefault -o default {joined}\n\
                            else\n    \
                            complete -F _lx -o bashdefault -o default {joined}\n\
                            fi\n");
                    }
                    clap_complete::Shell::Zsh => {
                        let _ = writeln!(out, "\ncompdef _lx {joined}");
                    }
                    clap_complete::Shell::Fish => {
                        let _ = writeln!(out);
                        for name in &names {
                            let _ = writeln!(out, "complete -c {name} -w lx");
                        }
                    }
                    _ => {} // Elvish, PowerShell — contributions welcome
                }
            }
        }

        OptionsResult::InitConfig => {
            let path = config::init_config_path();
            config::write_init_config(&path)?;
            eprintln!("Wrote default config to {}", path.display());
        }

        OptionsResult::ShowConfig => {
            let name = active_personality.as_deref().unwrap_or("lx");
            let cli_theme = find_theme_arg(&cli_args);
            config::show_config(name, personality_source, cli_theme.as_deref());
        }

        OptionsResult::SaveAs(ref name, _, _) => {
            // Parse just the CLI args (without personality layer) to
            // extract only what the user typed on this command line.
            let cli_settings = match Options::parse(&cli_args, &LiveVars) {
                OptionsResult::SaveAs(_, _, settings) => settings,
                _ => std::collections::HashMap::new(),
            };
            let inherits = active_personality.as_deref();
            config::save_personality_as(name, inherits, &cli_settings)?;
        }

        OptionsResult::DumpClass(ref name) => {
            if name.is_empty() {
                config::show_class_all();
            } else {
                config::show_class(name)?;
            }
        }

        OptionsResult::DumpFormat(ref name) => {
            if name.is_empty() {
                config::show_format_all();
            } else {
                config::show_format(name)?;
            }
        }

        OptionsResult::DumpPersonality(ref name) => {
            if name.is_empty() {
                config::dump_personality_all();
            } else {
                config::dump_personality(name)?;
            }
        }

        OptionsResult::DumpTheme(ref name) => {
            if name.is_empty() {
                config::dump_theme_all();
            } else {
                config::dump_theme(name)?;
            }
        }

        OptionsResult::DumpStyle(ref name) => {
            if name.is_empty() {
                config::dump_style_all();
            } else {
                config::dump_style(name)?;
            }
        }

        OptionsResult::UpgradeConfig => {
            let path = config::find_config_path()
                .ok_or(config::ConfigError::NothingToUpgrade)?;
            config::upgrade_config(&path)?;
        }

        OptionsResult::InvalidOptions(error) => return Err(LxError::from(error)),
    }

    Ok(exits::SUCCESS)
}


/// The main program wrapper.  Holds parsed options, the theme, and any
/// pre-populated caches (e.g. Git status).
pub struct Lx {

    /// List of command-line options, having been successfully parsed.
    pub options: Options,

    /// The output handle that we write to.
    pub writer: io::Stdout,

    /// List of the free command-line arguments that should correspond to file
    /// names (anything that isn't an option).
    pub input_paths: Vec<OsString>,

    /// The theme that has been configured from the command-line options and
    /// environment variables. If colours are disabled, this is a theme with
    /// every style set to the default.
    pub theme: Theme,

    /// The detected width of the console. This is used to determine which
    /// view to use.
    pub console_width: Option<usize>,

    /// A global VCS cache, if the option was passed in.
    /// This has to last the lifetime of the program, because the user might
    /// want to list several directories in the same repository.
    pub vcs: Option<Box<dyn VcsCache>>,

    /// Running count of items listed (for `--count`).
    pub item_count: usize,

    /// Running total of displayed file sizes in bytes (for `-CZ`).
    pub size_total: u64,

    /// Numeric locale with personality overrides applied.
    pub numeric: locale::Numeric,
}

/// Format a byte count as a human-readable string for the `-CZ` summary.
/// Respects the active size format (`-B` for binary, `-b` for bytes)
/// and the personality's numeric formatting overrides.
fn format_size(bytes: u64, fmt: crate::output::table::SizeFormat, numeric: &locale::Numeric) -> String {
    use unit_prefix::NumberPrefix;
    use crate::output::table::SizeFormat;

    match fmt {
        SizeFormat::JustBytes => {
            // Thousands-separated with "bytes" suffix for clarity.
            let s = bytes.to_string();
            let sep = &numeric.thousands_sep;
            let formatted = if sep.is_empty() || s.len() <= 3 {
                s
            } else {
                let mut out = String::new();
                for (i, c) in s.chars().rev().enumerate() {
                    if i > 0 && i % 3 == 0 { out.insert_str(0, sep); }
                    out.insert(0, c);
                }
                out
            };
            format!("{formatted} bytes")
        }
        SizeFormat::DecimalBytes => match NumberPrefix::decimal(bytes as f64) {
            NumberPrefix::Standalone(n) => format!("{n} B"),
            NumberPrefix::Prefixed(prefix, n) if n < 10.0 => format!("{n:.1} {prefix}B"),
            NumberPrefix::Prefixed(prefix, n) => format!("{n:.0} {prefix}B"),
        },
        SizeFormat::BinaryBytes => match NumberPrefix::binary(bytes as f64) {
            NumberPrefix::Standalone(n) => format!("{n} B"),
            NumberPrefix::Prefixed(prefix, n) if n < 10.0 => format!("{n:.1} {prefix}B"),
            NumberPrefix::Prefixed(prefix, n) => format!("{n:.0} {prefix}B"),
        },
    }
}

/// The "real" environment variables type.
/// Instead of just calling `var_os` from within the options module,
/// the method of looking up environment variables has to be passed in.
struct LiveVars;
impl Vars for LiveVars {
    fn get(&self, name: &'static str) -> Option<OsString> {
        env::var_os(name)
    }
}

/// Discover a jj workspace and create a VCS cache using jj-lib.
fn discover_jj(paths: &[PathBuf]) -> Option<Box<dyn VcsCache>> {
    JjCache::discover(paths).map(|c| {
        let b: Box<dyn VcsCache> = Box::new(c);
        b
    })
}

/// Create a VCS cache based on the selected backend and the paths that
/// are going to be listed.
fn vcs_cache(options: &Options, args: &[OsString]) -> Option<Box<dyn VcsCache>> {
    if !options.should_scan_for_vcs() {
        return None;
    }

    let paths: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();

    match options.vcs_backend {
        VcsBackend::None => None,

        VcsBackend::Git => {
            let cache: GitCache = paths.into_iter().collect();
            Some(Box::new(cache))
        }

        VcsBackend::Jj => {
            discover_jj(&paths)
        }

        VcsBackend::Auto => {
            // Prefer jj if a workspace is detected, fall back to git.
            if let Some(jj) = discover_jj(&paths) {
                Some(jj)
            } else {
                let cache: GitCache = paths.into_iter().collect();
                Some(Box::new(cache))
            }
        }
    }
}

impl Lx {
    /// # Errors
    ///
    /// Will return `Err` if printing to stderr fails.
    pub fn run(mut self) -> io::Result<i32> {
        debug!("Running with options: {:#?}", self.options);

        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut exit_status = 0;

        for file_path in &self.input_paths {
            match File::from_args(PathBuf::from(file_path), None, None, None) {
                Err(e) => {
                    exit_status = 2;
                    writeln!(io::stderr(), "{}: {e}", file_path.to_string_lossy())?;
                }

                Ok(f) => {
                    if f.points_to_directory() && ! self.options.dir_action.treat_dirs_as_files() {
                        match f.to_dir() {
                            Ok(d)   => dirs.push(d),
                            Err(e)  => writeln!(io::stderr(), "{}: {e}", file_path.to_string_lossy())?,
                        }
                    }
                    else {
                        files.push(f);
                    }
                }
            }
        }

        // We want to print a directory's name before we list it, *except* in
        // the case where it's the only directory, *except* if there are any
        // files to print as well. (It's a double negative)

        let no_files = files.is_empty();
        let is_only_dir = dirs.len() == 1 && no_files;

        self.options.filter.filter_argument_files(&mut files);
        self.print_files(None, files)?;

        let result = self.print_dirs(dirs, no_files, is_only_dir, exit_status);

        if self.options.count {
            let count = self.numeric.format_int(self.item_count as isize);
            // Theme the footer: numbers in size.major (highlighted),
            // chrome text in punctuation (subdued).  When colour is
            // off both styles are Style::default() — no escapes.
            let number_style = self.theme.ui.size.major;
            let chrome_style = self.theme.ui.punctuation;
            let count_p = number_style.paint(&count);
            let label_p = chrome_style.paint(" items shown");
            if self.options.view.has_total_size() {
                let fmt = self.options.view.size_format()
                    .unwrap_or(crate::output::table::SizeFormat::DecimalBytes);
                let size_str = format_size(self.size_total, fmt, &self.numeric);
                let comma_p = chrome_style.paint(", ");
                let size_p = number_style.paint(&size_str);
                eprintln!("{count_p}{label_p}{comma_p}{size_p}");
            } else {
                eprintln!("{count_p}{label_p}");
            }
        }

        result
    }

    fn print_dirs(&mut self, dir_files: Vec<Dir>, mut first: bool, is_only_dir: bool, exit_status: i32) -> io::Result<i32> {
        for dir in dir_files {

            // Put a gap between directories, or between the list of files and
            // the first directory.
            if first {
                first = false;
            }
            else {
                writeln!(&mut self.writer)?;
            }

            if ! is_only_dir {
                let mut bits = Vec::new();
                escape(dir.path.display().to_string(), &mut bits, Style::default(), Style::default());
                writeln!(&mut self.writer, "{}:", AnsiStrings(&bits))?;
            }

            let mut children = Vec::new();
            let vcs_ignore = self.options.filter.vcs_ignore == VcsIgnore::CheckAndIgnore;
            for file in dir.files(self.options.filter.dot_filter, self.vcs.as_deref(), vcs_ignore) {
                match file {
                    Ok(file)        => children.push(file),
                    Err((path, e))  => writeln!(io::stderr(), "[{}: {}]", path.display(), e)?,
                }
            };

            self.options.filter.filter_child_files(&mut children);
            self.options.filter.sort_files(&mut children, self.vcs.as_deref());

            if let Some(recurse_opts) = self.options.dir_action.recurse_options() {
                let depth = dir.path.components().filter(|&c| c != Component::CurDir).count() + 1;
                if ! recurse_opts.tree && ! recurse_opts.is_too_deep(depth) {

                    let mut child_dirs = Vec::new();
                    for child_dir in children.iter().filter(|f| f.is_directory() && ! f.is_all_all && ! self.options.filter.is_pruned(f)) {
                        match child_dir.to_dir() {
                            Ok(d)   => child_dirs.push(d),
                            Err(e)  => writeln!(io::stderr(), "{}: {}", child_dir.path.display(), e)?,
                        }
                    }

                    self.print_files(Some(&dir), children)?;
                    match self.print_dirs(child_dirs, false, false, exit_status) {
                        Ok(_)   => (),
                        Err(e)  => return Err(e),
                    }
                    continue;
                }
            }

            self.print_files(Some(&dir), children)?;
        }

        Ok(exit_status)
    }

    /// Prints the list of files using whichever view is selected.
    fn print_files(&mut self, dir: Option<&Dir>, files: Vec<File<'_>>) -> io::Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let theme = &self.theme;
        let View { mode, file_style, .. } = &self.options.view;

        // Sum file sizes for non-details modes (flat listing).
        let sum_file_sizes = |files: &[File<'_>]| -> u64 {
            files.iter()
                .filter(|f| f.is_file())
                .map(|f| f.metadata().len())
                .sum()
        };

        match (mode, self.console_width) {
            (Mode::Grid(opts), Some(console_width)) => {
                self.item_count += files.len();
                self.size_total += sum_file_sizes(&files);
                let filter = &self.options.filter;
                let r = grid::Render { files, theme, file_style, opts, console_width, filter };
                r.render(&mut self.writer)
            }

            (Mode::Grid(_), None) |
            (Mode::Lines,   _)    => {
                self.item_count += files.len();
                self.size_total += sum_file_sizes(&files);
                let filter = &self.options.filter;
                let r = lines::Render { files, theme, file_style, filter };
                r.render(&mut self.writer)
            }

            (Mode::Details(opts), _) => {
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();

                let vcs_ignoring = self.options.filter.vcs_ignore == VcsIgnore::CheckAndIgnore;
                let vcs = self.vcs.as_deref();
                let r = details::Render { dir, files, theme, file_style, opts, recurse, filter, vcs_ignoring, vcs };
                let (count, bytes) = r.render(&mut self.writer)?;
                self.item_count += count;
                self.size_total += bytes;
                Ok(())
            }

            (Mode::GridDetails(opts), Some(console_width)) => {
                self.item_count += files.len();
                self.size_total += sum_file_sizes(&files);
                let grid = &opts.grid;
                let details = &opts.details;
                let row_threshold = opts.row_threshold;

                let filter = &self.options.filter;
                let vcs_ignoring = self.options.filter.vcs_ignore == VcsIgnore::CheckAndIgnore;
                let vcs = self.vcs.as_deref();

                let r = grid_details::Render { dir, files, theme, file_style, grid, details, filter, row_threshold, vcs_ignoring, vcs, console_width };
                r.render(&mut self.writer)
            }

            (Mode::GridDetails(opts), None) => {
                let opts = &opts.to_details_options();
                let filter = &self.options.filter;
                let recurse = self.options.dir_action.recurse_options();
                let vcs_ignoring = self.options.filter.vcs_ignore == VcsIgnore::CheckAndIgnore;

                let vcs = self.vcs.as_deref();
                let r = details::Render { dir, files, theme, file_style, opts, recurse, filter, vcs_ignoring, vcs };
                let (count, bytes) = r.render(&mut self.writer)?;
                self.item_count += count;
                self.size_total += bytes;
                Ok(())
            }
        }
    }
}


/// Scan raw args for --personality=NAME or -p NAME before Clap parsing.
fn find_personality_arg(args: &[OsString]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();
        if let Some(name) = s.strip_prefix("--personality=") {
            return Some(name.to_string());
        }
        if (s == "--personality" || s == "-p")
            && let Some(next) = iter.next() {
                return Some(next.to_string_lossy().to_string());
            }
        // Handle -pNAME (short flag with attached value)
        if let Some(name) = s.strip_prefix("-p")
            && !name.is_empty() && !name.starts_with('-') {
                return Some(name.to_string());
            }
    }
    None
}

/// Scan raw args for --theme=NAME or --theme NAME.  Used by
/// --show-config to display the effective theme override.
fn find_theme_arg(args: &[OsString]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();
        if let Some(name) = s.strip_prefix("--theme=") {
            return Some(name.to_string());
        }
        if s == "--theme"
            && let Some(next) = iter.next() {
                return Some(next.to_string_lossy().to_string());
            }
    }
    None
}


mod exits {

    /// Exit code for when lx runs OK.
    pub const SUCCESS: i32 = 0;

    /// Exit code for when there was at least one I/O error during execution.
    pub const RUNTIME_ERROR: i32 = 1;

    /// Exit code for when the command-line options are invalid.
    pub const OPTIONS_ERROR: i32 = 3;
}
