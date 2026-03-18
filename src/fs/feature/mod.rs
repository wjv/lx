use std::path::Path;

use crate::fs::fields as f;

pub mod xattr;

/// Backend-agnostic VCS cache.  Implementations discover repositories,
/// cache per-file status, and answer queries for paths within those repos.
pub trait VcsCache: Sync {
    /// Whether this cache has any status information for files under `path`.
    fn has_anything_for(&self, path: &Path) -> bool;

    /// Look up the VCS status for a single path.  `prefix_lookup` is true
    /// when querying a directory (returns the aggregate status of its
    /// contents).
    fn get(&self, path: &Path, prefix_lookup: bool) -> f::VcsFileStatus;
}


#[cfg(feature = "git")]
pub mod git;

#[cfg(not(feature = "git"))]
pub mod git {
    use std::iter::FromIterator;
    use std::path::{Path, PathBuf};

    use crate::fs::fields as f;
    use super::VcsCache;

    pub struct GitCache;

    impl FromIterator<PathBuf> for GitCache {
        fn from_iter<I>(_iter: I) -> Self
        where I: IntoIterator<Item=PathBuf>
        {
            Self
        }
    }

    impl VcsCache for GitCache {
        fn has_anything_for(&self, _path: &Path) -> bool {
            false
        }

        fn get(&self, _path: &Path, _prefix_lookup: bool) -> f::VcsFileStatus {
            f::VcsFileStatus::default()
        }
    }
}


#[cfg(feature = "jj")]
pub mod jj;

#[cfg(not(feature = "jj"))]
pub mod jj {
    use std::path::{Path, PathBuf};

    use crate::fs::fields as f;
    use super::VcsCache;

    pub struct JjCache;

    impl JjCache {
        pub fn discover(_paths: &[PathBuf]) -> Option<Self> {
            None
        }
    }

    impl VcsCache for JjCache {
        fn has_anything_for(&self, _path: &Path) -> bool {
            false
        }

        fn get(&self, _path: &Path, _prefix_lookup: bool) -> f::VcsFileStatus {
            f::VcsFileStatus::default()
        }
    }
}
