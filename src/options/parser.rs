//! CLI argument parsing using Clap 4.5.
//!
//! Clap handles validation, help text, version display, and "last flag wins"
//! semantics via `overrides_with`.  The `MatchedFlags` wrapper provides a
//! simple query API for the deduce functions.

use std::ffi::OsString;

use clap::builder::{PossibleValue, styling};

use crate::options::flags;


/// Clap help-text styling matching the owner's preferred colour scheme.
const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Yellow.on_default().bold())
    .usage(styling::AnsiColor::Yellow.on_default().bold())
    .literal(styling::AnsiColor::Cyan.on_default().bold())
    .placeholder(styling::AnsiColor::Green.on_default())
    .error(styling::AnsiColor::Red.on_default().bold());


/// Thin wrapper around `clap::ArgMatches` that the deduce functions query.
#[derive(Debug)]
pub struct MatchedFlags(clap::ArgMatches);

impl MatchedFlags {
    /// Wrap parsed `ArgMatches`.
    pub fn new(matches: clap::ArgMatches) -> Self {
        Self(matches)
    }

    /// Whether the given flag was specified at all.
    pub fn has(&self, id: &str) -> bool {
        self.0.get_count(id) > 0
    }

    /// Return the value of a flag that takes a parameter, or `None` if the
    /// flag was not given.
    pub fn get(&self, id: &str) -> Option<&str> {
        self.0.get_one::<String>(id).map(String::as_str)
    }

    /// Number of times a flag was given (useful for `-a` / `-aa`).
    pub fn count(&self, id: &str) -> u8 {
        self.0.get_count(id)
    }

    /// Return the value of a flag parsed as `usize`, or `None` if absent.
    pub fn get_usize(&self, id: &str) -> Option<usize> {
        self.0.get_one::<usize>(id).copied()
    }
}


/// Build the set of accepted `--sort` values, with most aliases hidden.
fn sort_values() -> clap::builder::PossibleValuesParser {
    let mut vals = vec![
        // Shown in help
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
        // Hidden aliases
        PossibleValue::new("filename").hide(true),
        PossibleValue::new("Filename").hide(true),
        PossibleValue::new(".name").hide(true),
        PossibleValue::new(".Name").hide(true),
        PossibleValue::new(".filename").hide(true),
        PossibleValue::new(".Filename").hide(true),
        PossibleValue::new("filesize").hide(true),
        PossibleValue::new("ext").hide(true),
        PossibleValue::new("Ext").hide(true),
        PossibleValue::new("date").hide(true),
        PossibleValue::new("time").hide(true),
        PossibleValue::new("mod").hide(true),
        PossibleValue::new("new").hide(true),
        PossibleValue::new("newest").hide(true),
        PossibleValue::new("age").hide(true),
        PossibleValue::new("old").hide(true),
        PossibleValue::new("oldest").hide(true),
        PossibleValue::new("ch").hide(true),
        PossibleValue::new("acc").hide(true),
        PossibleValue::new("cr").hide(true),
    ];

    #[cfg(unix)]
    vals.push(PossibleValue::new("inode"));

    clap::builder::PossibleValuesParser::new(vals)
}


/// Build the Clap command with all flag definitions, override groups, and
/// aliases.
pub fn build_command() -> clap::Command {
    use clap::{Arg, ArgAction};

    clap::Command::new("lx")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .styles(STYLES)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)
        .after_help("\
Environment variables:\n  \
  COLUMNS          Override terminal width (characters)\n  \
  LX_GRID_ROWS    Minimum rows before grid-details view activates\n  \
  LX_ICON_SPACING Spaces between icon and file name\n  \
  NO_COLOR         Disable colours (overridden by --colour)\n  \
  LS_COLORS        File-type colour scheme\n  \
  LX_COLORS       Extended colour scheme (UI elements and metadata)\n  \
  TIME_STYLE       Default timestamp style (overridden by --time-style)")

        // ── Display mode flags ──────────────────────────────────

        .arg(Arg::new(flags::ONE_LINE)
            .short('1').long("oneline")
            .help("Display one entry per line")
            .action(ArgAction::Count)
            .overrides_with_all([flags::LONG, flags::GRID]))
        .arg(Arg::new(flags::LONG)
            .short('l').long("long")
            .help("Display extended file metadata as a table")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::GRID)
            .short('G').long("grid")
            .help("Display entries as a grid (default)")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::ACROSS)
            .short('x').long("across")
            .help("Sort the grid across, rather than downwards")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::RECURSE)
            .short('R').long("recurse")
            .help("Recurse into directories")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TREE)
            .short('T').long("tree")
            .help("Recurse into directories as a tree")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CLASSIFY)
            .long("classify")
            .help("Display file kind indicators next to file names")
            .action(ArgAction::Count))

        // ── Colour ──────────────────────────────────────────────

        .arg(Arg::new(flags::COLOR)
            .long("colour").visible_alias("color")
            .help("When to use terminal colours")
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
            .action(ArgAction::Count))

        // ── Filtering and sorting ───────────────────────────────

        .arg(Arg::new(flags::ALL)
            .short('a').long("all")
            .help("Show hidden and dot files (-aa for . and ..)")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LIST_DIRS)
            .short('d').long("list-dirs")
            .help("List directories as regular files")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LEVEL)
            .short('L').long("level")
            .help("Limit the depth of recursion")
            .action(ArgAction::Set)
            .value_name("DEPTH")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new(flags::REVERSE)
            .short('r').long("reverse")
            .help("Reverse the sort order")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SORT)
            .short('s').long("sort")
            .help("Sort field")
            .action(ArgAction::Set)
            .value_name("FIELD")
            .value_parser(sort_values()))
        .arg(Arg::new(flags::IGNORE_GLOB)
            .short('I').long("ignore-glob")
            .help("Glob patterns (pipe-separated) of files to ignore")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::GIT_IGNORE)
            .long("git-ignore")
            .help("Hide files ignored by VCS (alias for --vcs-ignore)")
            .action(ArgAction::Count)
            .hide(true))
        .arg(Arg::new(flags::GROUP_DIRS)
            .long("group-dirs")
            .help("Group directories before or after other files")
            .action(ArgAction::Set)
            .value_name("WHEN")
            .value_parser([
                PossibleValue::new("first"),
                PossibleValue::new("last"),
                PossibleValue::new("none"),
            ]))
        .arg(Arg::new(flags::DIRS_FIRST)
            .long("group-directories-first")
            .help("List directories first (alias for --group-dirs=first)")
            .action(ArgAction::Count)
            .hide(true))
        .arg(Arg::new(flags::ONLY_DIRS)
            .short('D').long("only-dirs")
            .help("List only directories, not files")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_FILES)
            .short('f').long("only-files")
            .help("List only regular files, not directories")
            .action(ArgAction::Count))

        // ── Long-view detail columns ────────────────────────────

        .arg(Arg::new(flags::BINARY)
            .short('b').long("binary")
            .help("List file sizes with binary prefixes")
            .action(ArgAction::Count)
            .overrides_with(flags::BYTES))
        .arg(Arg::new(flags::BYTES)
            .short('B').long("bytes")
            .help("List file sizes in bytes, without prefixes")
            .action(ArgAction::Count)
            .overrides_with(flags::BINARY))
        .arg(Arg::new(flags::GROUP)
            .short('g').long("group")
            .help("List each file's group")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NUMERIC)
            .short('n').long("numeric")
            .help("List numeric user and group IDs")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::HEADER)
            .short('h').long("header")
            .help("Add a header row to each column")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ICONS)
            .long("icons")
            .help("Display icons next to file names")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::INODE)
            .short('i').long("inode")
            .help("List each file's inode number")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LINKS)
            .short('H').long("links")
            .help("List each file's number of hard links")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::MODIFIED)
            .short('m').long("modified")
            .help("Use the modified timestamp field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CHANGED)
            .short('c').long("changed")
            .help("Use the changed timestamp field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::BLOCKS)
            .short('S').long("blocks")
            .help("List each file's number of file system blocks")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME)
            .short('t').long("time")
            .help("Which timestamp field to display")
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
        .arg(Arg::new(flags::ACCESSED)
            .short('u').long("accessed")
            .help("Use the accessed timestamp field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CREATED)
            .short('U').long("created")
            .help("Use the created timestamp field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME_STYLE)
            .long("time-style")
            .help("How to format timestamps")
            .action(ArgAction::Set)
            .value_name("STYLE")
            .value_parser([
                PossibleValue::new("default"),
                PossibleValue::new("iso"),
                PossibleValue::new("long-iso"),
                PossibleValue::new("full-iso"),
            ]))

        // ── Column visibility ─────────────────────────────────

        .arg(Arg::new(flags::SHOW_PERMISSIONS)
            .long("permissions")
            .help("Show the permissions field")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_PERMISSIONS))
        .arg(Arg::new(flags::NO_PERMISSIONS)
            .long("no-permissions")
            .help("Suppress the permissions field")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_PERMISSIONS))
        .arg(Arg::new(flags::SHOW_FILESIZE)
            .long("filesize")
            .help("Show the file size field")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_FILESIZE))
        .arg(Arg::new(flags::NO_FILESIZE)
            .long("no-filesize")
            .help("Suppress the file size field")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_FILESIZE))
        .arg(Arg::new(flags::SHOW_USER)
            .long("user")
            .help("Show the user field")
            .action(ArgAction::Count)
            .overrides_with(flags::NO_USER))
        .arg(Arg::new(flags::NO_USER)
            .long("no-user")
            .help("Suppress the user field")
            .action(ArgAction::Count)
            .overrides_with(flags::SHOW_USER))
        .arg(Arg::new(flags::NO_TIME)
            .long("no-time")
            .help("Suppress the time field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_ICONS)
            .long("no-icons")
            .help("Don't display icons (overrides --icons)")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_INODE)
            .long("no-inode")
            .help("Suppress the inode field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_GROUP)
            .long("no-group")
            .help("Suppress the group field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_LINKS)
            .long("no-links")
            .help("Suppress the hard links field")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_BLOCKS)
            .long("no-blocks")
            .help("Suppress the blocks field")
            .action(ArgAction::Count))

        // ── Column / format selection ─────────────────────────

        .arg(Arg::new(flags::COLUMNS)
            .long("columns")
            .help("Comma-separated list of columns to display [perms, size, user, group, links, inode, blocks, octal, modified, changed, accessed, created, vcs]")
            .action(ArgAction::Set)
            .value_name("COLS"))
        .arg(Arg::new(flags::FORMAT)
            .long("format")
            .help("Named column format")
            .action(ArgAction::Set)
            .value_name("NAME")
            .value_parser({
                let names = crate::options::view::format_names();
                // Leak the strings so Clap can hold &'static str references.
                // This is called once at startup; the small leak is acceptable.
                names.into_iter()
                    .map(|s| { let leaked: &'static str = Box::leak(s.into_boxed_str()); PossibleValue::new(leaked) })
                    .collect::<Vec<_>>()
            }))

        .arg(Arg::new(flags::PERSONALITY)
            .short('p').long("personality")
            .help("Apply a named personality (columns + flags)")
            .action(ArgAction::Set)
            .value_name("NAME"))

        // ── VCS integration ────────────────────────────────────

        .arg(Arg::new(flags::VCS)
            .long("vcs")
            .help("VCS backend to use")
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
            .action(ArgAction::Count))
        .arg(Arg::new(flags::VCS_IGNORE)
            .long("vcs-ignore")
            .help("Hide files ignored by VCS")
            .action(ArgAction::Count))

        // ── Optional features ───────────────────────────────────

        .arg(Arg::new(flags::GIT)
            .long("git")
            .help("Show per-file Git status (alias for --vcs-status)")
            .action(ArgAction::Count)
            .hide(true))
        .arg(Arg::new(flags::EXTENDED)
            .short('@').long("extended")
            .help("List each file's extended attributes and sizes")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::OCTAL)
            .long("octal-permissions")
            .help("List each file's permissions in octal format")
            .action(ArgAction::Count))

        // ── Help & version ──────────────────────────────────────

        .arg(Arg::new("help")
            .short('?').long("help")
            .help("Print help information")
            .action(ArgAction::HelpShort))
        .arg(Arg::new("version")
            .short('v').long("version")
            .help("Print version information")
            .action(ArgAction::Version))
        .arg(Arg::new("completions")
            .long("completions")
            .help("Generate shell completions and exit")
            .action(ArgAction::Set)
            .value_name("SHELL")
            .value_parser(clap::value_parser!(clap_complete::Shell))
            .hide(true))

        // ── Positional file arguments ───────────────────────────

        .arg(Arg::new("FILE")
            .action(ArgAction::Append)
            .value_parser(clap::value_parser!(OsString))
            .num_args(0..))
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn has_flag() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--long"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert!(mf.has(flags::LONG));
        assert!(!mf.has(flags::GRID));
    }

    #[test]
    fn get_value() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--sort", "name"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert_eq!(mf.get(flags::SORT), Some("name"));
    }

    #[test]
    fn count_all() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "-aa"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert_eq!(mf.count(flags::ALL), 2);
    }

    #[test]
    fn override_long_oneline() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--long", "--oneline"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert!(!mf.has(flags::LONG));
        assert!(mf.has(flags::ONE_LINE));
    }

    #[test]
    fn override_oneline_long() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--oneline", "--long"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert!(mf.has(flags::LONG));
        assert!(!mf.has(flags::ONE_LINE));
    }

    #[test]
    fn colour_alias() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--colour=always"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert_eq!(mf.get(flags::COLOR), Some("always"));
    }

    #[test]
    fn frees() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "foo", "bar"]).unwrap();
        let frees: Vec<OsString> = m.get_many::<OsString>("FILE")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default();
        assert_eq!(frees, vec![OsString::from("foo"), OsString::from("bar")]);
    }

    #[test]
    fn binary_overrides_bytes() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--bytes", "--binary"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert!(mf.has(flags::BINARY));
        assert!(!mf.has(flags::BYTES));
    }

    #[test]
    fn bytes_overrides_binary() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["lx", "--binary", "--bytes"]).unwrap();
        let mf = MatchedFlags::new(m);
        assert!(!mf.has(flags::BINARY));
        assert!(mf.has(flags::BYTES));
    }
}
