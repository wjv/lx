//! Filtering and sorting the list of files before displaying them.

use std::cmp::Ordering;
use std::iter::FromIterator;

use crate::fs::DotFilter;
use crate::fs::File;

/// The **file filter** processes a list of files before displaying them to
/// the user, by removing files they don’t want to see, and putting the list
/// in the desired order.
///
/// Usually a user does not want to see *every* file in the list. The most
/// common case is to remove files starting with `.`, which are designated
/// as ‘hidden’ files.
///
/// The special files `.` and `..` files are not actually filtered out, but
/// need to be inserted into the list, in a special case.
///
/// The filter also governs sorting the list. After being filtered, pairs of
/// files are compared and sorted based on the result, with the sort field
/// performing the comparison.
#[derive(PartialEq, Eq, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FileFilter {
    /// Whether to group directories before or after other files.
    pub group_dirs: GroupDirs,

    /// The metadata field to sort by.
    pub sort_field: SortField,

    /// When true, sort by total (recursive) size instead of file size.
    pub sort_by_total_size: bool,

    /// Whether to reverse the sorting order. This would sort the largest
    /// files first, or files starting with Z, or the most-recently-changed
    /// ones, depending on the sort field.
    pub reverse: bool,

    /// Whether to only show directories.
    pub only_dirs: bool,

    /// Whether to only show regular files (non-directories).
    pub only_files: bool,

    /// Which invisible “dot” files to include when listing a directory.
    ///
    /// Files starting with a single “.” are used to determine “system” or
    /// “configuration” files that should not be displayed in a regular
    /// directory listing, and the directory entries “.” and “..” are
    /// considered extra-special.
    ///
    /// This came about more or less by a complete historical accident,
    /// when the original `ls` tried to hide `.` and `..`:
    ///
    /// [Linux History: How Dot Files Became Hidden Files](https://linux-audit.com/linux-history-how-dot-files-became-hidden-files/)
    pub dot_filter: DotFilter,

    /// Glob patterns to ignore. Any file name that matches *any* of these
    /// patterns won’t be displayed in the list.
    pub ignore_patterns: IgnorePatterns,

    /// Glob patterns to prune. Matching directories are shown (with
    /// metadata and total-size) but not recursed into.
    pub prune_patterns: IgnorePatterns,

    /// Whether to ignore VCS-ignored patterns.
    pub vcs_ignore: VcsIgnore,

    /// How to handle symlinks.
    pub symlink_mode: SymlinkMode,
}

/// How symlinks should be handled.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default)]
pub enum SymlinkMode {
    /// Show symlinks as-is (default).
    #[default]
    Show,
    /// Hide symlinks from listings.
    Hide,
    /// Follow (dereference) symlinks: show target metadata, recurse
    /// into symlinked directories.
    Follow,
}

impl FileFilter {
    /// Whether a directory should be pruned (shown but not recursed into).
    pub fn is_pruned(&self, file: &File<'_>) -> bool {
        file.is_directory() && self.prune_patterns.is_matched(&file.name)
    }

    /// Remove every file in the given vector that does *not* pass the
    /// filter predicate for files found inside a directory.
    pub fn filter_child_files(&self, files: &mut Vec<File<'_>>) {
        files.retain(|f| !self.ignore_patterns.is_matched(&f.name));

        match self.symlink_mode {
            SymlinkMode::Hide => {
                files.retain(|f| !f.is_link());
            }
            SymlinkMode::Follow => {
                for f in files.iter_mut() {
                    f.deref_link();
                }
            }
            SymlinkMode::Show => {}
        }

        if self.only_dirs {
            files.retain(File::is_directory);
        }

        if self.only_files {
            files.retain(|f| !f.is_directory());
        }
    }

    /// Remove every file in the given vector that does *not* pass the
    /// filter predicate for file names specified on the command-line.
    ///
    /// The rules are different for these types of files than the other
    /// type because the ignore rules can be used with globbing. For
    /// example, running `lx -I=’*.tmp’ .vimrc` shouldn’t filter out the
    /// dotfile, because it’s been directly specified. But running
    /// `lx -I=’*.ogg’ music/*` should filter out the ogg files obtained
    /// from the glob, even though the globbing is done by the shell!
    pub fn filter_argument_files(&self, files: &mut Vec<File<'_>>) {
        files.retain(|f| !self.ignore_patterns.is_matched(&f.name));

        if self.symlink_mode == SymlinkMode::Follow {
            for f in files.iter_mut() {
                f.deref_link();
            }
        }

        // `--only-dirs` / `--only-files` are explicit type filters
        // the user just asked for; they apply to CLI-named files
        // too, not just to children discovered by recursing.  This
        // is a deliberate departure from `ignore_patterns`, which
        // keeps explicitly-named files (so `lx -I '*.tmp' .vimrc`
        // still shows `.vimrc`).
        if self.only_dirs {
            files.retain(File::is_directory);
        }
        if self.only_files {
            files.retain(|f| !f.is_directory());
        }
    }

    /// Sort the files in the given vector based on the sort field option.
    ///
    /// The optional `vcs` parameter supports `-s vcs`: sort fields
    /// marked `needs_vcs = true` in the registry require the VCS
    /// cache to look up each file's status.  Callsites that don't
    /// have a cache (grid and lines views) pass `None`; those sort
    /// fields then fall back to the registry entry's normal
    /// comparator (typically by-name for VCS sort).
    pub fn sort_files<'a, F>(&self, files: &mut [F], vcs: Option<&dyn crate::fs::feature::VcsCache>)
    where
        F: AsRef<File<'a>>,
    {
        use crate::fs::sort_registry::SortFieldDef;

        let def = SortFieldDef::for_field(self.sort_field);

        if self.sort_by_total_size && self.sort_field == SortField::Size {
            // When --total-size is active, sort by recursive dir size.
            use crate::fs::fields::Size;
            files.sort_by(|a, b| {
                let sa = match a.as_ref().total_size() {
                    Size::Some(s) => s,
                    _ => 0,
                };
                let sb = match b.as_ref().total_size() {
                    Size::Some(s) => s,
                    _ => 0,
                };
                sa.cmp(&sb)
            });
        } else if def.needs_vcs && vcs.is_some() {
            let cache = vcs.unwrap();
            files.sort_by(|a, b| {
                let sa = cache.get(&a.as_ref().path, a.as_ref().is_directory());
                let sb = cache.get(&b.as_ref().path, b.as_ref().is_directory());
                // Use the unstaged status as the primary key (staged
                // and unstaged are identical in jj; for git the
                // unstaged state is usually the more interesting one
                // for "what needs attention").
                Self::vcs_sort_key(sa.unstaged)
                    .cmp(&Self::vcs_sort_key(sb.unstaged))
                    .then_with(|| natord::compare(&a.as_ref().name, &b.as_ref().name))
            });
        } else {
            files.sort_by(|a, b| (def.compare)(a.as_ref(), b.as_ref()));
        }

        if self.reverse {
            files.reverse();
        }

        match self.group_dirs {
            GroupDirs::First => {
                // This relies on the fact that `sort_by` is *stable*: it will
                // keep adjacent elements next to each other.
                files.sort_by(|a, b| {
                    b.as_ref()
                        .points_to_directory()
                        .cmp(&a.as_ref().points_to_directory())
                });
            }
            GroupDirs::Last => {
                files.sort_by(|a, b| {
                    a.as_ref()
                        .points_to_directory()
                        .cmp(&b.as_ref().points_to_directory())
                });
            }
            GroupDirs::None => {}
        }
    }

    /// Sort ordering for VCS status under `-s vcs`.  Attention-worthy
    /// states come first (conflicted, modified, etc.) so they appear
    /// at the top of the listing; unmodified files sort last.
    ///
    /// Explicit integers rather than declaration-order `Ord` because
    /// the enum's declaration order is documentation-first, not
    /// sort-order-first — changing the enum for readability
    /// shouldn't silently change sort behaviour.
    fn vcs_sort_key(status: crate::fs::fields::VcsStatus) -> u8 {
        use crate::fs::fields::VcsStatus::*;
        match status {
            Conflicted => 0,
            Modified => 1,
            New => 2,
            Deleted => 3,
            Renamed => 4,
            Copied => 5,
            TypeChange => 6,
            Untracked => 7,
            Ignored => 8,
            NotModified => 9,
        }
    }
}

/// User-supplied field to sort by.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SortField {
    /// Don’t apply any sorting. This is usually used as an optimisation in
    /// scripts, where the order doesn’t matter.
    Unsorted,

    /// The file name. This is the default sorting.
    Name(SortCase),

    /// The file’s extension, with extensionless files being listed first.
    Extension(SortCase),

    /// The file’s size, in bytes.
    Size,

    /// The file’s inode, which usually corresponds to the order in which
    /// files were created on the filesystem, more or less.
    #[cfg(unix)]
    FileInode,

    /// The time the file was modified (the “mtime”).
    ///
    /// As this is stored as a Unix timestamp, rather than a local time
    /// instance, the time zone does not matter and will only be used to
    /// display the timestamps, not compare them.
    ModifiedDate,

    /// The time the file was accessed (the “atime”).
    ///
    /// Oddly enough, this field rarely holds the *actual* accessed time.
    /// Recording a read time means writing to the file each time it’s read
    /// slows the whole operation down, so many systems will only update the
    /// timestamp in certain circumstances. This has become common enough that
    /// it’s now expected behaviour!
    /// <https://unix.stackexchange.com/a/8842>
    AccessedDate,

    /// The time the file was changed (the “ctime”).
    ///
    /// This field is used to mark the time when a file’s metadata
    /// changed — its permissions, owners, or link count.
    ///
    /// In original Unix, this was, however, meant as creation time.
    /// <https://www.bell-labs.com/usr/dmr/www/cacm.html>
    ChangedDate,

    /// The time the file was created (the “btime” or “birthtime”).
    CreatedDate,

    /// The type of the file: directories, links, pipes, regular, files, etc.
    ///
    /// Files are ordered according to the `PartialOrd` implementation of
    /// `fs::fields::Type`, so changing that will change this.
    FileType,

    /// The “age” of the file, which is the time it was modified sorted
    /// backwards. The reverse of the `ModifiedDate` ordering!
    ///
    /// It turns out that listing the most-recently-modified files first is a
    /// common-enough use case that it deserves its own variant. This would be
    /// implemented by just using the modified date and setting the reverse
    /// flag, but this would make reversing *that* output not work, which is
    /// bad, even though that’s kind of nonsensical. So it’s its own variant
    /// that can be reversed like usual.
    ModifiedAge,

    /// The file's name, however if the name of the file begins with `.`
    /// ignore the leading `.` and then sort as Name
    NameMixHidden(SortCase),

    /// The permission bits, in octal order.  Symbolic and octal views
    /// both compare numerically against the underlying mode bits.
    #[cfg(unix)]
    Permissions,

    /// The number of allocated filesystem blocks.
    #[cfg(unix)]
    Blocks,

    /// The hard link count.
    #[cfg(unix)]
    HardLinks,

    /// Platform file flags (BSD/macOS `chflags`, Linux attributes).
    /// Compared numerically on the raw flag bits.
    Flags,

    /// The owner's name, looked up from the UID.  Files whose owner
    /// has no name entry fall back to numeric UID comparison.
    #[cfg(unix)]
    User(SortCase),

    /// The owner's group name, looked up from the GID.  Files whose
    /// group has no name entry fall back to numeric GID.
    #[cfg(unix)]
    Group(SortCase),

    /// The numeric user ID.
    #[cfg(unix)]
    Uid,

    /// The numeric group ID.
    #[cfg(unix)]
    Gid,

    /// The file's VCS status.  Files are grouped by their status with
    /// attention-worthy states (conflicted, modified, etc.) first and
    /// unmodified files last, so that `-s vcs` surfaces what needs
    /// work.  Secondary sort within a status group is by name.
    VcsStatusSort,
}

/// Whether a field should be sorted case-sensitively or case-insensitively.
/// This determines which of the `natord_plus_plus` functions to use.
///
/// I kept on forgetting which one was sensitive and which one was
/// insensitive. Would a case-sensitive sort put capital letters first because
/// it takes the case of the letters into account, or intermingle them with
/// lowercase letters because it takes the difference between the two cases
/// into account? I gave up and just named these two variants after the
/// effects they have.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SortCase {
    /// Sort files case-sensitively with uppercase first, with ‘A’ coming
    /// before ‘a’.
    ABCabc,

    /// Sort files case-insensitively, with ‘A’ being equal to ‘a’.
    AaBbCc,
}

impl SortField {
    /// Compares two files to determine the order they should be listed in,
    /// depending on the search field.
    ///
    /// The `natord_plus_plus` crate is used here to provide a more *natural* sorting
    /// order than just sorting character-by-character. This splits filenames
    /// into groups between letters and numbers, and then sorts those blocks
    /// together, so `file10` will sort after `file9`, instead of before it
    /// because of the `1`.
    /// Compares two files using this sort field's comparator.
    ///
    /// The actual comparison logic lives in
    /// `crate::fs::sort_registry::SORT_REGISTRY`; this method is a
    /// thin dispatch that looks up the current field's entry and
    /// calls its comparator function.
    ///
    /// `VcsStatusSort` has a fallback (by-name) comparator here;
    /// the VCS-aware comparator lives in `sort_files` because it
    /// needs the `VcsCache`.
    pub fn compare_files(self, a: &File<'_>, b: &File<'_>) -> Ordering {
        (crate::fs::sort_registry::SortFieldDef::for_field(self).compare)(a, b)
    }
}

/// The **ignore patterns** are a list of globs that are tested against
/// each filename, and if any of them match, that file isn’t displayed.
/// This lets a user hide, say, text files by ignoring `*.txt`.
#[derive(PartialEq, Eq, Default, Debug, Clone)]
pub struct IgnorePatterns {
    patterns: Vec<glob::Pattern>,
}

impl FromIterator<glob::Pattern> for IgnorePatterns {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = glob::Pattern>,
    {
        let patterns = iter.into_iter().collect();
        Self { patterns }
    }
}

impl IgnorePatterns {
    /// Create a new list from the input glob strings, turning the inputs that
    /// are valid glob patterns into an `IgnorePatterns`. The inputs that
    /// don’t parse correctly are returned separately.
    pub fn parse_from_iter<'a, I: IntoIterator<Item = &'a str>>(
        iter: I,
    ) -> (Self, Vec<glob::PatternError>) {
        let iter = iter.into_iter();

        // Almost all glob patterns are valid, so it’s worth pre-allocating
        // the vector with enough space for all of them.
        let mut patterns = match iter.size_hint() {
            (_, Some(count)) => Vec::with_capacity(count),
            _ => Vec::new(),
        };

        // Similarly, assume there won’t be any errors.
        let mut errors = Vec::new();

        for input in iter {
            match glob::Pattern::new(input) {
                Ok(pat) => patterns.push(pat),
                Err(e) => errors.push(e),
            }
        }

        (Self { patterns }, errors)
    }

    /// Create a new empty set of patterns that matches nothing.
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Test whether the given filename matches any of the patterns.
    pub fn is_matched(&self, file: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(file))
    }
}

/// How to group directories relative to other files in the listing.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default)]
pub enum GroupDirs {
    /// List directories before other files.
    First,
    /// List directories after other files.
    Last,
    /// No special grouping; directories sort with everything else.
    #[default]
    None,
}

/// Whether to ignore or display files that the VCS would ignore.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum VcsIgnore {
    /// Ignore files that the VCS would ignore.
    CheckAndIgnore,

    /// Display files, even if the VCS would ignore them.
    Off,
}

#[cfg(test)]
mod test_ignores {
    use super::*;

    #[test]
    fn empty_matches_nothing() {
        let pats = IgnorePatterns::empty();
        assert!(!pats.is_matched("nothing"));
        assert!(!pats.is_matched("test.mp3"));
    }

    #[test]
    fn ignores_a_glob() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["*.mp3"]);
        assert!(fails.is_empty());
        assert!(!pats.is_matched("nothing"));
        assert!(pats.is_matched("test.mp3"));
    }

    #[test]
    fn ignores_an_exact_filename() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["nothing"]);
        assert!(fails.is_empty());
        assert!(pats.is_matched("nothing"));
        assert!(!pats.is_matched("test.mp3"));
    }

    #[test]
    fn ignores_both() {
        let (pats, fails) = IgnorePatterns::parse_from_iter(vec!["nothing", "*.mp3"]);
        assert!(fails.is_empty());
        assert!(pats.is_matched("nothing"));
        assert!(pats.is_matched("test.mp3"));
    }
}
