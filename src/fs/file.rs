//! Files, and methods and fields to access their metadata.

use std::io;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use log::*;

use crate::fs::dir::Dir;
use crate::fs::fields as f;

/// Cache of total directory sizes keyed by (device, inode).
/// Prevents re-walking the same physical directory multiple times
/// (hardlinks, or the same dir seen via different paths).
#[cfg(unix)]
static DIR_SIZE_CACHE: LazyLock<Mutex<HashMap<(u64, u64), u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// A **File** is a wrapper around one of Rust’s `PathBuf` values, along with
/// associated data about the file.
///
/// Each file is definitely going to have its filename displayed at least
/// once, have its file extension extracted at least once, and have its metadata
/// information queried at least once, so it makes sense to do all this at the
/// start and hold on to all the information.
pub struct File<'dir> {
    /// The filename portion of this file’s path, including the extension.
    ///
    /// This is used to compare against certain filenames (such as checking if
    /// it’s “Makefile” or something) and to highlight only the filename in
    /// colour when displaying the path.
    pub name: String,

    /// The file’s name’s extension, if present, extracted from the name.
    ///
    /// This is queried many times over, so it’s worth caching it.
    pub ext: Option<String>,

    /// The path that begat this file.
    ///
    /// Even though the file’s name is extracted, the path needs to be kept
    /// around, as certain operations involve looking up the file’s absolute
    /// location (such as searching for compiled files) or using its original
    /// path (following a symlink).
    pub path: PathBuf,

    /// The file type, obtained cheaply from `readdir` (via `d_type`) or
    /// from a stat call for top-level arguments.  Always available without
    /// a stat call when the file was discovered via directory iteration.
    file_type: std::fs::FileType,

    /// A lazily-cached `metadata` (`stat`) call for this file.
    ///
    /// This is queried multiple times, and is *not* cached by the OS, as
    /// it could easily change between invocations — but lx is so short-lived
    /// it’s better to just cache it.  Using `OnceLock` avoids the cost of
    /// a stat call when only the file type is needed (e.g. tree view
    /// without `-l`).
    metadata: std::sync::OnceLock<std::fs::Metadata>,

    /// A reference to the directory that contains this file, if any.
    ///
    /// Filenames that get passed in on the command-line directly will have no
    /// parent directory reference — although they technically have one on the
    /// filesystem, we’ll never need to look at it, so it’ll be `None`.
    /// However, *directories* that get passed in will produce files that
    /// contain a reference to it, which is used in certain operations (such
    /// as looking up compiled files).
    pub parent_dir: Option<&'dir Dir>,

    /// Whether this is one of the two `--all all` directories, `.` and `..`.
    ///
    /// Unlike all other entries, these are not returned as part of the
    /// directory’s children, and are in fact added specifically by lx; this
    /// means that they should be skipped when recursing.
    pub is_all_all: bool,

    /// Cached recursive directory size (computed lazily on first call to
    /// `total_size()`).  Avoids recomputing when sorting and rendering
    /// both need the value.
    cached_total_size: std::sync::OnceLock<u64>,
}

impl<'dir> File<'dir> {
    pub fn from_args<PD, FN>(
        path: PathBuf,
        parent_dir: PD,
        filename: FN,
        known_file_type: Option<std::fs::FileType>,
    ) -> io::Result<File<'dir>>
    where
        PD: Into<Option<&'dir Dir>>,
        FN: Into<Option<String>>,
    {
        let parent_dir = parent_dir.into();
        let name = filename.into().unwrap_or_else(|| File::filename(&path));
        let ext = File::ext(&path);
        let is_all_all = false;

        // If the caller provides a file type (from readdir's d_type), use
        // it directly and defer the full stat call.  Otherwise stat now to
        // obtain both file type and metadata eagerly.
        let (file_type, metadata) = if let Some(ft) = known_file_type {
            (ft, std::sync::OnceLock::new())
        } else {
            debug!("Statting file {}", path.display());
            let md = std::fs::symlink_metadata(&path)?;
            let ft = md.file_type();
            (ft, std::sync::OnceLock::from(md))
        };

        Ok(File {
            name,
            ext,
            path,
            file_type,
            metadata,
            parent_dir,
            is_all_all,
            cached_total_size: std::sync::OnceLock::new(),
        })
    }

    pub fn new_aa_current(parent_dir: &'dir Dir) -> io::Result<File<'dir>> {
        let path = parent_dir.path.clone();
        let ext = File::ext(&path);

        debug!("Statting file {}", path.display());
        let md = std::fs::symlink_metadata(&path)?;
        let file_type = md.file_type();
        let is_all_all = true;
        let parent_dir = Some(parent_dir);

        Ok(File {
            path,
            parent_dir,
            file_type,
            metadata: std::sync::OnceLock::from(md),
            ext,
            name: ".".into(),
            is_all_all,
            cached_total_size: std::sync::OnceLock::new(),
        })
    }

    pub fn new_aa_parent(path: PathBuf, parent_dir: &'dir Dir) -> io::Result<File<'dir>> {
        let ext = File::ext(&path);

        debug!("Statting file {}", path.display());
        let md = std::fs::symlink_metadata(&path)?;
        let file_type = md.file_type();
        let is_all_all = true;
        let parent_dir = Some(parent_dir);

        Ok(File {
            path,
            parent_dir,
            file_type,
            metadata: std::sync::OnceLock::from(md),
            ext,
            name: "..".into(),
            is_all_all,
            cached_total_size: std::sync::OnceLock::new(),
        })
    }

    /// A file’s name is derived from its string. This needs to handle directories
    /// such as `/` or `..`, which have no `file_name` component. So instead, just
    /// use the last component as the name.
    pub fn filename(path: &Path) -> String {
        if let Some(back) = path.components().next_back() {
            back.as_os_str().to_string_lossy().to_string()
        } else {
            // use the path as fallback
            error!("Path {} has no last component", path.display());
            path.display().to_string()
        }
    }

    /// Extract an extension from a file path, if one is present, in lowercase.
    ///
    /// The extension is the series of characters after the last dot. This
    /// deliberately counts dotfiles, so the “.git” folder has the extension “git”.
    ///
    /// ASCII lowercasing is used because these extensions are only compared
    /// against a pre-compiled list of extensions which are known to only exist
    /// within ASCII, so it’s alright.
    fn ext(path: &Path) -> Option<String> {
        let name = path.file_name().map(|f| f.to_string_lossy().to_string())?;

        name.rfind('.').map(|p| name[p + 1..].to_ascii_lowercase())
    }

    /// Lazily obtain the file's full metadata, performing a stat call only
    /// on the first access.  Panics if the stat call fails — callers that
    /// construct `File` from a directory iterator provide the file type
    /// cheaply, and metadata is only needed for column rendering where
    /// the file is known to exist.
    pub fn metadata(&self) -> &std::fs::Metadata {
        self.metadata.get_or_init(|| {
            debug!("Lazy-statting file {}", self.path.display());
            std::fs::symlink_metadata(&self.path).expect("metadata for known-existing file")
        })
    }

    /// Whether this file is a directory on the filesystem.
    pub fn is_directory(&self) -> bool {
        self.file_type.is_dir()
    }

    /// Detect whether this directory is a VCS repository root.
    /// Returns the backend type and clean/dirty status.
    pub fn vcs_repo_status(&self) -> f::VcsRepoStatus {
        if !self.is_directory() {
            return f::VcsRepoStatus::None;
        }

        // Check for jj first (preferred when colocated).
        if self.path.join(".jj").is_dir() {
            return f::VcsRepoStatus::Repo {
                backend: "jj",
                clean: true,  // jj has no dirty concept
                branch: None, // TODO: nearest bookmark
            };
        }

        // Check for git.
        if self.path.join(".git").exists() {
            let (clean, branch) = Self::git_repo_info(&self.path);
            return f::VcsRepoStatus::Repo {
                backend: "git",
                clean,
                branch,
            };
        }

        f::VcsRepoStatus::None
    }

    /// Query a git repo for clean/dirty status and current branch.
    fn git_repo_info(path: &std::path::Path) -> (bool, Option<String>) {
        #[cfg(feature = "git")]
        {
            if let Ok(repo) = git2::Repository::open(path) {
                let clean = repo.statuses(None).map(|s| s.is_empty()).unwrap_or(true);
                let branch = repo
                    .head()
                    .ok()
                    .and_then(|h| h.shorthand().map(String::from));
                return (clean, branch);
            }
        }
        let _ = path;
        (true, None)
    }

    /// Whether this file is a directory, or a symlink pointing to a directory.
    pub fn points_to_directory(&self) -> bool {
        if self.is_directory() {
            return true;
        }

        if self.is_link() {
            let target = self.link_target();
            if let FileTarget::Ok(target) = target {
                return target.points_to_directory();
            }
        }

        false
    }

    /// If this file is a directory on the filesystem, then clone its
    /// `PathBuf` for use in one of our own `Dir` values, and read a list of
    /// its contents.
    ///
    /// Returns an IO error upon failure, but this shouldn’t be used to check
    /// if a `File` is a directory or not! For that, just use `is_directory()`.
    pub fn to_dir(&self) -> io::Result<Dir> {
        Dir::read_dir(self.path.clone())
    }

    /// Whether this file is a regular file on the filesystem — that is, not a
    /// directory, a link, or anything else treated specially.
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    /// Whether this file is both a regular file *and* executable for the
    /// current user. An executable file has a different purpose from an
    /// executable directory, so they should be highlighted differently.
    #[cfg(unix)]
    pub fn is_executable_file(&self) -> bool {
        let bit = modes::USER_EXECUTE;
        self.is_file() && (self.metadata().permissions().mode() & bit) == bit
    }

    /// Whether this file is a symlink on the filesystem.
    pub fn is_link(&self) -> bool {
        self.file_type.is_symlink()
    }

    /// Dereference this file if it is a symlink: replace its file type and
    /// metadata with the target's (using `std::fs::metadata` which follows
    /// symlinks).  No-op if the file is not a symlink or the target cannot
    /// be stat'd.
    pub fn deref_link(&mut self) {
        if self.is_link()
            && let Ok(target_meta) = std::fs::metadata(&self.path)
        {
            self.file_type = target_meta.file_type();
            self.metadata = std::sync::OnceLock::from(target_meta);
        }
    }

    /// Whether this file is a named pipe on the filesystem.
    #[cfg(unix)]
    pub fn is_pipe(&self) -> bool {
        self.file_type.is_fifo()
    }

    /// Whether this file is a char device on the filesystem.
    #[cfg(unix)]
    pub fn is_char_device(&self) -> bool {
        self.file_type.is_char_device()
    }

    /// Whether this file is a block device on the filesystem.
    #[cfg(unix)]
    pub fn is_block_device(&self) -> bool {
        self.file_type.is_block_device()
    }

    /// Whether this file is a socket on the filesystem.
    #[cfg(unix)]
    pub fn is_socket(&self) -> bool {
        self.file_type.is_socket()
    }

    /// Re-prefixes the path pointed to by this file, if it’s a symlink, to
    /// make it an absolute path that can be accessed from whichever
    /// directory lx is being run from.
    fn reorient_target_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(dir) = self.parent_dir {
            dir.join(path)
        } else if let Some(parent) = self.path.parent() {
            parent.join(path)
        } else {
            self.path.join(path)
        }
    }

    /// Again assuming this file is a symlink, follows that link and returns
    /// the result of following it.
    ///
    /// For a working symlink that the user is allowed to follow,
    /// this will be the `File` object at the other end, which can then have
    /// its name, colour, and other details read.
    ///
    /// For a broken symlink, returns where the file *would* be, if it
    /// existed. If this file cannot be read at all, returns the error that
    /// we got when we tried to read it.
    pub fn link_target(&self) -> FileTarget<'dir> {
        // We need to be careful to treat the path actually pointed to by
        // this file — which could be absolute or relative — to the path
        // we actually look up and turn into a `File` — which needs to be
        // absolute to be accessible from any directory.
        debug!("Reading link {}", self.path.display());
        let path = match std::fs::read_link(&self.path) {
            Ok(p) => p,
            Err(e) => return FileTarget::Err(e),
        };

        let absolute_path = self.reorient_target_path(&path);

        // Use plain `metadata` instead of `symlink_metadata` - we *want* to
        // follow links.
        match std::fs::metadata(&absolute_path) {
            Ok(metadata) => {
                let ext = File::ext(&path);
                let name = File::filename(&path);
                let file_type = metadata.file_type();
                let file = File {
                    parent_dir: None,
                    path,
                    ext,
                    file_type,
                    metadata: std::sync::OnceLock::from(metadata),
                    name,
                    is_all_all: false,
                    cached_total_size: std::sync::OnceLock::new(),
                };
                FileTarget::Ok(Box::new(file))
            }
            Err(e) => {
                error!("Error following link {}: {e}", path.display());
                FileTarget::Broken(path)
            }
        }
    }

    /// This file’s number of hard links.
    ///
    /// It also reports whether this is both a regular file, and a file with
    /// multiple links. This is important, because a file with multiple links
    /// is uncommon, while you come across directories and other types
    /// with multiple links much more often. Thus, it should get highlighted
    /// more attentively.
    #[cfg(unix)]
    pub fn links(&self) -> f::Links {
        let count = self.metadata().nlink();

        f::Links {
            count,
            multiple: self.is_file() && count > 1,
        }
    }

    /// This file’s inode.
    #[cfg(unix)]
    pub fn inode(&self) -> f::Inode {
        f::Inode(self.metadata().ino())
    }

    /// This file’s number of filesystem blocks.
    ///
    /// (Not the size of each block, which we don’t actually report on)
    #[cfg(unix)]
    pub fn blocks(&self) -> f::Blocks {
        if self.is_file() || self.is_link() {
            f::Blocks::Some(self.metadata().blocks())
        } else {
            f::Blocks::None
        }
    }

    /// BSD/macOS file flags (from `st_flags`).
    #[cfg(target_os = "macos")]
    pub fn flags(&self) -> f::FileFlags {
        use std::os::darwin::fs::MetadataExt;
        f::FileFlags(self.metadata().st_flags())
    }

    /// BSD/FreeBSD file flags (from `st_flags`).
    #[cfg(target_os = "freebsd")]
    pub fn flags(&self) -> f::FileFlags {
        use std::os::freebsd::fs::MetadataExt;
        f::FileFlags(self.metadata().st_flags())
    }

    /// Linux file attributes via `ioctl(FS_IOC_GETFLAGS)`.
    #[cfg(target_os = "linux")]
    pub fn flags(&self) -> f::FileFlags {
        use std::os::unix::io::AsRawFd;

        // FS_IOC_GETFLAGS = _IOR('f', 1, long) = 0x80086601
        const FS_IOC_GETFLAGS: libc::c_ulong = 0x8008_6601;

        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return f::FileFlags(0),
        };

        let mut flags: libc::c_long = 0;
        let ret = unsafe { libc::ioctl(file.as_raw_fd(), FS_IOC_GETFLAGS, &mut flags) };
        if ret < 0 {
            return f::FileFlags(0);
        }
        f::FileFlags(flags as u32)
    }

    /// File flags are not available on this platform.
    #[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "linux")))]
    pub fn flags(&self) -> f::FileFlags {
        f::FileFlags(0)
    }

    /// The ID of the user that own this file.
    #[cfg(unix)]
    pub fn user(&self) -> f::User {
        f::User(self.metadata().uid())
    }

    /// The ID of the group that owns this file.
    #[cfg(unix)]
    pub fn group(&self) -> f::Group {
        f::Group(self.metadata().gid())
    }

    /// This file’s size, if it’s a regular file.
    ///
    /// For directories, no size is given. Although they do have a size on
    /// some filesystems, I’ve never looked at one of those numbers and gained
    /// any information from it. So it’s going to be hidden instead.
    ///
    /// Block and character devices return their device IDs, because they
    /// usually just have a file size of zero.
    #[cfg(unix)]
    pub fn size(&self) -> f::Size {
        if self.is_directory() {
            f::Size::None
        } else if self.is_char_device() || self.is_block_device() {
            let device_ids = self.metadata().rdev().to_be_bytes();

            // In C-land, getting the major and minor device IDs is done with
            // preprocessor macros called `major` and `minor` that depend on
            // the size of `dev_t`, but we just take the second-to-last and
            // last bytes.
            f::Size::DeviceIDs(f::DeviceIDs {
                major: device_ids[6],
                minor: device_ids[7],
            })
        } else {
            f::Size::Some(self.metadata().len())
        }
    }

    #[cfg(windows)]
    pub fn size(&self) -> f::Size {
        if self.is_directory() {
            f::Size::None
        } else {
            f::Size::Some(self.metadata().len())
        }
    }

    /// The total size of this file or directory.  For regular files, this
    /// is the same as `size()`.  For directories, it recursively sums the
    /// sizes of all contents.
    pub fn total_size(&self) -> f::Size {
        if self.is_directory() {
            let size = *self.cached_total_size.get_or_init(|| {
                #[cfg(unix)]
                let key = Some((self.metadata().dev(), self.metadata().ino()));
                #[cfg(not(unix))]
                let key = None;
                Self::dir_total_size(&self.path, key)
            });
            f::Size::Some(size)
        } else {
            self.size()
        }
    }

    /// Recursively sum file sizes in a directory.
    ///
    /// Uses rayon to parallelise subdirectory walks for performance
    /// on multi-core systems.  The syscall overhead (many `stat()`
    /// calls) dominates, so parallelism lets the kernel pipeline I/O.
    ///
    /// Results are cached by `(dev, ino)` to avoid re-walking the same
    /// physical directory when encountered through hardlinks or
    /// multiple paths.
    /// The `cache_key` is the `(dev, ino)` of the directory, passed
    /// from the caller to avoid an extra `symlink_metadata()` call.
    /// When `None` (non-Unix or unavailable), the cache is bypassed.
    fn dir_total_size(path: &std::path::Path, cache_key: Option<(u64, u64)>) -> u64 {
        use rayon::prelude::*;

        // Check the (dev, ino) cache before walking.
        #[cfg(unix)]
        if let Some(key) = cache_key
            && let Some(&cached) = DIR_SIZE_CACHE.lock().unwrap().get(&key)
        {
            return cached;
        }

        let entries: Vec<_> = match std::fs::read_dir(path) {
            Ok(e) => e.filter_map(std::result::Result::ok).collect(),
            Err(_) => return 0,
        };

        let size: u64 = entries
            .par_iter()
            .map(|entry| {
                let Ok(ft) = entry.file_type() else { return 0 };

                if ft.is_dir() {
                    // Extract (dev, ino) from the entry's metadata for
                    // the recursive call — avoids an extra stat.
                    #[cfg(unix)]
                    let child_key = entry.metadata().ok().map(|m| (m.dev(), m.ino()));
                    #[cfg(not(unix))]
                    let child_key = None;
                    Self::dir_total_size(&entry.path(), child_key)
                } else if ft.is_file() {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                }
            })
            .sum();

        // Store in cache for future lookups.
        #[cfg(unix)]
        if let Some(key) = cache_key {
            DIR_SIZE_CACHE.lock().unwrap().insert(key, size);
        }

        size
    }

    /// This file’s last modified timestamp, if available on this platform.
    pub fn modified_time(&self) -> Option<SystemTime> {
        self.metadata().modified().ok()
    }

    /// This file’s last changed timestamp, if available on this platform.
    ///
    /// The Unix impl is infallible (ctime is always present in stat),
    /// but we return `Option` to match the other three timestamp
    /// accessors and the Windows impl, which delegates to
    /// `modified_time()` and genuinely can fail.
    #[cfg(unix)]
    #[allow(clippy::unnecessary_wraps)]
    pub fn changed_time(&self) -> Option<SystemTime> {
        let (mut sec, mut nanosec) = (self.metadata().ctime(), self.metadata().ctime_nsec());

        if sec < 0 {
            if nanosec > 0 {
                sec += 1;
                nanosec -= 1_000_000_000;
            }

            let duration = Duration::new(sec.unsigned_abs(), nanosec.unsigned_abs() as u32);
            Some(UNIX_EPOCH - duration)
        } else {
            let duration = Duration::new(sec as u64, nanosec as u32);
            Some(UNIX_EPOCH + duration)
        }
    }

    #[cfg(windows)]
    pub fn changed_time(&self) -> Option<SystemTime> {
        return self.modified_time();
    }

    /// This file’s last accessed timestamp, if available on this platform.
    pub fn accessed_time(&self) -> Option<SystemTime> {
        self.metadata().accessed().ok()
    }

    /// This file’s created timestamp, if available on this platform.
    pub fn created_time(&self) -> Option<SystemTime> {
        self.metadata().created().ok()
    }

    /// This file’s ‘type’.
    ///
    /// This is used a the leftmost character of the permissions column.
    /// The file type can usually be guessed from the colour of the file, but
    /// ls puts this character there.
    #[cfg(unix)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else if self.is_pipe() {
            f::Type::Pipe
        } else if self.is_link() {
            f::Type::Link
        } else if self.is_char_device() {
            f::Type::CharDevice
        } else if self.is_block_device() {
            f::Type::BlockDevice
        } else if self.is_socket() {
            f::Type::Socket
        } else {
            f::Type::Special
        }
    }

    #[cfg(windows)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else {
            f::Type::Special
        }
    }

    /// The raw permission bits from the file's mode, masked to the
    /// permission-significant bits (rwx × owner/group/other plus
    /// setuid/setgid/sticky).  Used for sorting by permissions — both
    /// the symbolic and octal views sort numerically on this value.
    #[cfg(unix)]
    pub fn permissions_octal(&self) -> u32 {
        self.metadata().mode() & 0o7777
    }

    /// This file’s permissions, with flags for each bit.
    #[cfg(unix)]
    pub fn permissions(&self) -> f::Permissions {
        let bits = self.metadata().mode();
        let has_bit = |bit| bits & bit == bit;

        f::Permissions {
            user_read: has_bit(modes::USER_READ),
            user_write: has_bit(modes::USER_WRITE),
            user_execute: has_bit(modes::USER_EXECUTE),

            group_read: has_bit(modes::GROUP_READ),
            group_write: has_bit(modes::GROUP_WRITE),
            group_execute: has_bit(modes::GROUP_EXECUTE),

            other_read: has_bit(modes::OTHER_READ),
            other_write: has_bit(modes::OTHER_WRITE),
            other_execute: has_bit(modes::OTHER_EXECUTE),

            sticky: has_bit(modes::STICKY),
            setgid: has_bit(modes::SETGID),
            setuid: has_bit(modes::SETUID),
        }
    }

    #[cfg(windows)]
    pub fn attributes(&self) -> f::Attributes {
        let bits = self.metadata().file_attributes();
        let has_bit = |bit| bits & bit == bit;

        // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
        f::Attributes {
            directory: has_bit(0x10),
            archive: has_bit(0x20),
            readonly: has_bit(0x1),
            hidden: has_bit(0x2),
            system: has_bit(0x4),
            reparse_point: has_bit(0x400),
        }
    }

    /// Whether this file’s extension is any of the strings that get passed in.
    ///
    /// This will always return `false` if the file has no extension.
    pub fn extension_is_one_of(&self, choices: &[&str]) -> bool {
        match &self.ext {
            Some(ext) => choices.contains(&&ext[..]),
            None => false,
        }
    }

    /// Whether this file’s name, including extension, is any of the strings
    /// that get passed in.
    pub fn name_is_one_of(&self, choices: &[&str]) -> bool {
        choices.contains(&&self.name[..])
    }
}

impl<'a> AsRef<File<'a>> for File<'a> {
    fn as_ref(&self) -> &File<'a> {
        self
    }
}

/// The result of following a symlink.
pub enum FileTarget<'dir> {
    /// The symlink pointed at a file that exists.
    Ok(Box<File<'dir>>),

    /// The symlink pointed at a file that does not exist. Holds the path
    /// where the file would be, if it existed.
    Broken(PathBuf),

    /// There was an IO error when following the link. This can happen if the
    /// file isn’t a link to begin with, but also if, say, we don’t have
    /// permission to follow it.
    Err(io::Error),
    // Err is its own variant, instead of having the whole thing be inside an
    // `io::Result`, because being unable to follow a symlink is not a serious
    // error — we just display the error message and move on.
}

impl FileTarget<'_> {
    /// Whether this link doesn’t lead to a file, for whatever reason. This
    /// gets used to determine how to highlight the link in grid views.
    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Broken(_) | Self::Err(_))
    }
}

/// More readable aliases for the permission bits exposed by libc.
#[allow(trivial_numeric_casts)]
#[cfg(unix)]
mod modes {

    // The `libc::mode_t` type’s actual type varies, but the value returned
    // from `metadata.permissions().mode()` is always `u32`.
    pub type Mode = u32;

    pub const USER_READ: Mode = libc::S_IRUSR as Mode;
    pub const USER_WRITE: Mode = libc::S_IWUSR as Mode;
    pub const USER_EXECUTE: Mode = libc::S_IXUSR as Mode;

    pub const GROUP_READ: Mode = libc::S_IRGRP as Mode;
    pub const GROUP_WRITE: Mode = libc::S_IWGRP as Mode;
    pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;

    pub const OTHER_READ: Mode = libc::S_IROTH as Mode;
    pub const OTHER_WRITE: Mode = libc::S_IWOTH as Mode;
    pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;

    pub const STICKY: Mode = libc::S_ISVTX as Mode;
    pub const SETGID: Mode = libc::S_ISGID as Mode;
    pub const SETUID: Mode = libc::S_ISUID as Mode;
}

#[cfg(test)]
mod ext_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn extension() {
        assert_eq!(Some("dat".to_string()), File::ext(Path::new("fester.dat")))
    }

    #[test]
    fn dotfile() {
        assert_eq!(Some("vimrc".to_string()), File::ext(Path::new(".vimrc")))
    }

    #[test]
    fn no_extension() {
        assert_eq!(None, File::ext(Path::new("jarlsberg")))
    }
}

#[cfg(test)]
mod filename_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn file() {
        assert_eq!("fester.dat", File::filename(Path::new("fester.dat")))
    }

    #[test]
    fn no_path() {
        assert_eq!("foo.wha", File::filename(Path::new("/var/cache/foo.wha")))
    }

    #[test]
    fn here() {
        assert_eq!(".", File::filename(Path::new(".")))
    }

    #[test]
    fn there() {
        assert_eq!("..", File::filename(Path::new("..")))
    }

    #[test]
    fn everywhere() {
        assert_eq!("..", File::filename(Path::new("./..")))
    }

    #[test]
    #[cfg(unix)]
    fn topmost() {
        assert_eq!("/", File::filename(Path::new("/")))
    }
}
