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

use crate::fs::{Dir, File};
use crate::fs::feature::git::GitCache;
use crate::fs::feature::jj::JjCache;
use crate::fs::feature::VcsCache;
use crate::fs::filter::VcsIgnore;
use crate::options::{Options, VcsBackend, Vars, vars, OptionsResult};
use crate::output::{escape, lines, grid, grid_details, details, View, Mode};
use crate::theme::Theme;

mod config;
mod fs;
mod info;
mod logger;
mod options;
mod output;
mod theme;


fn main() {
    use std::process::exit;

    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    logger::configure(env::var_os(vars::LX_DEBUG));

    #[cfg(windows)]
    if let Err(e) = nu_ansi_term::enable_ansi_support() {
        warn!("Failed to enable ANSI support: {}", e);
    }

    let cli_args: Vec<OsString> = env::args_os().skip(1).collect();

    // Skip config loading for --upgrade-config (the config may be
    // legacy and would emit an error before we get to handle it).
    let upgrading = cli_args.iter().any(|a| a == "--upgrade-config");
    if !upgrading {
        // Force config to load (populates the CONFIG static).
        let _ = &*config::CONFIG;
    }

    // Build the arg list in layers; Clap's args_override_self ensures
    // later flags win: personality < CLI flags.
    let mut args: Vec<OsString> = Vec::new();

    // Layer 1: personality (skip when upgrading — config may be legacy).
    if !upgrading {
        // Determine which personality to apply.
        // Explicit --personality/-p takes priority; if absent, check argv[0].
        let explicit_personality = find_personality_arg(&cli_args);
        let personality_name = explicit_personality.or_else(|| {
            let argv0 = env::args().next()?;
            let bin_name = std::path::Path::new(&argv0)
                .file_name()?
                .to_string_lossy()
                .to_string();
            // Check if this name resolves to a personality (config or compiled-in).
            // We only probe here; actual resolution + error handling happens below.
            match config::resolve_personality(&bin_name) {
                Ok(Some(_)) | Err(_) => {
                    debug!("argv[0] dispatch: {bin_name}");
                    Some(bin_name)
                }
                Ok(None) => None,  // unknown name, not a personality
            }
        });

        if let Some(ref personality_name) = personality_name {
            match config::resolve_personality(personality_name) {
                Ok(Some(personality)) => {
                    args.extend(personality.to_args());
                }
                Ok(None) => {}  // unknown personality, ignore
                Err(e) => {
                    eprintln!("lx: {e}");
                    std::process::exit(exits::OPTIONS_ERROR);
                }
            }
        }
    }

    // Layer 3: actual CLI args (override everything above).
    args.extend(cli_args);


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
            let theme = options.theme.to_theme(console_width.is_some());
            let lx = Lx { options, writer, input_paths, theme, console_width, vcs };

            match lx.run() {
                Ok(exit_status) => {
                    exit(exit_status);
                }

                Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                    warn!("Broken pipe error: {e}");
                    exit(exits::SUCCESS);
                }

                Err(e) => {
                    eprintln!("{e}");
                    exit(exits::RUNTIME_ERROR);
                }
            }
        }

        OptionsResult::HelpOrVersion(clap_err) => {
            clap_err.exit();
        }

        OptionsResult::InvalidOptionsClap(clap_err) => {
            clap_err.exit();
        }

        OptionsResult::Completions(shell) => {
            let mut cmd = crate::options::parser::build_command();
            clap_complete::generate(shell, &mut cmd, "lx", &mut io::stdout());
        }

        OptionsResult::InitConfig => {
            let path = config::init_config_path();
            match config::write_init_config(&path) {
                Ok(()) => {
                    eprintln!("Wrote default config to {}", path.display());
                }
                Err(e) => {
                    eprintln!("lx: failed to write config to {}: {e}", path.display());
                    exit(exits::RUNTIME_ERROR);
                }
            }
        }

        OptionsResult::UpgradeConfig => {
            let Some(path) = config::find_config_path() else {
                eprintln!("lx: no config file found to upgrade");
                exit(exits::RUNTIME_ERROR);
            };
            if let Err(e) = config::upgrade_config(&path) {
                eprintln!("lx: {e}");
                exit(exits::RUNTIME_ERROR);
            }
        }

        OptionsResult::InvalidOptions(error) => {
            eprintln!("lx: {error}");
            exit(exits::OPTIONS_ERROR);
        }
    }
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
            JjCache::discover(&paths).map(|c| {
                let b: Box<dyn VcsCache> = Box::new(c);
                b
            })
        }

        VcsBackend::Auto => {
            // Prefer jj if a workspace is detected, fall back to git.
            if let Some(jj) = JjCache::discover(&paths) {
                let b: Box<dyn VcsCache> = Box::new(jj);
                Some(b)
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
            match File::from_args(PathBuf::from(file_path), None, None) {
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

        self.print_dirs(dirs, no_files, is_only_dir, exit_status)
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
            self.options.filter.sort_files(&mut children);

            if let Some(recurse_opts) = self.options.dir_action.recurse_options() {
                let depth = dir.path.components().filter(|&c| c != Component::CurDir).count() + 1;
                if ! recurse_opts.tree && ! recurse_opts.is_too_deep(depth) {

                    let mut child_dirs = Vec::new();
                    for child_dir in children.iter().filter(|f| f.is_directory() && ! f.is_all_all) {
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

        match (mode, self.console_width) {
            (Mode::Grid(opts), Some(console_width)) => {
                let filter = &self.options.filter;
                let r = grid::Render { files, theme, file_style, opts, console_width, filter };
                r.render(&mut self.writer)
            }

            (Mode::Grid(_), None) |
            (Mode::Lines,   _)    => {
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
                r.render(&mut self.writer)
            }

            (Mode::GridDetails(opts), Some(console_width)) => {
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
                r.render(&mut self.writer)
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
        if s == "--personality" || s == "-p" {
            if let Some(next) = iter.next() {
                return Some(next.to_string_lossy().to_string());
            }
        }
        // Handle -pNAME (short flag with attached value)
        if let Some(name) = s.strip_prefix("-p") {
            if !name.is_empty() && !name.starts_with('-') {
                return Some(name.to_string());
            }
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
