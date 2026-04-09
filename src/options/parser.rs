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

/// Choose help styles based on environment: respect `NO_COLOR` and
/// check whether stderr is a terminal (help is written to stderr).
fn help_styles() -> styling::Styles {
    if std::env::var_os("NO_COLOR").is_some() {
        return styling::Styles::plain();
    }
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return styling::Styles::plain();
    }
    STYLES
}


/// Return valid values for the `--sort` flag.
///
/// Generated from the sort registry (`src/fs/sort_registry.rs`) so
/// adding a new sort field doesn't require touching this file.
/// Canonical names are visible in `--help`; aliases are hidden.
///
/// `version` is a special case: it's an alias for `name` that isn't
/// a distinct registry entry, so we append it explicitly.
fn sort_values() -> Vec<PossibleValue> {
    use crate::fs::sort_registry::SortFieldDef;

    let mut values: Vec<PossibleValue> = SortFieldDef::visible_canonical_names()
        .map(PossibleValue::new)
        .collect();

    // `version` alias — not a distinct registry entry.
    values.push(PossibleValue::new("version"));

    // Hidden aliases and hidden-canonical entries from the registry.
    values.extend(
        SortFieldDef::all_hidden_names().map(|n| PossibleValue::new(n).hide(true))
    );

    values
}


/// `TypedValueParser` for `--time-style`.
///
/// `+strftime` formats are accepted directly; everything else is
/// delegated to a `PossibleValuesParser` so clap's native error
/// formatting *and* its built-in "did you mean" suggestions both
/// kick in.  `+FORMAT` appears in `possible_values()` so the help
/// text and `[possible values: ...]` hint advertise it, but the
/// `+`-prefix shortcut means a real `+%Y-%m-%d` value never has to
/// match against the literal `+FORMAT` token.
#[derive(Clone)]
struct TimeStyleParser;

impl clap::builder::TypedValueParser for TimeStyleParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        if let Some(s) = value.to_str()
            && s.starts_with('+') {
                return Ok(s.to_string());
            }
        // `+FORMAT` is in the list so the error hint and "did you
        // mean" suggestions know about it; the `+`-prefix shortcut
        // above means a real `+%Y-%m-%d` value never has to match
        // against the literal token.
        clap::builder::PossibleValuesParser::new([
            "default", "iso", "long-iso", "full-iso", "relative", "+FORMAT",
        ])
        .parse_ref(cmd, arg, value)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            [
                PossibleValue::new("default"),
                PossibleValue::new("iso"),
                PossibleValue::new("long-iso"),
                PossibleValue::new("full-iso"),
                PossibleValue::new("relative"),
                PossibleValue::new("+FORMAT"),
            ]
            .into_iter(),
        ))
    }
}


/// `TypedValueParser` for selectors like `--dump-theme=NAME`,
/// `--dump-personality=NAME`, `--personality=NAME`, etc.
///
/// Wraps a function that returns the list of valid names (called at
/// parse time so it sees the loaded config).  Empty values are
/// always accepted (the dump flags use `default_missing_value("")`
/// to mean "list all", and that path skips the lookup).  Non-empty
/// unknown names produce a clap-formatted error with `[possible
/// values: ...]` and a "did you mean" suggestion via clap's built-in
/// error context, just like every other valued flag.
#[derive(Clone)]
struct NameListParser {
    names_fn: fn() -> Vec<String>,
    /// If true, accept the empty string (the `--dump-foo` no-value
    /// case).  Set to false for selectors that always need a name
    /// (e.g. `--personality=NAME`).
    allow_empty: bool,
}

impl NameListParser {
    const fn new(names_fn: fn() -> Vec<String>) -> Self {
        Self { names_fn, allow_empty: true }
    }

    const fn no_empty(names_fn: fn() -> Vec<String>) -> Self {
        Self { names_fn, allow_empty: false }
    }
}

impl clap::builder::TypedValueParser for NameListParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value.to_str().ok_or_else(|| {
            clap::Error::new(clap::error::ErrorKind::InvalidUtf8).with_cmd(cmd)
        })?;
        if s.is_empty() && self.allow_empty {
            return Ok(String::new());
        }
        // Delegate to PossibleValuesParser for both validation and
        // error construction.  Doing it this way (rather than
        // hand-rolling a clap::Error with ValidValue context) is what
        // wires up clap's built-in "did you mean" suggestion.
        let names = (self.names_fn)();
        let leaked: Vec<&'static str> = names
            .into_iter()
            .map(|n| -> &'static str { Box::leak(n.into_boxed_str()) })
            .collect();
        clap::builder::PossibleValuesParser::new(leaked).parse_ref(cmd, arg, value)
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let names = (self.names_fn)();
        let pvs: Vec<PossibleValue> = names
            .into_iter()
            .map(|n| {
                let leaked: &'static str = Box::leak(n.into_boxed_str());
                PossibleValue::new(leaked)
            })
            .collect();
        Some(Box::new(pvs.into_iter()))
    }
}


/// `TypedValueParser` for `--columns`.
///
/// Splits the comma-separated list, finds the first bad name, and
/// delegates that single name to a `PossibleValuesParser` so clap
/// produces its native error format complete with the "did you mean"
/// suggestion when the typo is close to a real column name.
#[derive(Clone)]
struct ColumnsParser;

impl ColumnsParser {
    fn possible_values_static() -> Vec<&'static str> {
        use crate::output::column_registry::ColumnDef;
        // Leak each name once so we can hand clap `&'static str`s.
        // The list is small and built once at parser construction.
        ColumnDef::all_names_csv()
            .split(", ")
            .map(|n| -> &'static str { Box::leak(n.to_string().into_boxed_str()) })
            .collect()
    }
}

impl clap::builder::TypedValueParser for ColumnsParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        use crate::output::table::Column;

        let s = value.to_str().ok_or_else(|| {
            clap::Error::new(clap::error::ErrorKind::InvalidUtf8).with_cmd(cmd)
        })?;
        for name in s.split(',') {
            let name = name.trim();
            if Column::from_name(name).is_none() {
                // Hand the single bad name to PossibleValuesParser
                // and propagate its rich error.
                let bad: std::ffi::OsString = name.into();
                clap::builder::PossibleValuesParser::new(Self::possible_values_static())
                    .parse_ref(cmd, arg, &bad)?;
            }
        }
        Ok(s.to_string())
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            Self::possible_values_static()
                .into_iter()
                .map(PossibleValue::new),
        ))
    }
}


/// `TypedValueParser` for `--gradient`.
///
/// Accepts a comma-separated list of column names (`size`, `date`),
/// or one of the special tokens `none` / `all`.  Tokens may not be
/// mixed with column names — `none` and `all` only make sense alone.
/// On a typo (`--gradient=siz`) clap's "did you mean" suggests the
/// closest known token via the `possible_values()` advertised here.
#[derive(Clone)]
struct GradientParser;

impl GradientParser {
    /// All tokens accepted by the parser, including `none` and `all`.
    /// Listed in `possible_values()` so clap's hint and the "did you
    /// mean" computation see them.
    const TOKENS: &'static [&'static str] = &["none", "all", "size", "date"];
}

impl clap::builder::TypedValueParser for GradientParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value.to_str().ok_or_else(|| {
            clap::Error::new(clap::error::ErrorKind::InvalidUtf8).with_cmd(cmd)
        })?;
        let mut saw_none_or_all = false;
        let mut saw_column = false;
        for tok in s.split(',') {
            let tok = tok.trim();
            match tok {
                "none" | "all" => saw_none_or_all = true,
                "size" | "date" => saw_column = true,
                _ => {
                    // Unknown token — let PossibleValuesParser
                    // construct the error so we get the same
                    // [possible values: ...] hint and "did you mean"
                    // suggestion as every other valued flag.
                    let bad: std::ffi::OsString = tok.into();
                    clap::builder::PossibleValuesParser::new(Self::TOKENS)
                        .parse_ref(cmd, arg, &bad)?;
                }
            }
        }
        if saw_none_or_all && saw_column {
            // none/all are exclusive — `--gradient=none,size` is
            // nonsense.  Build a clap-style error.
            let mut err = clap::Error::new(
                clap::error::ErrorKind::InvalidValue,
            )
            .with_cmd(cmd);
            if let Some(arg) = arg {
                err.insert(
                    clap::error::ContextKind::InvalidArg,
                    clap::error::ContextValue::String(arg.to_string()),
                );
            }
            err.insert(
                clap::error::ContextKind::InvalidValue,
                clap::error::ContextValue::String(s.to_string()),
            );
            err.insert(
                clap::error::ContextKind::ValidValue,
                clap::error::ContextValue::Strings(
                    Self::TOKENS.iter().map(|s| (*s).to_string()).collect(),
                ),
            );
            return Err(err);
        }
        Ok(s.to_string())
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            Self::TOKENS.iter().copied().map(PossibleValue::new),
        ))
    }
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
        self.matches.get_one::<String>(flag).map(std::string::String::as_str)
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
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .styles(help_styles())
        .max_term_width(80)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)
        .help_template("{name} {version}\n{about-section}\n\
                        {usage-heading}\n{tab}{usage}\n\n\
                        {all-args}{after-help}")
        .after_help("\
Column overrides:\n  \
  Every Long view column flag has a matching --no-FLAG suppressor,\n  \
  e.g. --inode pairs with --no-inode, -M pairs with --no-M\n\
\n\
Environment:\n  \
  NO_COLOR        Disable all colour output (no-color.org)\n  \
  LX_CONFIG       Explicit path to config file\n  \
  LX_PERSONALITY  Session-level personality selection\n  \
  LS_COLORS       File-type colour scheme\n  \
  TIME_STYLE      Default timestamp style")

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
            .help("Display file kind indicators [always, auto, never]")
            .help_heading("Display")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
            .value_parser([
                PossibleValue::new("always"),
                PossibleValue::new("auto"),
                PossibleValue::new("never"),
            ])
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("auto"))
        .arg(Arg::new(flags::COUNT)
            .short('C').long("count")
            .help("Print item count to stderr (-CZ includes total size)")
            .help_heading("Display")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_COUNT))

        // ── Long view columns ─────────────────────────────────────

        // Display modifiers
        .arg(Arg::new(flags::HEADER)
            .short('h').long("header")
            .help("Add a header row")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_HEADER))

        // Column-add flags — canonical column order, modifiers after enabler flags
        .arg(Arg::new(flags::INODE)
            .short('i').long("inode")
            .help("Show inode numbers")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::OCTAL)
            .short('o').long("octal")
            .alias("octal-permissions")
            .help("Show permissions in octal format\n[aliases: --octal-permissions]")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_OCTAL))
        .arg(Arg::new(flags::SHOW_PERMISSIONS)
            .short('M').long("permissions").visible_alias("mode")
            .help("Show the permissions column")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_PERMISSIONS))
        .arg(Arg::new(flags::FILE_FLAGS)
            .short('O').long("flags")
            .help("Show file flags (macOS/BSD chflags)")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LINKS)
            .short('H').long("links")
            .help("Show hard link counts")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SHOW_FILESIZE)
            .short('z').long("filesize").visible_alias("size")
            .help("Show the file size column")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_FILESIZE))
        .arg(Arg::new(flags::SIZE_STYLE)
            .long("size-style")
            .help("How to display file sizes\n[decimal (default), binary, bytes]")
            .help_heading("Long view")
            .action(ArgAction::Set)
            .value_name("STYLE")
            .value_parser(["decimal", "binary", "bytes"])
            .hide_possible_values(true)
            .overrides_with_all([flags::BINARY, flags::BYTES, flags::DECIMAL]))
        .arg(Arg::new(flags::DECIMAL)
            .short('K').long("decimal")
            .help("Decimal size prefixes (k, M, G)\n[short for --size-style=decimal]")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with_all([flags::BINARY, flags::BYTES, flags::SIZE_STYLE]))
        .arg(Arg::new(flags::BINARY)
            .short('B').long("binary")
            .help("Binary size prefixes (KiB, MiB)\n[short for --size-style=binary]")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with_all([flags::BYTES, flags::DECIMAL, flags::SIZE_STYLE]))
        .arg(Arg::new(flags::BYTES)
            .short('b').long("bytes")
            .help("Raw byte counts [short for --size-style=bytes]")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with_all([flags::BINARY, flags::DECIMAL, flags::SIZE_STYLE]))
        .arg(Arg::new(flags::TOTAL_SIZE)
            .short('Z').long("total-size")
            .help("Show directory content sizes (recursive)")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_TOTAL_SIZE))
        .arg(Arg::new(flags::BLOCKS)
            .short('S').long("blocks")
            .help("Show file system block counts")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SHOW_USER)
            .short('u').long("user")
            .help("Show the user column")
            .help_heading("Long view")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_USER))
        .arg(Arg::new(flags::UID)
            .long("uid")
            .help("Show the numeric user ID column")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::GROUP)
            .short('g').long("group")
            .help("Show the group column")
            .help_heading("Long view")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::GID)
            .long("gid")
            .help("Show the numeric group ID column")
            .help_heading("Long view")
            .action(ArgAction::Count))

        // Extras
        .arg(Arg::new(flags::EXTENDED)
            .short('@').long("extended")
            .help("Show extended attributes and sizes")
            .help_heading("Long view")
            .action(ArgAction::Count))

        // ── Filtering and sorting ─────────────────────────────────

        .arg(Arg::new(flags::ALL)
            .short('a').long("all")
            .help("Show hidden and dot files (-aa for . and ..)")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LIST_DIRS)
            .short('d').long("list-dirs")
            .help("List directories as regular files")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_DIRS)
            .short('D').long("only-dirs")
            .help("List only directories, not files")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_FILES)
            .short('f').long("only-files")
            .help("List only regular files, not directories")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::IGNORE_GLOB)
            .short('I').long("ignore")
            .visible_alias("ignore-glob")
            .help("Glob patterns (pipe-separated) of files to hide")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::SYMLINKS)
            .long("symlinks")
            .help("How to handle symlinks [show, hide, follow]")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Set)
            .value_name("MODE")
            .hide_possible_values(true)
            .value_parser([
                PossibleValue::new("show"),
                PossibleValue::new("hide"),
                PossibleValue::new("follow"),
            ]))
        .arg(Arg::new(flags::PRUNE)
            .short('P').long("prune")
            .visible_alias("prune-glob")
            .help("Glob patterns of directories to show but not recurse")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::REVERSE)
            .short('r').long("reverse")
            .help("Reverse the sort order")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SORT)
            .short('s').long("sort")
            // NOTE: this list is hand-curated to keep the --help
            // output grouped by category.  The authoritative list
            // of accepted values is in src/fs/sort_registry.rs;
            // if you add a new sort field there, add it here too.
            // `clap` uses the registry (via sort_values()) for
            // validation and error-message suggestions, so a new
            // field will still *work* without updating this text —
            // it just won't appear in --help until you do.
            .help("Sort field\n\
                   [name, Name, extension, Extension, version,\n \
                   size, blocks, links, permissions, flags,\n \
                   user, User, group, Group, uid, gid,\n \
                   modified, changed, accessed, created,\n \
                   vcs, type, inode, none]")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Set)
            .value_name("FIELD")
            .hide_possible_values(true)
            .value_parser(sort_values()))
        .arg(Arg::new(flags::GROUP_DIRS)
            .long("group-dirs")
            .help("Group directories before or after other files\n[first, last, none]")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
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
            .help("Directories first [short for --group-dirs=first]")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::SetTrue)
            .overrides_with_all([flags::GROUP_DIRS, flags::DIRS_LAST]))
        .arg(Arg::new(flags::DIRS_LAST)
            .short('J')
            .long("dirs-last")
            .alias("group-directories-last")
            .help("Directories last [short for --group-dirs=last]")
            .help_heading("Filtering & Sorting")
            .action(ArgAction::SetTrue)
            .overrides_with_all([flags::GROUP_DIRS, flags::DIRS_FIRST]))

        // ── Column / format / personality ─────────────────────────

        .arg(Arg::new(flags::PERSONALITY)
            .short('p').long("personality")
            .help("Apply a named personality (columns + flags) 🌟\n[ll, lll, la, tree, ...]")
            .help_heading("Personalities & Formats")
            .action(ArgAction::Set)
            .value_name("NAME")
            .hide_possible_values(true)
            .value_parser(NameListParser::no_empty(crate::config::all_personality_names)))
        .arg(Arg::new(flags::COLUMNS)
            .long("columns")
            .help("Explicit column list (comma-separated)")
            .help_heading("Personalities & Formats")
            .action(ArgAction::Set)
            .value_name("COLS")
            .hide_possible_values(true)
            .value_parser(ColumnsParser))
        .arg(Arg::new(flags::FORMAT)
            .long("format")
            .help("Named column format [long, long2, long3, ...]")
            .help_heading("Personalities & Formats")
            .action(ArgAction::Set)
            .value_name("NAME")
            .hide_possible_values(true)
            .value_parser({
                let names = crate::options::view::format_names();
                names.into_iter()
                    .map(|s| { let leaked: &'static str = Box::leak(s.into_boxed_str()); PossibleValue::new(leaked) })
                    .collect::<Vec<_>>()
            }))

        // ── Timestamps ────────────────────────────────────────────

        .arg(Arg::new(flags::TIME_TIER)
            .short('t')
            .help("Show timestamps — compounds like -l:\n\
                   -t adds modified, -tt adds changed,\n\
                   -ttt adds created and accessed")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::MODIFIED)
            .short('m').long("modified")
            .help("Show the modified timestamp column")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CHANGED)
            .short('c').long("changed")
            .help("Show the changed timestamp column")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ACCESSED)
            .long("accessed")
            .help("Show the accessed timestamp column")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CREATED)
            .long("created")
            .help("Show the created timestamp column")
            .help_heading("Timestamps")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME_STYLE)
            .long("time-style")
            .help("How to format timestamps\n\
            [default, iso, long-iso, full-iso,\n \
            relative, +FORMAT]")
            .help_heading("Timestamps")
            .action(ArgAction::Set)
            .value_name("STYLE")
            .hide_possible_values(true)
            .value_parser(TimeStyleParser))
        .arg(Arg::new(flags::NO_TIME)
            .long("no-time").alias("no-timestamps")
            .help("Clear all timestamp columns from the base format\n\
                   (`--no-time --accessed` shows only accessed)")
            .help_heading("Timestamps")
            .action(ArgAction::Count))

        // ── Column overrides (suppressions) ───────────────────────
        //
        // Every column-add flag has a matching --no-FLAG suppressor
        // defined below.  All of them are hidden from --help because
        // they're mechanical negations of the positive flags; the
        // convention is documented once in the after_help block.
        // They remain fully functional and documented in lx(1).

        .arg(Arg::new(flags::NO_PERMISSIONS)
            .long("no-permissions").alias("no-mode").alias("no-M")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_PERMISSIONS))
        .arg(Arg::new(flags::NO_FILESIZE)
            .long("no-filesize").alias("no-size").alias("no-z")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_FILESIZE))
        .arg(Arg::new(flags::NO_USER)
            .long("no-user").alias("no-u")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_USER))
        .arg(Arg::new(flags::NO_ICONS)
            .long("no-icons")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_INODE)
            .long("no-inode").alias("no-i")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_GROUP)
            .long("no-group").alias("no-g")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_UID)
            .long("no-uid")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::UID))
        .arg(Arg::new(flags::NO_GID)
            .long("no-gid")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::GID))
        .arg(Arg::new(flags::NO_LINKS)
            .long("no-links").alias("no-H")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_BLOCKS)
            .long("no-blocks").alias("no-S")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_FLAGS)
            .long("no-flags").alias("no-O")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::FILE_FLAGS))
        .arg(Arg::new(flags::NO_OCTAL)
            .long("no-octal").alias("no-o")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::OCTAL))
        .arg(Arg::new(flags::NO_HEADER)
            .long("no-header").alias("no-h")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::HEADER))
        .arg(Arg::new(flags::NO_COUNT)
            .long("no-count").alias("no-C")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::COUNT))
        .arg(Arg::new(flags::NO_TOTAL_SIZE)
            .long("no-total-size").alias("no-Z")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::TOTAL_SIZE))
        .arg(Arg::new(flags::NO_MODIFIED)
            .long("no-modified").alias("no-m")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::MODIFIED))
        .arg(Arg::new(flags::NO_CHANGED)
            .long("no-changed").alias("no-c")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::CHANGED))
        .arg(Arg::new(flags::NO_ACCESSED)
            .long("no-accessed")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::ACCESSED))
        .arg(Arg::new(flags::NO_CREATED)
            .long("no-created")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::CREATED))

        // ── VCS ───────────────────────────────────────────────────

        .arg(Arg::new(flags::VCS)
            .long("vcs")
            .help("VCS backend [auto, git, jj, none]")
            .help_heading("VCS")
            .action(ArgAction::Set)
            .value_name("BACKEND")
            .hide_possible_values(true)
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
        .arg(Arg::new(flags::NO_VCS_STATUS)
            .long("no-vcs-status")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::VCS_STATUS))
        .arg(Arg::new(flags::NO_VCS_REPOS)
            .long("no-vcs-repos")
            .hide(true)
            .action(ArgAction::Count)
            .overrides_with(flags::VCS_REPOS))

        // ── Appearance ────────────────────────────────────────────

        .arg(Arg::new(flags::COLOR)
            .long("colour").visible_alias("color")
            .help("When to use terminal colours\n[always, auto, never]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
            .value_parser([
                PossibleValue::new("always"),
                PossibleValue::new("auto"),
                PossibleValue::new("never"),
                PossibleValue::new("automatic").hide(true),
            ]))
        .arg(Arg::new(flags::COLOR_SCALE)
            .long("colour-scale").visible_alias("color-scale")
            .help("Colour file sizes on a scale\n[16, 256, none]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("MODE")
            .hide_possible_values(true)
            .value_parser([
                PossibleValue::new("16"),
                PossibleValue::new("256"),
                PossibleValue::new("none"),
            ])
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("16"))
        .arg(Arg::new(flags::GRADIENT)
            .long("gradient")
            .help("Per-column gradient on/off\n[size, date, all, none]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("COLUMNS")
            .hide_possible_values(true)
            .value_parser(GradientParser)
            .num_args(0..=1)
            .require_equals(true)
            .default_missing_value("all"))
        .arg(Arg::new(flags::NO_GRADIENT)
            .long("no-gradient")
            .hide(true)
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ICONS)
            .long("icons")
            .help("Display icons next to file names\n[always, auto, never]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
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
            .value_name("NAME")
            .hide_possible_values(true)
            .value_parser(NameListParser::no_empty(crate::config::all_theme_names)))
        .arg(Arg::new("hyperlink")
            .long("hyperlink")
            .help("File names as clickable hyperlinks\n[always, auto, never]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
            .default_missing_value("always")
            .require_equals(true)
            .num_args(0..=1)
            .value_parser(["always", "auto", "never"]))
        .arg(Arg::new("quotes")
            .long("quotes")
            .help("Quote file names containing spaces\n[always, auto, never]")
            .help_heading("Appearance")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .hide_possible_values(true)
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
            .require_equals(true)
            .hide_possible_values(true)
            .value_parser(NameListParser::new(crate::config::all_class_names)))
        .arg(Arg::new("dump-format")
            .long("dump-format")
            .help("Dump format definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true)
            .hide_possible_values(true)
            .value_parser(NameListParser::new(crate::options::view::format_names)))
        .arg(Arg::new("dump-personality")
            .long("dump-personality")
            .help("Dump personality definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true)
            .hide_possible_values(true)
            .value_parser(NameListParser::new(crate::config::all_personality_names)))
        .arg(Arg::new("dump-theme")
            .long("dump-theme")
            .help("Dump theme definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true)
            .hide_possible_values(true)
            .value_parser(NameListParser::new(crate::config::all_theme_names)))
        .arg(Arg::new("dump-style")
            .long("dump-style")
            .help("Dump style definitions as TOML")
            .help_heading("Configuration")
            .value_name("NAME")
            .default_missing_value("")
            .num_args(0..=1)
            .require_equals(true)
            .hide_possible_values(true)
            .value_parser(NameListParser::new(crate::config::all_style_names)))
        .arg(Arg::new("save-as")
            .long("save-as")
            .help("Save CLI flags as a personality in conf.d/")
            .help_heading("Configuration")
            .value_name("NAME")
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
            .help("Generate shell completions\n[bash, zsh, fish, elvish, powershell]")
            .help_heading("Shell completions")
            .action(ArgAction::Set)
            .value_name("SHELL")
            .hide_possible_values(true)
            .value_parser(clap::value_parser!(clap_complete::Shell)))

        // ── Hidden config-only flags ──────────────────────────────
        // These exist so personality definitions can set them via the
        // SETTING_FLAGS pipeline.  Not shown in --help.

        .arg(Arg::new("grid-rows")
            .long("grid-rows")
            .action(ArgAction::Set)
            .value_parser(clap::value_parser!(usize))
            .hide(true))
        .arg(Arg::new("icon-spacing")
            .long("icon-spacing")
            .action(ArgAction::Set)
            .value_parser(clap::value_parser!(usize))
            .hide(true))
        .arg(Arg::new("decimal-point")
            .long("decimal-point")
            .action(ArgAction::Set)
            .hide(true))
        .arg(Arg::new("thousands-separator")
            .long("thousands-separator")
            .action(ArgAction::Set)
            .hide(true))

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
