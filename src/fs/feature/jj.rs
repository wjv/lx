//! Getting the VCS status of files in a Jujutsu (jj) repository using
//! the `jj-lib` library directly.
//!
//! Uses jj-lib for workspace discovery, tree diffing, and file tracking
//! state.  Gitignore rules are handled by `git2`, which correctly
//! resolves all layers: `core.excludesFile`, `.git/info/exclude`, and
//! per-directory `.gitignore` files.
//!
//! Enabled by the `jj` feature flag (which implies `git`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::*;

use crate::fs::fields as f;

use jj_lib::config::StackedConfig;
use jj_lib::repo::Repo;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{self, Workspace};
use jj_lib::matchers::EverythingMatcher;


/// A cache of per-file jj status, built using the jj-lib crate.
pub struct JjCache {
    /// Map from absolute file path to VCS change status.
    statuses: HashMap<PathBuf, f::VcsStatus>,

    /// Set of absolute paths that are tracked in the working copy tree.
    tracked: std::collections::HashSet<PathBuf>,

    /// Git repository for gitignore queries.  jj repos are backed by git,
    /// so we delegate ignore checking to git2 which handles all layers
    /// (global excludes, info/exclude, per-directory .gitignore).
    /// Wrapped in Mutex because git2::Repository is not Sync.
    git_repo: Option<Mutex<git2::Repository>>,

    /// The workspace root, used to resolve relative paths.
    workdir: PathBuf,
}

impl JjCache {
    /// Discover a jj workspace and build a cache of file statuses.
    /// Returns `None` if the paths are not inside a jj workspace.
    pub fn discover(paths: &[PathBuf]) -> Option<Self> {
        let probe = if paths.is_empty() {
            PathBuf::from(".")
        } else {
            paths[0].clone()
        };

        let probe_dir = if probe.is_dir() {
            probe.canonicalize().unwrap_or(probe)
        } else {
            let p = probe.parent().unwrap_or(Path::new("."));
            // An empty parent (from a bare filename like "foo.txt") means cwd.
            let p = if p.as_os_str().is_empty() { Path::new(".") } else { p };
            p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
        };

        // Set up minimal jj configuration.
        let config = StackedConfig::with_defaults();
        let settings = match UserSettings::from_config(config) {
            Ok(s) => s,
            Err(e) => {
                debug!("jj: failed to create settings: {e}");
                return None;
            }
        };

        let store_factories = jj_lib::repo::StoreFactories::default();
        let wc_factories = workspace::default_working_copy_factories();

        // Walk up the directory tree to find the workspace root.
        let mut search_dir = probe_dir.as_path();
        let ws = loop {
            match Workspace::load(&settings, search_dir, &store_factories, &wc_factories) {
                Ok(ws) => break ws,
                Err(e) => {
                    debug!("jj: Workspace::load({}) failed: {e}", search_dir.display());
                    match search_dir.parent() {
                        Some(parent) => search_dir = parent,
                        None => {
                            debug!("jj: no jj workspace found");
                            return None;
                        }
                    }
                }
            }
        };

        let workdir = ws.workspace_root().to_path_buf();
        info!("jj: found workspace at {}", workdir.display());

        // Build a tokio runtime for async jj calls.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");

        // Load the repo (async).
        let repo = match rt.block_on(ws.repo_loader().load_at_head()) {
            Ok(repo) => repo,
            Err(e) => {
                warn!("jj: failed to load repo: {e}");
                return Some(Self::empty(workdir));
            }
        };

        let wc_commit_id = match repo.view().get_wc_commit_id(ws.workspace_name()) {
            Some(id) => id.clone(),
            None => {
                warn!("jj: no working copy commit");
                return Some(Self::empty(workdir));
            }
        };

        let wc_commit: jj_lib::commit::Commit = match repo.store().get_commit(&wc_commit_id) {
            Ok(c) => c,
            Err(e) => {
                warn!("jj: failed to get working copy commit: {e}");
                return Some(Self::empty(workdir));
            }
        };

        // Get parent tree and working copy tree.
        let parent_tree = {
            let parent_ids = wc_commit.parent_ids();
            if parent_ids.is_empty() {
                // Empty parent — use an empty tree.
                repo.store().empty_merged_tree()
            } else {
                let parent: jj_lib::commit::Commit = match repo.store().get_commit(&parent_ids[0]) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("jj: failed to get parent commit: {e}");
                        return Some(Self::empty(workdir));
                    }
                };
                parent.tree()
            }
        };

        let wc_tree = wc_commit.tree();

        // Diff the trees to get per-file status.
        let mut statuses = HashMap::new();
        let matcher = EverythingMatcher;

        use futures::StreamExt;
        let mut stream = parent_tree.diff_stream(&wc_tree, &matcher);

        rt.block_on(async {
            while let Some(entry) = stream.next().await {
                let path_str = entry.path.as_internal_file_string();
                let abs_path = workdir.join(path_str);

                let status = match &entry.values {
                    Ok(diff) => {
                        if !diff.after.is_resolved() {
                            // Unresolved merge conflict in the working copy.
                            f::VcsStatus::Conflicted
                        } else if diff.before.is_absent() && !diff.after.is_absent() {
                            f::VcsStatus::New
                        } else if !diff.before.is_absent() && diff.after.is_absent() {
                            f::VcsStatus::Deleted
                        } else {
                            f::VcsStatus::Modified
                        }
                    }
                    Err(_) => f::VcsStatus::Conflicted,
                };

                statuses.insert(abs_path, status);
            }
        });

        // Collect tracked files from the working copy tree.
        let mut tracked = std::collections::HashSet::new();
        for entry in wc_tree.entries() {
            let (path, _value) = entry;
            let abs_path = workdir.join(path.as_internal_file_string());
            tracked.insert(abs_path);
        }

        // Open the underlying git repo for gitignore queries.
        // git2 handles all ignore layers: core.excludesFile,
        // .git/info/exclude, and per-directory .gitignore files.
        //
        // Try colocated (.git at workspace root) first, then read
        // .jj/repo/store/git_target to find the backing git store
        // (works for both colocated and non-colocated repos).
        let git_repo = Self::open_git_repo(&workdir).map(Mutex::new);

        debug!("jj cache: {} file statuses, {} tracked files",
               statuses.len(), tracked.len());
        Some(Self { statuses, tracked, git_repo, workdir })
    }

    /// Create an empty cache (used for error fallback paths).
    fn empty(workdir: PathBuf) -> Self {
        Self {
            statuses: HashMap::new(),
            tracked: std::collections::HashSet::new(),
            git_repo: None,
            workdir,
        }
    }

    /// Open the git repo backing this jj workspace.  Reads
    /// `.jj/repo/store/git_target` to find the backing store — this
    /// works for colocated repos (points to `../../../.git`),
    /// non-colocated repos (internal bare store), and external repos
    /// (from `jj git init --git-repo <path>`).
    fn open_git_repo(workdir: &Path) -> Option<git2::Repository> {
        let git_target_path = workdir.join(".jj/repo/store/git_target");
        let target = match std::fs::read_to_string(&git_target_path) {
            Ok(t) => t,
            Err(_) => {
                debug!("jj: no git_target found (ignores will not work)");
                return None;
            }
        };

        let git_path = workdir.join(".jj/repo/store").join(target.trim());
        match git2::Repository::open(&git_path) {
            Ok(repo) => {
                debug!("jj: opened backing git store at {}", git_path.display());
                Some(repo)
            }
            Err(e) => {
                debug!("jj: failed to open backing git store: {e}");
                None
            }
        }
    }

    /// Check whether a path (relative to workdir) is ignored by
    /// gitignore rules.  Delegates to git2 which handles all layers.
    fn is_ignored(&self, rel_path: &Path) -> bool {
        match &self.git_repo {
            Some(mutex) => {
                let repo = mutex.lock().unwrap();
                repo.is_path_ignored(rel_path).unwrap_or(false)
            }
            None => false,
        }
    }
}


impl super::VcsCache for JjCache {
    fn has_anything_for(&self, path: &Path) -> bool {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workdir.join(path)
        };
        let abs = abs.canonicalize().unwrap_or(abs);
        abs.starts_with(&self.workdir)
    }

    fn get(&self, path: &Path, prefix_lookup: bool) -> f::VcsFileStatus {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // Relative paths are relative to cwd, not workdir.
            std::env::current_dir()
                .unwrap_or_else(|_| self.workdir.clone())
                .join(path)
        };
        let abs = abs.canonicalize().unwrap_or(abs);

        if prefix_lookup {
            // Directory: aggregate child statuses.
            let mut worst_change = f::VcsStatus::NotModified;
            for (p, &status) in &self.statuses {
                if p.starts_with(&abs) {
                    worst_change = worse_status(worst_change, status);
                }
            }
            f::VcsFileStatus { staged: worst_change, unstaged: worst_change }
        } else {
            // Single file: change status + tracking/ignore status.

            // Check gitignore first — ignored files get Ignored in
            // the unstaged column, which --vcs-ignore uses to filter.
            let rel = abs.strip_prefix(&self.workdir).unwrap_or(&abs);
            if self.is_ignored(rel) {
                return f::VcsFileStatus {
                    staged: f::VcsStatus::NotModified,
                    unstaged: f::VcsStatus::Ignored,
                };
            }

            let change = self.statuses.get(&abs)
                .copied()
                .unwrap_or(f::VcsStatus::NotModified);

            let tracking = if self.tracked.contains(&abs) {
                change
            } else {
                // Untracked file — show U in the second column.
                f::VcsStatus::Untracked
            };

            f::VcsFileStatus { staged: change, unstaged: tracking }
        }
    }

    fn header_name(&self) -> &'static str { "JJ" }
}


fn worse_status(a: f::VcsStatus, b: f::VcsStatus) -> f::VcsStatus {
    fn rank(s: f::VcsStatus) -> u8 {
        match s {
            f::VcsStatus::NotModified => 0,
            f::VcsStatus::Ignored    => 1,
            f::VcsStatus::Untracked  => 2,
            f::VcsStatus::Copied     => 3,
            f::VcsStatus::Renamed    => 4,
            f::VcsStatus::TypeChange => 5,
            f::VcsStatus::Modified   => 6,
            f::VcsStatus::New        => 7,
            f::VcsStatus::Deleted    => 8,
            f::VcsStatus::Conflicted => 9,
        }
    }
    if rank(b) > rank(a) { b } else { a }
}
