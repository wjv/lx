//! Getting the VCS status of files in a Jujutsu (jj) repository.
//!
//! jj has no staging area, so both `staged` and `unstaged` fields of
//! `VcsFileStatus` hold the same value.  We shell out to the `jj` CLI
//! rather than linking against jj-lib (which would pull in tokio, gix,
//! prost, and dozens of other heavy dependencies).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::*;

use crate::fs::fields as f;


/// A cache of per-file jj status, built by running `jj diff --summary`.
pub struct JjCache {
    /// Map from absolute file path to VCS status.
    statuses: HashMap<PathBuf, f::VcsStatus>,

    /// The workspace root, used to resolve relative paths from jj output.
    workdir: PathBuf,
}

impl JjCache {
    /// Discover whether any of the given paths lie inside a jj workspace,
    /// and if so, build a cache of file statuses.  Returns `None` if `jj`
    /// is not installed or the paths are not inside a jj workspace.
    pub fn discover(paths: &[PathBuf]) -> Option<Self> {
        // Use the first path (or cwd) to probe for a jj workspace.
        let probe = if paths.is_empty() {
            PathBuf::from(".")
        } else {
            paths[0].clone()
        };

        let probe_dir = if probe.is_dir() {
            &probe
        } else {
            probe.parent().unwrap_or(Path::new("."))
        };

        // Ask jj for the workspace root.
        let root_output = Command::new("jj")
            .args(["workspace", "root"])
            .current_dir(probe_dir)
            .output();

        let root_output = match root_output {
            Ok(o) if o.status.success() => o,
            Ok(o) => {
                debug!("jj workspace root failed: {}", String::from_utf8_lossy(&o.stderr).trim());
                return None;
            }
            Err(e) => {
                debug!("jj not found or not executable: {e}");
                return None;
            }
        };

        let workdir = PathBuf::from(String::from_utf8_lossy(&root_output.stdout).trim());
        info!("Found jj workspace at {}", workdir.display());

        // Get the diff summary for the working copy.
        let diff_output = Command::new("jj")
            .args(["diff", "--summary", "--ignore-working-copy"])
            .current_dir(&workdir)
            .output();

        let diff_output = match diff_output {
            Ok(o) if o.status.success() => o,
            Ok(o) => {
                warn!("jj diff --summary failed: {}", String::from_utf8_lossy(&o.stderr).trim());
                return Some(Self { statuses: HashMap::new(), workdir });
            }
            Err(e) => {
                warn!("Failed to run jj diff: {e}");
                return Some(Self { statuses: HashMap::new(), workdir });
            }
        };

        let stdout = String::from_utf8_lossy(&diff_output.stdout);
        let mut statuses = HashMap::new();

        for line in stdout.lines() {
            if let Some((status_char, path_str)) = line.split_once(' ') {
                let status = match status_char {
                    "M" => f::VcsStatus::Modified,
                    "A" => f::VcsStatus::New,
                    "D" => f::VcsStatus::Deleted,
                    "C" => f::VcsStatus::Copied,
                    "R" => f::VcsStatus::Renamed,
                    other => {
                        debug!("Unknown jj status char: {other}");
                        f::VcsStatus::Modified
                    }
                };

                let abs_path = workdir.join(path_str);
                statuses.insert(abs_path, status);
            }
        }

        debug!("jj cache: {} file statuses", statuses.len());
        Some(Self { statuses, workdir })
    }
}

impl super::VcsCache for JjCache {
    fn has_anything_for(&self, path: &Path) -> bool {
        // Return true if the path lies inside this jj workspace.
        // Unlike the git backend, we don't check for actual statuses —
        // the column should appear (showing "not modified") even in a
        // clean working copy.
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
            // Directory: aggregate child statuses (worst wins).
            let mut worst = f::VcsStatus::NotModified;
            for (p, &status) in &self.statuses {
                if p.starts_with(&abs) {
                    worst = worse_status(worst, status);
                }
            }
            f::VcsFileStatus { staged: worst, unstaged: worst }
        } else {
            // Single file lookup.
            let status = self.statuses.get(&abs)
                .copied()
                .unwrap_or(f::VcsStatus::NotModified);
            f::VcsFileStatus { staged: status, unstaged: status }
        }
    }

    fn header_name(&self) -> &'static str { "JJ" }
}


/// Return the "worse" of two statuses for directory aggregation.
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
