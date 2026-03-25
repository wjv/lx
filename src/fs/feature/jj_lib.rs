//! Getting the VCS status of files in a Jujutsu (jj) repository using
//! the `jj-lib` library directly.
//!
//! This is an alternative to the CLI-based `jj` module that avoids
//! subprocess overhead and gives access to file tracking state,
//! ignore rules, and other information not exposed by the jj CLI.
//!
//! Enabled by the `jj-lib` feature flag.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::*;

use crate::fs::fields as f;

use jj_lib::config::StackedConfig;
use jj_lib::repo::Repo;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{self, Workspace};
use jj_lib::matchers::EverythingMatcher;


/// A cache of per-file jj status, built using jj-lib directly.
pub struct JjLibCache {
    /// Map from absolute file path to VCS change status.
    statuses: HashMap<PathBuf, f::VcsStatus>,

    /// Set of absolute paths that are tracked in the working copy tree.
    tracked: std::collections::HashSet<PathBuf>,

    /// Gitignore rules, for `--vcs-ignore` support.
    gitignore: Option<std::sync::Arc<jj_lib::gitignore::GitIgnoreFile>>,

    /// The workspace root, used to resolve relative paths.
    workdir: PathBuf,
}

impl JjLibCache {
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
            p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
        };

        // Set up minimal jj-lib configuration.
        let config = StackedConfig::with_defaults();
        let settings = match UserSettings::from_config(config) {
            Ok(s) => s,
            Err(e) => {
                debug!("jj-lib: failed to create settings: {e}");
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
                Err(_) => {
                    match search_dir.parent() {
                        Some(parent) => search_dir = parent,
                        None => {
                            debug!("jj-lib: no jj workspace found");
                            return None;
                        }
                    }
                }
            }
        };

        let workdir = ws.workspace_root().to_path_buf();
        info!("jj-lib: found workspace at {}", workdir.display());

        // Build a tokio runtime for async jj-lib calls.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");

        // Load the repo (async).
        let repo = match rt.block_on(ws.repo_loader().load_at_head()) {
            Ok(repo) => repo,
            Err(e) => {
                warn!("jj-lib: failed to load repo: {e}");
                return Some(Self { statuses: HashMap::new(), tracked: std::collections::HashSet::new(), gitignore: None, workdir });
            }
        };

        let wc_commit_id = match repo.view().get_wc_commit_id(ws.workspace_name()) {
            Some(id) => id.clone(),
            None => {
                warn!("jj-lib: no working copy commit");
                return Some(Self { statuses: HashMap::new(), tracked: std::collections::HashSet::new(), gitignore: None, workdir });
            }
        };

        let wc_commit: jj_lib::commit::Commit = match repo.store().get_commit(&wc_commit_id) {
            Ok(c) => c,
            Err(e) => {
                warn!("jj-lib: failed to get working copy commit: {e}");
                return Some(Self { statuses: HashMap::new(), tracked: std::collections::HashSet::new(), gitignore: None, workdir });
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
                        warn!("jj-lib: failed to get parent commit: {e}");
                        return Some(Self { statuses: HashMap::new(), tracked: std::collections::HashSet::new(), gitignore: None, workdir });
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
                        let before_absent = diff.before.is_absent();
                        let after_absent = diff.after.is_absent();
                        if before_absent && !after_absent {
                            f::VcsStatus::New
                        } else if !before_absent && after_absent {
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

        // Build gitignore chain for --vcs-ignore support.
        let gitignore = {
            use jj_lib::gitignore::GitIgnoreFile;
            let base = GitIgnoreFile::empty();
            // Chain the root .gitignore if it exists.
            match base.chain_with_file("", workdir.join(".gitignore")) {
                Ok(gi) => Some(gi),
                Err(e) => {
                    debug!("jj-lib: failed to load .gitignore: {e}");
                    None
                }
            }
        };

        debug!("jj-lib cache: {} file statuses, {} tracked files, gitignore: {}",
               statuses.len(), tracked.len(), gitignore.is_some());
        Some(Self { statuses, tracked, gitignore, workdir })
    }
}


impl super::VcsCache for JjLibCache {
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
            self.workdir.join(path)
        };
        let abs = abs.canonicalize().unwrap_or(abs);

        if prefix_lookup {
            // Directory: aggregate child statuses.
            let mut worst_change = f::VcsStatus::NotModified;
            let mut has_untracked = false;
            for (p, &status) in &self.statuses {
                if p.starts_with(&abs) {
                    worst_change = worse_status(worst_change, status);
                }
            }
            // Check if any file under this directory is untracked.
            // (We can't enumerate disk files here, so just use change status.)
            f::VcsFileStatus { staged: worst_change, unstaged: worst_change }
        } else {
            // Single file: change status + tracking/ignore status.

            // Check gitignore first — ignored files get Ignored in
            // the unstaged column, which --vcs-ignore uses to filter.
            if let Some(ref gi) = self.gitignore {
                let rel = abs.strip_prefix(&self.workdir)
                    .unwrap_or(&abs);
                let rel_str = rel.to_string_lossy();
                if gi.matches(&rel_str) {
                    return f::VcsFileStatus {
                        staged: f::VcsStatus::NotModified,
                        unstaged: f::VcsStatus::Ignored,
                    };
                }
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
