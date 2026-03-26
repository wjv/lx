use clap::{Arg, ArgAction, Command};
use clap::builder::PossibleValue;
use clap::builder::styling;

use super::flags;


/// Clap styling: yellow headers, cyan literals, green placeholders, red errors.
const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Yellow.on_default().bold())
    .usage(styling::AnsiColor::Yellow.on_default().bold())
    .literal(styling::AnsiColor::Cyan.on_default().bold())
    .placeholder(styling::AnsiColor::Green.on_default())
    .error(styling::AnsiColor::Red.on_default().bold());


/// Return valid values for the `--sort` flag.
fn sort_values() -> Vec<PossibleValue> {
    vec![
        PossibleValue::new("name"),
        PossibleValue::new("Name"),
        PossibleValue::new("size"),
        PossibleValue::new("extension"),
        PossibleValue::new("Extension"),
        PossibleValue::new("modified"),
        PossibleValue::new("changed"),
        PossibleValue::new("accessed"),
        PossibleValue::new("created"),
        PossibleValue::new("type"),
        PossibleValue::new("none"),
        PossibleValue::new("inode"),
        // Aliases (hidden from help):
        PossibleValue::new("ext").hide(true),
        PossibleValue::new("Ext").hide(true),
        PossibleValue::new("date").hide(true),
        PossibleValue::new("time").hide(true),
        PossibleValue::new("mod").hide(true),
        PossibleValue::new("newest").hide(true),
        PossibleValue::new("new").hide(true),
        PossibleValue::new("age").hide(true),
        PossibleValue::new("old").hide(true),
        PossibleValue::new("oldest").hide(true),
        PossibleValue::new(".name").hide(true),
        PossibleValue::new(".Name").hide(true),
        PossibleValue::new("ch").hide(true),
        PossibleValue::new("acc").hide(true),
        PossibleValue::new("cr").hide(true),
    ]
}


/// A wrapper around clap's `ArgMatches` that provides convenience
/// methods matching those expected by the deduce functions.
pub struct MatchedFlags {
    matches: clap::ArgMatches,
}

impl MatchedFlags {
    pub fn new(matches: clap::ArgMatches) -> Self {
        Self { matches }
    }

    /// Whether the given flag was present at all.
    pub fn has(&self, flag: &str) -> bool {
        // Try bool first (SetTrue), then u8 (Count).
        if self.matches.try_get_one::<bool>(flag).ok().flatten().copied().unwrap_or(false) {
            return true;
        }
        self.matches.try_get_one::<u8>(flag).ok().flatten().copied().unwrap_or(0) > 0
    }

    /// How many times a counted flag was passed.
    pub fn count(&self, flag: &str) -> u8 {
        self.matches.get_count(flag)
    }

    /// Get the string value of a flag, if present.
    pub fn get(&self, flag: &str) -> Option<&str> {
        self.matches.get_one::<String>(flag).map(|s| s.as_str())
    }

    /// Get a usize value of a flag, if present.
    pub fn get_usize(&self, flag: &str) -> Option<usize> {
        self.matches.get_one::<usize>(flag).copied()
    }

    /// Whether the given flag was present at all (including default values).
    pub fn is_present(&self, flag: &str) -> bool {
        self.matches.contains_id(flag) && self.matches.value_source(flag) == Some(clap::parser::ValueSource::CommandLine)
    }
}


/// Build the Clap `Command` that defines all lx flags.
pub fn build_command() -> Command {
    Command::new("lx")
        .version(env!("CARGO_PKG_VERSION"))
        .about("The file lister with personality! 🌟")
        .styles(STYLES)
        .max_term_width(80)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)
        .help_template("{name} {version}\n{about-section}\n\
                        {usage-heading}\n{tab}{usage}\n\n\
                        {all-args}{after-help}")
        .after_help("\
Environment variables:\n  \
  COLUMNS          Override terminal width (characters)\n  \
  LX_GRID_ROWS    Minimum rows before grid-details view activates\n  \
  LX_ICON_SPACING Spaces between icon and file name\n  \
  NO_COLOR         Disable colours (overridden by --colour)\n  \
  LS_COLORS        File-type colour scheme\n  \
  LX_COLORS       Extended colour scheme (UI elements and metadata)\n  \
  TIME_STYLE       Default timestamp style (overridden by --time-style)")

        // ── Display mode ──────────────────────────────────────────

        .arg(Arg::new(flags::ONE_LINE)
            .short('1').long("oneline")
            .help("Display one entry per line")
            .help_heading("Display")
            .action(ArgAction::Count)
            .overrides_with_all([flags::LONG, flags::GRID]))
        .arg(Arg::new(flags::LONG)
            .short('l').long("long")
            .help("Long view — repeat for more detail: -ll, -lll")
            .help_heading("Display")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::GRID)
            .short('G').long("grid")
            .help("Display entries as a grid (default)")
            .help_heading("Display")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::ACROSS)
            .short('x').long("across")
            .help("Sort the grid across, rather than downwards")
            .help_heading("Display")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::RECURSE)
            .short('R').long("recurse")
            .help("Recurse into directories")
            .help_heading("Display")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TREE)
            .short('T').long("tree")
            .help("Recurse into directories as a tree")
            .help_heading("Display")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LEVEL)
            .short('L').long("level")
            .help("Limit the depth of recursion")
            .help_heading("Display")
            .action(ArgAction::Set)
            .value_name("DEPTH")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new(flags::CLASSIFY)
            .long("classify")
            .help("Display file kind indicators")
            .help_heading("Display")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .value_parser([
                PossibleValue::new("always"),
                PossibleValue::new("auto"),
                PossibleValue::new("never"),
            ])
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("auto"))

        // ── Filtering and sorting ─────────────────────────────────

        .arg(Arg::new(flags::ALL)
            .short('a').long("all")
            .help("Show hidden and dot files (-aa for . and ..)")
            .help_heading("Filtering")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LIST_DIRS)
            .short('d').long("list-dirs")
            .help("List directories as regular files")
            .help_heading("Filtering")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_DIRS)
            .short('D').long("only-dirs")
            .help("List only directories, not files")
            .help_heading("Filtering")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_FILES)
            .short('f').long("only-files")
            .help("List only regular files, not directories")
            .help_heading("Filtering")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::IGNORE_GLOB)
            .short('I').long("ignore")
            .visible_alias("ignore-glob")
            .help("Glob patterns (pipe-separated) of files to hide")
            .help_heading("Filtering")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::SYMLINKS)
            .long("symlinks")
            .help("How to handle symlinks [show, hide, follow]")
            .help_heading("Filtering")
            .action(ArgAction::Set)
            .value_name("MODE")
            .value_parser([
                PossibleValue::new("show"),
                PossibleValue::new("hide"),
                PossibleValue::new("follow"),
            ]))
        .arg(Arg::new(flags::PRUNE)
            .short('P').long("prune")
            .visible_alias("prune-glob")
            .help("Glob patterns of directories to show but not recurse into")
            .help_heading("Filtering")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::REVERSE)
            .short('r').long("reverse")
            .help("Reverse the sort order")
            .help_heading("Filtering")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SORT)
            .short('s').long("sort")
            .help("Sort field")
            .help_heading("Filtering")
            .action(ArgAction::Set)
            .value_name("FIELD")
            .value_parser(sort_values()))
        .arg(Arg::new(flags::GROUP_DIRS)
            .long("group-dirs")
            .help("Group directories before or after other files")
            .help_heading("Filtering")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .overrides_with_all([flags::DIRS_FIRST, flags::DIRS_LAST])
            .value_parser([
                PossibleValue::new("first"),
                PossibleValue::new("last"),
                PossibleValue::new("none"),
            ]))
        .arg(Arg::new(flags::DIRS_FIRST)
            .short('F')
            .long("dirs-first")
            .alias("group-directories-first")
            .help("Directories first (short for --group-dirs=first)")
            .help_heading("Filtering")
            .action(ArgAction::SetTrue)
            .overrides_with_all([flags::GROUP_DIRS, flags::DIRS_LAST]))
        .arg(Arg::new(flags::DIRS_LAST)
            .short('J')
            .long("dirs-last")
            .alias("group-directories-last")
            .help("Directories last (short for --group-dirs=last)")
            .help_heading("Filtering")
            .action(ArgAction::SetTrue)
            .overrides_with_all([flags::GROUP_DIRS, flags::DIRS_FIRST]))

        // ── Long view columns ─────────────────────────────────────

        .arg(Arg::new(flags::BINARY)
            .short('b').long("binary")
            .help("File sizes with binary prefixes (KiB, MiB)")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::BYTES))
        .arg(Arg::new(flags::BYTES)
            .short('B').long("bytes")
            .help("File sizes in bytes, without prefixes")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::BINARY))
        .arg(Arg::new(flags::GROUP)
            .short('g').long("group")
            .help("Show the group column")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NUMERIC)
            .short('n').long("numeric")
            .help("Numeric user and group IDs")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::HEADER)
            .short('h').long("header")
            .help("Add a header row")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::INODE)
            .short('i').long("inode")
            .help("Show inode numbers")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LINKS)
            .short('H').long("links")
            .help("Show hard link counts")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::BLOCKS)
            .short('S').long("blocks")
            .help("Show file system block counts")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::OCTAL)
            .short('o').long("octal")
            .visible_alias("octal-permissions")
            .help("Show permissions in octal format")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TOTAL_SIZE)
            .short('Z').long("total-size")
            .help("Show directory content sizes (recursive)")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::EXTENDED)
            .short('@').long("extended")
            .help("Show extended attributes and sizes")
            .help_heading("Long view")
            .action(ArgAction::Count))

        // ── Timestamps ────────────────────────────────────────────

        .arg(Arg::new(flags::MODIFIED)
            .short('m').long("modified")
            .help("Use the modified timestamp")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CHANGED)
            .short('c').long("changed")
            .help("Use the changed timestamp")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ACCESSED)
            .short('u').long("accessed")
            .help("Use the accessed timestamp")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CREATED)
            .short('U').long("created")
            .help("Use the created timestamp")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME)
            .short('t').long("time")
            .help("Which timestamp field to display")
            .help_heading("Timestamps")
            .action(ArgAction::Set)
            .value_name("FIELD")
            .value_parser([
                PossibleValue::new("modified"),
                PossibleValue::new("changed"),
                PossibleValue::new("accessed"),
                PossibleValue::new("created"),
                PossibleValue::new("mod").hide(true),
                PossibleValue::new("ch").hide(true),
                PossibleValue::new("acc").hide(true),
                PossibleValue::new("cr").hide(true),
            ])
            .conflicts_with_all([
                flags::MODIFIED,
                flags::CHANGED,
                flags::ACCESSED,
                flags::CREATED,
            ]))
        .arg(Arg::new(flags::TIME_STYLE)
            .long("time-style")
            .help("How to format timestamps [default, iso, long-iso, full-iso, relative, +FORMAT]")
            .help_heading("Timestamps")
            .action(ArgAction::Set)
            .value_name("STYLE"))

        // ── Column visibility ─────────────────────────────────────

        .arg(Arg::new(flags::SHOW_PERMISSIONS)
            .long("permissions")
            .help("Show the permissions field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_PERMISSIONS))
        .arg(Arg::new(flags::NO_PERMISSIONS)
            .long("no-permissions")
            .help("Suppress the permissions field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_PERMISSIONS))
        .arg(Arg::new(flags::SHOW_FILESIZE)
            .long("filesize")
            .help("Show the file size field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_FILESIZE))
        .arg(Arg::new(flags::NO_FILESIZE)
            .long("no-filesize")
            .help("Suppress the file size field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_FILESIZE))
        .arg(Arg::new(flags::SHOW_USER)
            .long("user")
            .help("Show the user field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_USER))
        .arg(Arg::new(flags::NO_USER)
            .long("no-user")
            .help("Suppress the user field")
            .help_heading("Column visibility")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_USER))
        .arg(Arg::new(flags::NO_TIME)
            .long("no-time")
            .help("Suppress the time field")
            .help_heading("Column visibility")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_ICONS)
            .long("no-icons")
            .help("Suppress icons (alias for --icons=never)")
            .help_heading("Column visibility")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_INODE)
            .long("no-inode")
            .help("Suppress the inode field")
            .help_heading("Column visibility")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_GROUP)
            .long("no-group")
            .help("Suppress the group field")
            .help_heading("Column visibility")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_LINKS)
            .long("no-links")
            .help("Suppress the hard links field")
            .help_heading("Column visibility")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_BLOCKS)
            .long("no-blocks")
            .help("Suppress the blocks field")
            .help_heading("Column visibility")
            .action(ArgAction::Count))

        // ── Column / format / personality ─────────────────────────

        .arg(Arg::new(flags::COLUMNS)
            .long("columns")
            .help("Explicit column list (comma-separated)")
            .help_heading("Formats & personalities")
            .action(ArgAction::Set)
            .value_name("COLS"))
        .arg(Arg::new(flags::FORMAT)
            .long("format")
            .help("Named column format")
            .help_heading("Formats & personalities")
            .action(ArgAction::Set)
            .value_name("NAME")
            .value_parser({
                let names = crate::options::view::format_names();
                names.into_iter()
                    .map(|s| { let leaked: &'static str = Box::leak(s.into_boxed_str()); PossibleValue::new(leaked) })
                    .collect::<Vec<_>>()
            }))
        .arg(Arg::new(flags::PERSONALITY)
            .short('p').long("personality")
            .help("Apply a named personality (columns + flags)")
            .help_heading("Formats & personalities")
            .action(ArgAction::Set)
            .value_name("NAME"))

        // ── VCS ───────────────────────────────────────────────────

        .arg(Arg::new(flags::VCS)
            .long("vcs")
            .help("VCS backend [auto, git, jj, none]")
            .help_heading("VCS")
            .action(ArgAction::Set)
            .value_name("BACKEND")
            .value_parser([
                PossibleValue::new("auto"),
                PossibleValue::new("git"),
                PossibleValue::new("jj"),
                PossibleValue::new("none"),
            ]))
        .arg(Arg::new(flags::VCS_STATUS)
            .long("vcs-status")
            .help("Show per-file VCS status column")
            .help_heading("VCS")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::VCS_IGNORE)
            .long("vcs-ignore")
            .help("Hide VCS-ignored files and metadata directories")
            .help_heading("VCS")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::VCS_REPOS)
            .long("vcs-repos")
            .help("Show per-directory VCS repo indicator")
            .help_heading("VCS")
            .action(ArgAction::Count))

        // ── Appearance ────────────────────────────────────────────

        .arg(Arg::new(flags::COLOR)
            .long("colour").visible_alias("color")
            .help("When to use terminal colours")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .value_parser([
                PossibleValue::new("always"),
                PossibleValue::new("auto"),
                PossibleValue::new("never"),
                PossibleValue::new("automatic").hide(true),
            ]))
        .arg(Arg::new(flags::COLOR_SCALE)
            .long("colour-scale").visible_alias("color-scale")
            .help("Colour file sizes on a scale")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("MODE")
            .value_parser([
                PossibleValue::new("16"),
                PossibleValue::new("256"),
                PossibleValue::new("none"),
            ])
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("16"))
        .arg(Arg::new(flags::ICONS)
            .long("icons")
            .help("Display icons next to file names")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .value_parser([
                PossibleValue::new("always"),
                PossibleValue::new("auto"),
                PossibleValue::new("never"),
            ])
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("auto"))
        .arg(Arg::new("theme")
            .long("theme")
            .help("Use a named colour theme")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("NAME"))
        .arg(Arg::new("hyperlink")
            .long("hyperlink")
            .help("File names as clickable hyperlinks")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .default_missing_value("always")
            .require_equals(true)
            .num_args(0..=1)
            .value_parser(["always", "auto", "never"]))
        .arg(Arg::new("quotes")
            .long("quotes")
            .help("Quote file names containing spaces")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .default_missing_value("always")
            .require_equals(true)
            .num_args(0..=1)
            .value_parser(["always", "auto", "never"]))
        .arg(Arg::new("width")
            .long("width")
            .short('w')
            .help("Override terminal width")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("COLS")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new("absolute")
            .short('A')
            .long("absolute")
            .help("Show absolute file paths")
            .help_heading("Appearance")
            .action(ArgAction::SetTrue))

        // ── Configuration ─────────────────────────────────────────

        .arg(Arg::new("show-config")
            .long("show-config")
            .help("Show the active configuration and exit")
            .help_heading("Configuration")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("dump-class")
            .long("dump-class")
            .help("Dump class definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true))
        .arg(Arg::new("dump-format")
            .long("dump-format")
            .help("Dump format definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true))
        .arg(Arg::new("dump-personality")
            .long("dump-personality")
            .help("Dump personality definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true))
        .arg(Arg::new("dump-theme")
            .long("dump-theme")
            .help("Dump theme definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true))
        .arg(Arg::new("dump-style")
            .long("dump-style")
            .help("Dump style definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true))
        .arg(Arg::new("init-config")
            .long("init-config")
            .help("Generate a default config file")
            .help_heading("Configuration")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("upgrade-config")
            .long("upgrade-config")
            .help("Upgrade a legacy config file")
            .help_heading("Configuration")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("completions")
            .long("completions")
            .help("Generate shell completions")
            .help_heading("Shell completions")
            .action(ArgAction::Set)
            .value_name("SHELL")
            .value_parser(clap::value_parser!(clap_complete::Shell)))

        // ── Help & version ────────────────────────────────────────

        .arg(Arg::new("help")
            .short('?').long("help")
            .help("Print help information")
            .action(ArgAction::HelpShort))
        .arg(Arg::new("version")
            .short('v').long("version")
            .help("Print version information")
            .action(ArgAction::Version))

        // Positional arguments (files/directories)
        .arg(Arg::new("FILE")
            .action(ArgAction::Append)
            .value_parser(clap::value_parser!(std::ffi::OsString)))
}
