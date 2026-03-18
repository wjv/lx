//! CLI argument parsing using Clap 4.5.
//!
//! Clap handles validation, help text, version display, and "last flag wins"
//! semantics via `overrides_with`.  The `MatchedFlags` wrapper provides a
//! simple query API for the deduce functions.

use std::ffi::OsString;

use crate::options::flags;


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
}


/// Build the Clap command with all flag definitions, override groups, and
/// aliases.
pub fn build_command() -> clap::Command {
    use clap::{Arg, ArgAction};

    clap::Command::new("lx")
        .version(include_str!(concat!(env!("OUT_DIR"), "/version_string.txt")))
        .about("list extended (but call me Alex!)")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .args_override_self(true)

        // ── Display mode flags ──────────────────────────────────

        .arg(Arg::new(flags::ONE_LINE)
            .short('1').long("oneline")
            .action(ArgAction::Count)
            .overrides_with_all([flags::LONG, flags::GRID]))
        .arg(Arg::new(flags::LONG)
            .short('l').long("long")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::GRID)
            .short('G').long("grid")
            .action(ArgAction::Count)
            .overrides_with(flags::ONE_LINE))
        .arg(Arg::new(flags::ACROSS)
            .short('x').long("across")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::RECURSE)
            .short('R').long("recurse")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TREE)
            .short('T').long("tree")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CLASSIFY)
            .short('F').long("classify")
            .action(ArgAction::Count))

        // ── Colour ──────────────────────────────────────────────

        .arg(Arg::new(flags::COLOR)
            .long("color").visible_alias("colour")
            .action(ArgAction::Set)
            .value_name("WHEN"))
        .arg(Arg::new(flags::COLOR_SCALE)
            .long("color-scale").visible_alias("colour-scale")
            .action(ArgAction::Count))

        // ── Filtering and sorting ───────────────────────────────

        .arg(Arg::new(flags::ALL)
            .short('a').long("all")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LIST_DIRS)
            .short('d').long("list-dirs")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LEVEL)
            .short('L').long("level")
            .action(ArgAction::Set)
            .value_name("DEPTH"))
        .arg(Arg::new(flags::REVERSE)
            .short('r').long("reverse")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::SORT)
            .short('s').long("sort")
            .action(ArgAction::Set)
            .value_name("FIELD"))
        .arg(Arg::new(flags::IGNORE_GLOB)
            .short('I').long("ignore-glob")
            .action(ArgAction::Set)
            .value_name("GLOB"))
        .arg(Arg::new(flags::GIT_IGNORE)
            .long("git-ignore")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::DIRS_FIRST)
            .long("group-directories-first")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ONLY_DIRS)
            .short('D').long("only-dirs")
            .action(ArgAction::Count))

        // ── Long-view detail columns ────────────────────────────

        .arg(Arg::new(flags::BINARY)
            .short('b').long("binary")
            .action(ArgAction::Count)
            .overrides_with(flags::BYTES))
        .arg(Arg::new(flags::BYTES)
            .short('B').long("bytes")
            .action(ArgAction::Count)
            .overrides_with(flags::BINARY))
        .arg(Arg::new(flags::GROUP)
            .short('g').long("group")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NUMERIC)
            .short('n').long("numeric")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::HEADER)
            .short('h').long("header")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::ICONS)
            .long("icons")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::INODE)
            .short('i').long("inode")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::LINKS)
            .short('H').long("links")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::MODIFIED)
            .short('m').long("modified")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CHANGED)
            .long("changed")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::BLOCKS)
            .short('S').long("blocks")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME)
            .short('t').long("time")
            .action(ArgAction::Set)
            .value_name("FIELD"))
        .arg(Arg::new(flags::ACCESSED)
            .short('u').long("accessed")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::CREATED)
            .short('U').long("created")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::TIME_STYLE)
            .long("time-style")
            .action(ArgAction::Set)
            .value_name("STYLE"))

        // ── Suppressing columns ─────────────────────────────────

        .arg(Arg::new(flags::NO_PERMISSIONS)
            .long("no-permissions")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_FILESIZE)
            .long("no-filesize")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_USER)
            .long("no-user")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_TIME)
            .long("no-time")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::NO_ICONS)
            .long("no-icons")
            .action(ArgAction::Count))

        // ── Optional features ───────────────────────────────────

        .arg(Arg::new(flags::GIT)
            .long("git")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::EXTENDED)
            .short('@').long("extended")
            .action(ArgAction::Count))
        .arg(Arg::new(flags::OCTAL)
            .long("octal-permissions")
            .action(ArgAction::Count))

        // ── Help & version ──────────────────────────────────────

        .arg(Arg::new("help")
            .short('?').long("help")
            .action(ArgAction::HelpShort))
        .arg(Arg::new("version")
            .short('v').long("version")
            .action(ArgAction::Version))

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
