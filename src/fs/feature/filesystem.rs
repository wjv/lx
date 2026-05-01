//! Platform-specific detection of network filesystems.
//!
//! Used by `--filesystem=local` to decide whether to cross a
//! mount boundary: local filesystems are crossed, network ones
//! aren't.  FUSE is treated as network by default — its targets
//! are usually remote (sshfs, gcsfuse, …) and the safer default
//! is "don't traverse".
//!
//! Called only at mount-boundary crossings (one syscall per
//! boundary), not per file.

use std::path::Path;

/// Whether `path` lives on a network filesystem (NFS, CIFS/SMB,
/// AFS, FUSE, …).
///
/// Returns `false` for local filesystems and `false` on syscall
/// failure (fail-open: traversal proceeds).  Returns `false` on
/// platforms without an implementation, which means
/// `--filesystem=local` behaves like `--filesystem=all` outside
/// the supported set (macOS, Linux, the major BSDs).
pub fn is_network_fs(path: &Path) -> bool {
    imp::is_network_fs(path)
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
mod imp {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// macOS and the BSDs all report filesystem type as a string
    /// in `statfs::f_fstypename`.  The list below pools the common
    /// network/remote filesystem names across these platforms.
    /// Anything fuse-derived (`fuse`, `osxfuse`, `macfuse`,
    /// `fusefs`) is treated as network: those mounts are most
    /// often sshfs/cloud-storage frontends.
    ///
    /// Empirically verified on macOS+SMB only.  The other BSDs
    /// share the API surface but the lookup table hasn't been
    /// confirmed against real network mounts; bug reports
    /// against unrecognised filesystems welcome.
    const NETWORK_FS_NAMES: &[&[u8]] = &[
        b"nfs", b"smbfs", b"cifs", b"afpfs", b"webdav", b"ftp",
        b"fuse",    // generic FUSE name on some BSDs
        b"fusefs",  // FreeBSD's name for FUSE
        b"osxfuse", // macOS, older
        b"macfuse", // macOS, newer
    ];

    pub fn is_network_fs(path: &Path) -> bool {
        let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        let mut buf: MaybeUninit<libc::statfs> = MaybeUninit::uninit();

        // SAFETY: `c_path` is a valid NUL-terminated C string;
        // `buf` is sized for `libc::statfs` which `statfs(2)`
        // populates on success.
        let rc = unsafe { libc::statfs(c_path.as_ptr(), buf.as_mut_ptr()) };
        if rc != 0 {
            return false;
        }
        // SAFETY: rc == 0 means statfs(2) populated the struct.
        let buf = unsafe { buf.assume_init() };

        // f_fstypename is a fixed-size [c_char; MFSTYPENAMELEN]
        // holding a NUL-terminated string.
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                buf.f_fstypename.as_ptr().cast::<u8>(),
                buf.f_fstypename.len(),
            )
        };
        let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let fs_name = &bytes[..nul];

        NETWORK_FS_NAMES.contains(&fs_name)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// Linux reports filesystem type as a magic number in
    /// `f_type`.  Constants from `<linux/magic.h>` and the
    /// filesystem-specific headers; values stable across kernels.
    /// FUSE is included for the same reason as on macOS.
    const NFS_SUPER_MAGIC: i64 = 0x6969;
    const CIFS_MAGIC_NUMBER: i64 = 0xff53_4d42;
    const SMB_SUPER_MAGIC: i64 = 0x517B;
    const SMB2_MAGIC_NUMBER: i64 = 0xfe53_4d42;
    const AFS_SUPER_MAGIC: i64 = 0x5346_414f;
    const CODA_SUPER_MAGIC: i64 = 0x7375_7245;
    const NCP_SUPER_MAGIC: i64 = 0x564c;
    const FUSE_SUPER_MAGIC: i64 = 0x6573_5546;
    const V9FS_MAGIC: i64 = 0x0102_1997;

    const NETWORK_FS_MAGICS: &[i64] = &[
        NFS_SUPER_MAGIC,
        CIFS_MAGIC_NUMBER,
        SMB_SUPER_MAGIC,
        SMB2_MAGIC_NUMBER,
        AFS_SUPER_MAGIC,
        CODA_SUPER_MAGIC,
        NCP_SUPER_MAGIC,
        FUSE_SUPER_MAGIC,
        V9FS_MAGIC,
    ];

    pub fn is_network_fs(path: &Path) -> bool {
        let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        let mut buf: MaybeUninit<libc::statfs> = MaybeUninit::uninit();

        // SAFETY: as macOS branch.
        let rc = unsafe { libc::statfs(c_path.as_ptr(), buf.as_mut_ptr()) };
        if rc != 0 {
            return false;
        }
        // SAFETY: rc == 0 means statfs(2) populated the struct.
        let buf = unsafe { buf.assume_init() };

        // f_type's exact integer type varies by libc/architecture
        // (`__fsword_t` on glibc; i64 on 64-bit, i32 on 32-bit).
        // `From` widens i32 to i64 and is identity on i64, so this
        // works portably without a cast (and without tripping the
        // trivial-numeric-cast lint on 64-bit targets).
        let f_type: i64 = buf.f_type.into();
        NETWORK_FS_MAGICS.contains(&f_type)
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "linux",
)))]
mod imp {
    use std::path::Path;

    /// No platform support yet.  `--filesystem=local` falls back
    /// to "no filesystem is network", which means it crosses
    /// every boundary just like `--filesystem=all`.  Documented
    /// in the man page and CHANGELOG.
    pub fn is_network_fs(_path: &Path) -> bool {
        false
    }
}
