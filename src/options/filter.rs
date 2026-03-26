//! Parsing the options for `FileFilter`.

use crate::fs::DotFilter;
use crate::fs::filter::{FileFilter, SortField, SortCase, GroupDirs, IgnorePatterns, VcsIgnore};

use crate::options::{flags, OptionsError};
use crate::options::parser::MatchedFlags;


impl FileFilter {

    /// Determines which of all the file filter options to use.
    pub fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        Ok(Self {
            group_dirs:       GroupDirs::deduce(matches),
            reverse:          matches.has(flags::REVERSE),
            only_dirs:        matches.has(flags::ONLY_DIRS),
            only_files:       matches.has(flags::ONLY_FILES),
            sort_field:       SortField::deduce(matches)?,
            sort_by_total_size: matches.has(flags::TOTAL_SIZE),
            dot_filter:       DotFilter::deduce(matches)?,
            ignore_patterns:  IgnorePatterns::deduce(matches)?,
            prune_patterns:   IgnorePatterns::deduce_from(matches, flags::PRUNE)?,
            vcs_ignore:       VcsIgnore::deduce(matches),
        })
    }
}

impl GroupDirs {
    fn deduce(matches: &MatchedFlags) -> Self {
        // --group-dirs=first|last|none takes priority
        if let Some(word) = matches.get(flags::GROUP_DIRS) {
            return match word {
                "first" => Self::First,
                "last"  => Self::Last,
                "none"  => Self::None,
                _       => unreachable!("Clap rejects invalid --group-dirs values"),
            };
        }

        // Short flags: -F (first), -J (last)
        if matches.has(flags::DIRS_FIRST) {
            return Self::First;
        }
        if matches.has(flags::DIRS_LAST) {
            return Self::Last;
        }

        Self::None
    }
}

impl SortField {

    /// Determines which sort field to use based on the `--sort` argument.
    /// This argument's value can be one of several flags, listed above.
    /// Returns the default sort field if none is given, or `Err` if the
    /// value doesn't correspond to a sort field we know about.
    fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        let word = match matches.get(flags::SORT) {
            Some(w)  => w,
            None     => return Ok(Self::default()),
        };

        let field = match word {
            "name" | "filename" => {
                Self::Name(SortCase::AaBbCc)
            }
            "Name" | "Filename" => {
                Self::Name(SortCase::ABCabc)
            }
            ".name" | ".filename" => {
                Self::NameMixHidden(SortCase::AaBbCc)
            }
            ".Name" | ".Filename" => {
                Self::NameMixHidden(SortCase::ABCabc)
            }
            "size" | "filesize" => {
                Self::Size
            }
            "ext" | "extension" => {
                Self::Extension(SortCase::AaBbCc)
            }
            "Ext" | "Extension" => {
                Self::Extension(SortCase::ABCabc)
            }

            // "new" sorts oldest at the top and newest at the bottom; "old"
            // sorts newest at the top and oldest at the bottom. I think this
            // is the right way round to do this: "size" puts the smallest at
            // the top and the largest at the bottom, doesn't it?
            "date" | "time" | "mod" | "modified" | "new" | "newest" => {
                Self::ModifiedDate
            }

            // Similarly, "age" means that files with the least age (the
            // newest files) get sorted at the top, and files with the most
            // age (the oldest) at the bottom.
            "age" | "old" | "oldest" => {
                Self::ModifiedAge
            }

            "ch" | "changed" => {
                Self::ChangedDate
            }
            "acc" | "accessed" => {
                Self::AccessedDate
            }
            "cr" | "created" => {
                Self::CreatedDate
            }
            #[cfg(unix)]
            "inode" => {
                Self::FileInode
            }
            "type" => {
                Self::FileType
            }
            "none" => {
                Self::Unsorted
            }
            _ => unreachable!("Clap rejects invalid --sort values"),
        };

        Ok(field)
    }
}


// I've gone back and forth between whether to sort case-sensitively or
// insensitively by default. The default string sort in most programming
// languages takes each character's ASCII value into account, sorting
// "Documents" before "apps", but there's usually an option to ignore
// characters' case, putting "apps" before "Documents".
//
// The argument for following case is that it's easy to forget whether an item
// begins with an uppercase or lowercase letter and end up having to scan both
// the uppercase and lowercase sub-lists to find the item you want. If you
// happen to pick the sublist it's not in, it looks like it's missing, which
// is worse than if you just take longer to find it.
// (https://ux.stackexchange.com/a/79266)
//
// The argument for ignoring case is that it makes lx sort files differently
// from shells. A user would expect a directory's files to be in the same
// order if they used "lx ~/directory" or "lx ~/directory/*", but lx sorts
// them in the first case, and the shell in the second case, so they wouldn't
// be exactly the same if lx does something non-conventional.
//
// However, lx already sorts files differently: it uses natural sorting,
// placing "2" before "10" because the number is smaller.  Users name their
// files with numbers expecting them to be treated as numbers, not as lists
// of ASCII characters.
//
// In the same way, users name their files with letters expecting the order
// of the letters to matter, not each character's ASCII value.  So lx
// (following exa's lead) breaks from tradition and ignores case while
// sorting: "apps" first, then "Documents".
//
// You can get the old behaviour back by sorting with `--sort=Name`.
impl Default for SortField {
    fn default() -> Self {
        Self::Name(SortCase::AaBbCc)
    }
}


impl DotFilter {

    /// Determines the dot filter based on how many `--all` options were
    /// given: one will show dotfiles, but two will show `.` and `..` too.
    ///
    /// It also checks for the `--tree` option, because of a special case
    /// where `--tree --all --all` won't work: listing the parent directory
    /// in tree mode would loop onto itself!
    pub fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        let count = matches.count(flags::ALL);

        if count == 0 {
            Ok(Self::JustFiles)
        }
        else if count == 1 {
            Ok(Self::Dotfiles)
        }
        else if matches.has(flags::TREE) {
            Err(OptionsError::TreeAllAll)
        }
        else {
            Ok(Self::DotfilesAndDots)
        }
    }
}


impl IgnorePatterns {

    /// Determines the set of glob patterns to use based on the
    /// `--ignore-glob` argument's value. This is a list of strings
    /// separated by pipe (`|`) characters, given in any order.
    pub fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        Self::deduce_from(matches, flags::IGNORE_GLOB)
    }

    /// Parse glob patterns from an arbitrary flag (used for both
    /// `--ignore-glob` and `--prune`).
    pub fn deduce_from(matches: &MatchedFlags, flag: &str) -> Result<Self, OptionsError> {
        let inputs = match matches.get(flag) {
            Some(is)  => is,
            None      => return Ok(Self::empty()),
        };

        let (patterns, mut errors) = Self::parse_from_iter(inputs.split('|'));

        match errors.pop() {
            Some(e)  => Err(e.into()),
            None     => Ok(patterns),
        }
    }
}


impl VcsIgnore {
    pub fn deduce(matches: &MatchedFlags) -> Self {
        if matches.has(flags::VCS_IGNORE) {
            Self::CheckAndIgnore
        }
        else {
            Self::Off
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
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf)) {
                    assert_eq!(result, $result);
                }
            }
        };
    }

    mod sort_fields {
        use super::*;

        // Default behaviour
        test!(empty:         SortField <- [];                  Ok(SortField::default()));

        // Sort field arguments
        test!(one_arg:       SortField <- ["--sort=mod"];       Ok(SortField::ModifiedDate));
        test!(one_long:      SortField <- ["--sort=size"];     Ok(SortField::Size));
        test!(one_short:     SortField <- ["-saccessed"];      Ok(SortField::AccessedDate));
        test!(lowercase:     SortField <- ["--sort", "name"];  Ok(SortField::Name(SortCase::AaBbCc)));
        test!(uppercase:     SortField <- ["--sort", "Name"];  Ok(SortField::Name(SortCase::ABCabc)));
        test!(old:           SortField <- ["--sort", "new"];   Ok(SortField::ModifiedDate));
        test!(oldest:        SortField <- ["--sort=newest"];   Ok(SortField::ModifiedDate));
        test!(new:           SortField <- ["--sort", "old"];   Ok(SortField::ModifiedAge));
        test!(newest:        SortField <- ["--sort=oldest"];   Ok(SortField::ModifiedAge));
        test!(age:           SortField <- ["-sage"];           Ok(SortField::ModifiedAge));

        test!(mix_hidden_lowercase:     SortField <- ["--sort", ".name"];  Ok(SortField::NameMixHidden(SortCase::AaBbCc)));
        test!(mix_hidden_uppercase:     SortField <- ["--sort", ".Name"];  Ok(SortField::NameMixHidden(SortCase::ABCabc)));

        // Errors — Clap rejects invalid values at parse time
        #[test]
        fn error() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--sort=colour"]).is_err());
        }

        // Overriding
        test!(overridden:    SortField <- ["--sort=cr",       "--sort", "mod"];     Ok(SortField::ModifiedDate));
        test!(overridden_2:  SortField <- ["--sort", "none",  "--sort=Extension"];  Ok(SortField::Extension(SortCase::ABCabc)));
    }


    mod dot_filters {
        use super::*;

        // Default behaviour
        test!(empty:      DotFilter <- [];               Ok(DotFilter::JustFiles));

        // --all
        test!(all:        DotFilter <- ["--all"];        Ok(DotFilter::Dotfiles));
        test!(all_all:    DotFilter <- ["--all", "-a"];  Ok(DotFilter::DotfilesAndDots));
        test!(all_all_2:  DotFilter <- ["-aa"];          Ok(DotFilter::DotfilesAndDots));

        test!(all_all_3:  DotFilter <- ["-aaa"];         Ok(DotFilter::DotfilesAndDots));

        // --all and --tree
        test!(tree_a:     DotFilter <- ["-Ta"];          Ok(DotFilter::Dotfiles));
        test!(tree_aa:    DotFilter <- ["-Taa"];         Err(OptionsError::TreeAllAll));
        test!(tree_aaa:   DotFilter <- ["-Taaa"];        Err(OptionsError::TreeAllAll));
    }


    mod ignore_patterns {
        use super::*;
        use std::iter::FromIterator;

        fn pat(string: &'static str) -> glob::Pattern {
            glob::Pattern::new(string).unwrap()
        }

        // Various numbers of globs
        test!(none:   IgnorePatterns <- [];                                        Ok(IgnorePatterns::empty()));
        test!(one:    IgnorePatterns <- ["--ignore-glob", "*.ogg"];                Ok(IgnorePatterns::from_iter(vec![ pat("*.ogg") ])));
        test!(two:    IgnorePatterns <- ["--ignore-glob=*.ogg|*.MP3"];             Ok(IgnorePatterns::from_iter(vec![ pat("*.ogg"), pat("*.MP3") ])));
        test!(loads:  IgnorePatterns <- ["-I*|?|.|*"];                             Ok(IgnorePatterns::from_iter(vec![ pat("*"), pat("?"), pat("."), pat("*") ])));

        // Overriding
        test!(overridden:   IgnorePatterns <- ["-I=*.ogg",    "-I", "*.mp3"];     Ok(IgnorePatterns::from_iter(vec![ pat("*.mp3") ])));
        test!(overridden_2: IgnorePatterns <- ["-I", "*.OGG", "-I*.MP3"];         Ok(IgnorePatterns::from_iter(vec![ pat("*.MP3") ])));
    }


    mod vcs_ignores {
        use super::*;

        test!(off:      VcsIgnore <- [];                VcsIgnore::Off);
        test!(vcs_flag: VcsIgnore <- ["--vcs-ignore"];   VcsIgnore::CheckAndIgnore);
    }
}
