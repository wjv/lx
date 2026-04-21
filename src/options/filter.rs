//! Parsing the options for `FileFilter`.

use crate::fs::DotFilter;
use crate::fs::filter::{
    FileFilter, GroupDirs, IgnorePatterns, SortCase, SortField, SymlinkMode, VcsIgnore,
};

use crate::options::parser::MatchedFlags;
use crate::options::{OptionsError, flags};

impl FileFilter {
    /// Determines which of all the file filter options to use.
    pub fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        Ok(Self {
            group_dirs: GroupDirs::deduce(matches),
            reverse: matches.has(flags::REVERSE),
            only_dirs: matches.has(flags::ONLY_DIRS),
            only_files: matches.has(flags::ONLY_FILES),
            sort_field: SortField::deduce(matches),
            sort_by_total_size: matches.has(flags::TOTAL),
            dot_filter: DotFilter::deduce(matches)?,
            ignore_patterns: IgnorePatterns::deduce(matches)?,
            prune_patterns: IgnorePatterns::deduce_from(matches, flags::PRUNE)?,
            vcs_ignore: VcsIgnore::deduce(matches),
            symlink_mode: SymlinkMode::deduce(matches),
        })
    }
}

impl GroupDirs {
    fn deduce(matches: &MatchedFlags) -> Self {
        // Hidden --no-dirs-first / --no-dirs-last suppress any prior
        // selection (e.g. from a personality) — aliases for
        // --group-dirs=none.  Checked first so the early returns
        // below don't skip them.
        if matches.has(flags::NO_DIRS_FIRST) || matches.has(flags::NO_DIRS_LAST) {
            return Self::None;
        }

        // --group-dirs=first|last|none takes priority
        if let Some(word) = matches.get(flags::GROUP_DIRS) {
            return match word {
                "first" => Self::First,
                "last" => Self::Last,
                "none" => Self::None,
                _ => unreachable!("Clap rejects invalid --group-dirs values"),
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
    /// Determines which sort field to use based on the `--sort`
    /// argument.  Delegates the name → variant mapping to the sort
    /// registry (`src/fs/sort_registry.rs`) so this function stays
    /// trivial regardless of how many sort fields lx gains.
    ///
    /// `version` is the one exception: it's not a distinct field
    /// but an explicit promise that the default name sort already
    /// handles version-style names via `natord`.  We resolve it to
    /// `Name(AaBbCc)` here so it doesn't need its own registry
    /// entry.
    fn deduce(matches: &MatchedFlags) -> Self {
        use crate::fs::sort_registry::SortFieldDef;

        let Some(word) = matches.get(flags::SORT) else {
            return Self::default();
        };

        if word == "version" {
            return Self::Name(SortCase::AaBbCc);
        }

        SortFieldDef::field_from_name(word).expect("Clap rejects invalid --sort values")
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
        } else if count == 1 {
            Ok(Self::Dotfiles)
        } else if matches.has(flags::TREE) {
            Err(OptionsError::TreeAllAll)
        } else {
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
        let Some(inputs) = matches.get(flag) else {
            return Ok(Self::empty());
        };

        let (patterns, mut errors) = Self::parse_from_iter(inputs.split('|'));

        match errors.pop() {
            Some(e) => Err(e.into()),
            None => Ok(patterns),
        }
    }
}

impl VcsIgnore {
    pub fn deduce(matches: &MatchedFlags) -> Self {
        if matches.has(flags::VCS_IGNORE) {
            Self::CheckAndIgnore
        } else {
            Self::Off
        }
    }
}

impl SymlinkMode {
    pub fn deduce(matches: &MatchedFlags) -> Self {
        match matches.get(flags::SYMLINKS) {
            Some("hide") => Self::Hide,
            Some("follow") => Self::Follow,
            _ => Self::Show,
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
        test!(empty:         SortField <- [];                  SortField::default());

        // Sort field arguments
        test!(one_arg:       SortField <- ["--sort=mod"];       SortField::ModifiedDate);
        test!(one_long:      SortField <- ["--sort=size"];     SortField::Size);
        test!(one_short:     SortField <- ["-saccessed"];      SortField::AccessedDate);
        test!(lowercase:     SortField <- ["--sort", "name"];  SortField::Name(SortCase::AaBbCc));
        test!(uppercase:     SortField <- ["--sort", "Name"];  SortField::Name(SortCase::ABCabc));
        test!(old:           SortField <- ["--sort", "new"];   SortField::ModifiedDate);
        test!(oldest:        SortField <- ["--sort=newest"];   SortField::ModifiedDate);
        test!(new:           SortField <- ["--sort", "old"];   SortField::ModifiedAge);
        test!(newest:        SortField <- ["--sort=oldest"];   SortField::ModifiedAge);
        test!(age:           SortField <- ["-sage"];           SortField::ModifiedAge);

        test!(mix_hidden_lowercase:     SortField <- ["--sort", ".name"];  SortField::NameMixHidden(SortCase::AaBbCc));
        test!(mix_hidden_uppercase:     SortField <- ["--sort", ".Name"];  SortField::NameMixHidden(SortCase::ABCabc));

        // Errors — Clap rejects invalid values at parse time
        #[test]
        fn error() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--sort=colour"]).is_err());
        }

        // Overriding
        test!(overridden:    SortField <- ["--sort=cr",       "--sort", "mod"];     SortField::ModifiedDate);
        test!(overridden_2:  SortField <- ["--sort", "none",  "--sort=Extension"];  SortField::Extension(SortCase::ABCabc));

        // Batch D: new metadata sort fields
        #[cfg(unix)]
        mod metadata_sorts {
            use super::*;

            test!(permissions:    SortField <- ["--sort=permissions"];  SortField::Permissions);
            test!(perms_mode:     SortField <- ["--sort=mode"];         SortField::Permissions);
            test!(perms_octal:    SortField <- ["--sort=octal"];        SortField::Permissions);
            test!(size_filesize:  SortField <- ["--sort=filesize"];     SortField::Size);

            // `-s perms` is no longer a valid sort value (removed in
            // Batch D).  `perms` survives only as a backward-compat
            // alias for `--columns=perms,...`; the canonical column
            // name is now `permissions`.
            #[test]
            fn perms_rejected_as_sort_field() {
                let cmd = crate::options::parser::build_command();
                assert!(cmd.try_get_matches_from(["lx", "--sort=perms"]).is_err());
            }
            test!(blocks:       SortField <- ["--sort=blocks"];       SortField::Blocks);
            test!(links:        SortField <- ["--sort=links"];        SortField::HardLinks);
            test!(flags:        SortField <- ["--sort=flags"];        SortField::Flags);
            test!(user_lower:   SortField <- ["--sort=user"];         SortField::User(SortCase::AaBbCc));
            test!(user_upper:   SortField <- ["--sort=User"];         SortField::User(SortCase::ABCabc));
            test!(group_lower:  SortField <- ["--sort=group"];        SortField::Group(SortCase::AaBbCc));
            test!(group_upper:  SortField <- ["--sort=Group"];        SortField::Group(SortCase::ABCabc));
            test!(uid:          SortField <- ["--sort=uid"];          SortField::Uid);
            test!(gid:          SortField <- ["--sort=gid"];          SortField::Gid);
            test!(vcs:          SortField <- ["--sort=vcs"];          SortField::VcsStatusSort);

            // `-s version` is an alias for the case-insensitive name
            // sort — natord already handles embedded-number ordering.
            test!(version:      SortField <- ["--sort=version"];      SortField::Name(SortCase::AaBbCc));
        }
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
