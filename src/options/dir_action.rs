//! Parsing the options for `DirAction`.

use crate::options::flags;
use crate::options::parser::MatchedFlags;

use crate::fs::dir_action::{DirAction, Filesystem, RecurseOptions};

impl DirAction {
    /// Determine which action to perform when trying to list a directory.
    /// There are three possible actions, and they overlap somewhat: the
    /// `--tree` flag is another form of recursion, so those two are allowed
    /// to both be present, but the `--list-dirs` flag is used separately.
    pub fn deduce(matches: &MatchedFlags, can_tree: bool) -> Self {
        let recurse = matches.has(flags::RECURSE);
        let as_file = matches.has(flags::LIST_DIRS);
        let tree = matches.has(flags::TREE);

        if tree && can_tree {
            // Tree is only appropriate in details mode, so this has to
            // examine the View, which should have already been deduced by now
            Self::Recurse(RecurseOptions::deduce(matches, true))
        } else if recurse {
            Self::Recurse(RecurseOptions::deduce(matches, false))
        } else if as_file {
            Self::AsFile
        } else {
            Self::List
        }
    }
}

impl RecurseOptions {
    /// Determine which files should be recursed into, based on the `--level`
    /// flag's value, and whether the `--tree` flag was passed, which was
    /// determined earlier.
    pub fn deduce(matches: &MatchedFlags, tree: bool) -> Self {
        // Clap validates --level as a usize at parse time.
        let max_depth = matches.get_usize(flags::LEVEL);
        let filesystem = Filesystem::deduce(matches);
        Self {
            tree,
            max_depth,
            filesystem,
        }
    }
}

impl Filesystem {
    fn deduce(matches: &MatchedFlags) -> Self {
        // --no-* takes precedence over both --filesystem=MODE and -X.
        if matches.has(flags::NO_FILESYSTEM) {
            return Self::All;
        }
        // -X / --xdev: short for --filesystem=same.
        if matches.has(flags::XDEV) {
            return Self::Same;
        }
        match matches.get(flags::FILESYSTEM) {
            Some("same") => Self::Same,
            Some("local") => Self::Local,
            _ => Self::All,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    macro_rules! test {
        ($name:ident: $type:ident <- $inputs:expr; $result:expr) => {
            #[test]
            fn $name() {
                use crate::options::test::parse_for_test;
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf, true)) {
                    assert_eq!(result, $result);
                }
            }
        };
    }

    // Default behaviour
    test!(empty:           DirAction <- [];               DirAction::List);

    // Listing files as directories
    test!(dirs_short:      DirAction <- ["-d"];           DirAction::AsFile);
    test!(dirs_long:       DirAction <- ["--list-dirs"];  DirAction::AsFile);

    // Recursing
    use self::DirAction::Recurse;
    test!(rec_short:       DirAction <- ["-R"];                           Recurse(RecurseOptions { tree: false, max_depth: None, filesystem: Filesystem::All }));
    test!(rec_long:        DirAction <- ["--recurse"];                    Recurse(RecurseOptions { tree: false, max_depth: None, filesystem: Filesystem::All }));
    test!(rec_lim_short:   DirAction <- ["-RL4"];                         Recurse(RecurseOptions { tree: false, max_depth: Some(4), filesystem: Filesystem::All }));
    test!(rec_lim_short_2: DirAction <- ["-RL=5"];                        Recurse(RecurseOptions { tree: false, max_depth: Some(5), filesystem: Filesystem::All }));
    test!(rec_lim_long:    DirAction <- ["--recurse", "--level", "666"];  Recurse(RecurseOptions { tree: false, max_depth: Some(666), filesystem: Filesystem::All }));
    test!(rec_lim_long_2:  DirAction <- ["--recurse", "--level=0118"];    Recurse(RecurseOptions { tree: false, max_depth: Some(118), filesystem: Filesystem::All }));
    test!(tree:            DirAction <- ["--tree"];                       Recurse(RecurseOptions { tree: true, max_depth: None, filesystem: Filesystem::All }));
    test!(rec_tree:        DirAction <- ["--recurse", "--tree"];          Recurse(RecurseOptions { tree: true, max_depth: None, filesystem: Filesystem::All }));
    test!(rec_short_tree:  DirAction <- ["-TR"];                          Recurse(RecurseOptions { tree: true, max_depth: None, filesystem: Filesystem::All }));

    // Overriding --list-dirs, --recurse, and --tree
    test!(dirs_recurse:    DirAction <- ["--list-dirs", "--recurse"];     Recurse(RecurseOptions { tree: false, max_depth: None, filesystem: Filesystem::All }));
    test!(dirs_tree:       DirAction <- ["--list-dirs", "--tree"];        Recurse(RecurseOptions { tree: true, max_depth: None, filesystem: Filesystem::All }));
    test!(just_level:      DirAction <- ["--level=4"];                    DirAction::List);

    // Overriding levels
    test!(overriding_1:    DirAction <- ["-RL=6", "-L=7"];               Recurse(RecurseOptions { tree: false, max_depth: Some(7), filesystem: Filesystem::All }));
}
