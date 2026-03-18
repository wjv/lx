//! Parsing the options for `DirAction`.

use crate::options::parser::MatchedFlags;
use crate::options::{flags, OptionsError};

use crate::fs::dir_action::{DirAction, RecurseOptions};


impl DirAction {

    /// Determine which action to perform when trying to list a directory.
    /// There are three possible actions, and they overlap somewhat: the
    /// `--tree` flag is another form of recursion, so those two are allowed
    /// to both be present, but the `--list-dirs` flag is used separately.
    pub fn deduce(matches: &MatchedFlags, can_tree: bool) -> Result<Self, OptionsError> {
        let recurse = matches.has(flags::RECURSE);
        let as_file = matches.has(flags::LIST_DIRS);
        let tree    = matches.has(flags::TREE);

        if tree && can_tree {
            // Tree is only appropriate in details mode, so this has to
            // examine the View, which should have already been deduced by now
            Ok(Self::Recurse(RecurseOptions::deduce(matches, true)?))
        }
        else if recurse {
            Ok(Self::Recurse(RecurseOptions::deduce(matches, false)?))
        }
        else if as_file {
            Ok(Self::AsFile)
        }
        else {
            Ok(Self::List)
        }
    }
}


impl RecurseOptions {

    /// Determine which files should be recursed into, based on the `--level`
    /// flag's value, and whether the `--tree` flag was passed, which was
    /// determined earlier. The maximum level should be a number, and this
    /// will fail with an `Err` if it isn't.
    pub fn deduce(matches: &MatchedFlags, tree: bool) -> Result<Self, OptionsError> {
        // Clap validates --level as a usize at parse time.
        let max_depth = matches.get_usize(flags::LEVEL);
        Ok(Self { tree, max_depth })
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
    test!(empty:           DirAction <- [];               Ok(DirAction::List));

    // Listing files as directories
    test!(dirs_short:      DirAction <- ["-d"];           Ok(DirAction::AsFile));
    test!(dirs_long:       DirAction <- ["--list-dirs"];  Ok(DirAction::AsFile));

    // Recursing
    use self::DirAction::Recurse;
    test!(rec_short:       DirAction <- ["-R"];                           Ok(Recurse(RecurseOptions { tree: false, max_depth: None })));
    test!(rec_long:        DirAction <- ["--recurse"];                    Ok(Recurse(RecurseOptions { tree: false, max_depth: None })));
    test!(rec_lim_short:   DirAction <- ["-RL4"];                         Ok(Recurse(RecurseOptions { tree: false, max_depth: Some(4) })));
    test!(rec_lim_short_2: DirAction <- ["-RL=5"];                        Ok(Recurse(RecurseOptions { tree: false, max_depth: Some(5) })));
    test!(rec_lim_long:    DirAction <- ["--recurse", "--level", "666"];  Ok(Recurse(RecurseOptions { tree: false, max_depth: Some(666) })));
    test!(rec_lim_long_2:  DirAction <- ["--recurse", "--level=0118"];    Ok(Recurse(RecurseOptions { tree: false, max_depth: Some(118) })));
    test!(tree:            DirAction <- ["--tree"];                       Ok(Recurse(RecurseOptions { tree: true,  max_depth: None })));
    test!(rec_tree:        DirAction <- ["--recurse", "--tree"];          Ok(Recurse(RecurseOptions { tree: true,  max_depth: None })));
    test!(rec_short_tree:  DirAction <- ["-TR"];                          Ok(Recurse(RecurseOptions { tree: true,  max_depth: None })));

    // Overriding --list-dirs, --recurse, and --tree
    test!(dirs_recurse:    DirAction <- ["--list-dirs", "--recurse"];     Ok(Recurse(RecurseOptions { tree: false, max_depth: None })));
    test!(dirs_tree:       DirAction <- ["--list-dirs", "--tree"];        Ok(Recurse(RecurseOptions { tree: true,  max_depth: None })));
    test!(just_level:      DirAction <- ["--level=4"];                    Ok(DirAction::List));

    // Overriding levels
    test!(overriding_1:    DirAction <- ["-RL=6", "-L=7"];               Ok(Recurse(RecurseOptions { tree: false, max_depth: Some(7) })));
}
