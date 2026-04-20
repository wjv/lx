//! Data-driven sort-field registry.
//!
//! Every `--sort` field is described by a [`SortFieldDef`] entry in
//! the [`SORT_REGISTRY`] slice.  The parser, the `--sort` deduce
//! logic, and the sort comparator all read from this single table,
//! so adding a new sort field is one entry here plus (optionally)
//! one comparator function.
//!
//! This mirrors the `ColumnDef` registry in `src/output/column_registry.rs`
//! for columns.

use std::cmp::Ordering;

use crate::fs::File;
use crate::fs::fields::Blocks;
use crate::fs::filter::SortField;


// ── SortFieldDef ────────────────────────────────────────────────

/// Metadata for a single `--sort` field.
pub struct SortFieldDef {
    /// The `SortField` enum variant this definition describes.
    pub field: SortField,

    /// Canonical `--sort=NAME` value.
    pub name: &'static str,

    /// Additional accepted names.  Always hidden from `--help`.
    pub aliases: &'static [&'static str],

    /// One-line human description of the sort field.  Populated for
    /// every entry but not yet read by any code path; the 0.10 audit
    /// will decide whether to surface these in `--help` (e.g. via a
    /// `--help=sort` section) or delete the field altogether.  Until
    /// then, treat any change here as user-visible documentation —
    /// keep the wording crisp.
    #[allow(dead_code)]
    pub description: &'static str,

    /// The comparator.  Receives two files and returns their order.
    /// For `VcsStatusSort` this is a fallback that sorts by name;
    /// the VCS-aware comparator lives in `FileFilter::sort_files`
    /// and is gated on `needs_vcs`.
    pub compare: fn(&File<'_>, &File<'_>) -> Ordering,

    /// If true, this sort field needs a `VcsCache` to work
    /// correctly.  `sort_files` checks this flag and routes to a
    /// special VCS-aware path when a cache is available; without
    /// a cache, the `compare` function above is used as a fallback.
    pub needs_vcs: bool,

    /// If true, this entry's canonical name is hidden from `--help`
    /// along with its aliases.  Used for specialised variants like
    /// `.name` and `.Name` (hidden-files-mixed-in) which are valid
    /// sort values but too niche for the main help list.
    pub hidden: bool,
}


// ── Comparator functions ────────────────────────────────────────
//
// Each `--sort` value has a dedicated top-level function here.
// Keeping them as plain `fn` pointers (rather than closures) lets
// them be stored in a `const`/`static` registry.

fn cmp_unsorted(_a: &File<'_>, _b: &File<'_>) -> Ordering {
    Ordering::Equal
}

fn cmp_name_ci(a: &File<'_>, b: &File<'_>) -> Ordering {
    natord::compare_ignore_case(&a.name, &b.name)
}

fn cmp_name_cs(a: &File<'_>, b: &File<'_>) -> Ordering {
    natord::compare(&a.name, &b.name)
}

fn cmp_name_mix_hidden_ci(a: &File<'_>, b: &File<'_>) -> Ordering {
    natord::compare_ignore_case(strip_dot(&a.name), strip_dot(&b.name))
}

fn cmp_name_mix_hidden_cs(a: &File<'_>, b: &File<'_>) -> Ordering {
    natord::compare(strip_dot(&a.name), strip_dot(&b.name))
}

fn strip_dot(n: &str) -> &str {
    n.strip_prefix('.').unwrap_or(n)
}

fn cmp_extension_ci(a: &File<'_>, b: &File<'_>) -> Ordering {
    match a.ext.cmp(&b.ext) {
        Ordering::Equal => natord::compare_ignore_case(&a.name, &b.name),
        order           => order,
    }
}

fn cmp_extension_cs(a: &File<'_>, b: &File<'_>) -> Ordering {
    match a.ext.cmp(&b.ext) {
        Ordering::Equal => natord::compare(&a.name, &b.name),
        order           => order,
    }
}

fn cmp_size(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.metadata().len().cmp(&b.metadata().len())
}

fn cmp_modified(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.modified_time().cmp(&b.modified_time())
}

fn cmp_modified_age(a: &File<'_>, b: &File<'_>) -> Ordering {
    // Reverse of modified: newest first.
    b.modified_time().cmp(&a.modified_time())
}

fn cmp_changed(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.changed_time().cmp(&b.changed_time())
}

fn cmp_accessed(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.accessed_time().cmp(&b.accessed_time())
}

fn cmp_created(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.created_time().cmp(&b.created_time())
}

fn cmp_type(a: &File<'_>, b: &File<'_>) -> Ordering {
    match a.type_char().cmp(&b.type_char()) {
        Ordering::Equal => natord::compare(&a.name, &b.name),
        order           => order,
    }
}

#[cfg(unix)]
fn cmp_inode(a: &File<'_>, b: &File<'_>) -> Ordering {
    use std::os::unix::fs::MetadataExt;
    a.metadata().ino().cmp(&b.metadata().ino())
}

#[cfg(unix)]
fn cmp_permissions(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.permissions_octal().cmp(&b.permissions_octal())
        .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn cmp_blocks(a: &File<'_>, b: &File<'_>) -> Ordering {
    blocks_value(a).cmp(&blocks_value(b))
        .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn blocks_value(f: &File<'_>) -> u64 {
    match f.blocks() {
        Blocks::Some(n) => n,
        Blocks::None    => 0,
    }
}

#[cfg(unix)]
fn cmp_hard_links(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.links().count.cmp(&b.links().count)
        .then_with(|| natord::compare(&a.name, &b.name))
}

fn cmp_flags(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.flags().0.cmp(&b.flags().0)
        .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn cmp_user_ci(a: &File<'_>, b: &File<'_>) -> Ordering {
    compare_user_names(a, b, CaseMode::Insensitive)
}

#[cfg(unix)]
fn cmp_user_cs(a: &File<'_>, b: &File<'_>) -> Ordering {
    compare_user_names(a, b, CaseMode::Sensitive)
}

#[cfg(unix)]
fn cmp_group_ci(a: &File<'_>, b: &File<'_>) -> Ordering {
    compare_group_names(a, b, CaseMode::Insensitive)
}

#[cfg(unix)]
fn cmp_group_cs(a: &File<'_>, b: &File<'_>) -> Ordering {
    compare_group_names(a, b, CaseMode::Sensitive)
}

#[cfg(unix)]
fn cmp_uid(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.user().0.cmp(&b.user().0)
        .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn cmp_gid(a: &File<'_>, b: &File<'_>) -> Ordering {
    a.group().0.cmp(&b.group().0)
        .then_with(|| natord::compare(&a.name, &b.name))
}

/// Fallback comparator for `-s vcs` when no VCS cache is available
/// (grid and lines views).  The real VCS-aware path is in
/// `FileFilter::sort_files`.
fn cmp_vcs_fallback(a: &File<'_>, b: &File<'_>) -> Ordering {
    natord::compare(&a.name, &b.name)
}


// ── User / group name resolution ────────────────────────────────

#[cfg(unix)]
#[derive(Copy, Clone)]
enum CaseMode { Sensitive, Insensitive }

#[cfg(unix)]
fn compare_user_names(a: &File<'_>, b: &File<'_>, case: CaseMode) -> Ordering {
    use uzers::Users;

    let env = crate::output::table::environment();
    let users = env.lock_users();
    let name_a = users.get_user_by_uid(a.user().0)
        .map(|u| u.name().to_string_lossy().into_owned());
    let name_b = users.get_user_by_uid(b.user().0)
        .map(|u| u.name().to_string_lossy().into_owned());
    drop(users);

    match (name_a, name_b) {
        (Some(na), Some(nb)) => case_compare(&na, &nb, case),
        (Some(_), None)      => Ordering::Less,
        (None, Some(_))      => Ordering::Greater,
        (None, None)         => a.user().0.cmp(&b.user().0),
    }
    .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn compare_group_names(a: &File<'_>, b: &File<'_>, case: CaseMode) -> Ordering {
    use uzers::Groups;

    let env = crate::output::table::environment();
    let users = env.lock_users();
    let name_a = users.get_group_by_gid(a.group().0)
        .map(|g| g.name().to_string_lossy().into_owned());
    let name_b = users.get_group_by_gid(b.group().0)
        .map(|g| g.name().to_string_lossy().into_owned());
    drop(users);

    match (name_a, name_b) {
        (Some(na), Some(nb)) => case_compare(&na, &nb, case),
        (Some(_), None)      => Ordering::Less,
        (None, Some(_))      => Ordering::Greater,
        (None, None)         => a.group().0.cmp(&b.group().0),
    }
    .then_with(|| natord::compare(&a.name, &b.name))
}

#[cfg(unix)]
fn case_compare(a: &str, b: &str, case: CaseMode) -> Ordering {
    match case {
        CaseMode::Sensitive   => natord::compare(a, b),
        CaseMode::Insensitive => natord::compare_ignore_case(a, b),
    }
}


// ── The registry ────────────────────────────────────────────────

use crate::fs::filter::SortCase;

/// Every `--sort` value accepted by lx, with its comparator, name,
/// aliases, and whether it needs VCS context.
///
/// Platform-specific entries (unix-only inode, permissions, uid,
/// etc.) are gated with `#[cfg(unix)]`.  On Windows the slice is
/// shorter at compile time.
///
/// Entries with the same `SortField` variant but different case
/// parameters (e.g. `Name(AaBbCc)` and `Name(ABCabc)`) each get
/// their own registry entry — that's how we map the string "name"
/// and "Name" to different comparators.
pub static SORT_REGISTRY: &[SortFieldDef] = &[
    // ── Name and extension ───────────────────────────────────

    SortFieldDef {
        field: SortField::Name(SortCase::AaBbCc),
        name: "name",
        aliases: &["filename"],
        description: "File name (case-insensitive, hidden files grouped)",
        compare: cmp_name_ci,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::Name(SortCase::ABCabc),
        name: "Name",
        aliases: &["Filename"],
        description: "File name (case-sensitive, uppercase first)",
        compare: cmp_name_cs,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::NameMixHidden(SortCase::AaBbCc),
        name: ".name",
        aliases: &[".filename"],
        description: "File name, hidden files mixed in (case-insensitive)",
        compare: cmp_name_mix_hidden_ci,
        needs_vcs: false,
        hidden: true,
    },
    SortFieldDef {
        field: SortField::NameMixHidden(SortCase::ABCabc),
        name: ".Name",
        aliases: &[".Filename"],
        description: "File name, hidden files mixed in (case-sensitive)",
        compare: cmp_name_mix_hidden_cs,
        needs_vcs: false,
        hidden: true,
    },
    SortFieldDef {
        field: SortField::Extension(SortCase::AaBbCc),
        name: "extension",
        aliases: &["ext"],
        description: "File extension (case-insensitive)",
        compare: cmp_extension_ci,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::Extension(SortCase::ABCabc),
        name: "Extension",
        aliases: &["Ext"],
        description: "File extension (case-sensitive)",
        compare: cmp_extension_cs,
        needs_vcs: false,
        hidden: false,
    },

    // ── Size and allocation ──────────────────────────────────

    SortFieldDef {
        field: SortField::Size,
        name: "size",
        aliases: &["filesize"],
        description: "File size in bytes",
        compare: cmp_size,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Blocks,
        name: "blocks",
        aliases: &[],
        description: "Allocated block count",
        compare: cmp_blocks,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::HardLinks,
        name: "links",
        aliases: &[],
        description: "Hard link count",
        compare: cmp_hard_links,
        needs_vcs: false,
        hidden: false,
    },

    // ── Ownership and mode ───────────────────────────────────

    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Permissions,
        name: "permissions",
        aliases: &["mode", "octal"],
        description: "Permission bits in numeric octal order",
        compare: cmp_permissions,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::Flags,
        name: "flags",
        aliases: &[],
        description: "Platform file flags (chflags/chattr) on raw bits",
        compare: cmp_flags,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::User(SortCase::AaBbCc),
        name: "user",
        aliases: &[],
        description: "Owner name (case-insensitive)",
        compare: cmp_user_ci,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::User(SortCase::ABCabc),
        name: "User",
        aliases: &[],
        description: "Owner name (case-sensitive)",
        compare: cmp_user_cs,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Group(SortCase::AaBbCc),
        name: "group",
        aliases: &[],
        description: "Group name (case-insensitive)",
        compare: cmp_group_ci,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Group(SortCase::ABCabc),
        name: "Group",
        aliases: &[],
        description: "Group name (case-sensitive)",
        compare: cmp_group_cs,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Uid,
        name: "uid",
        aliases: &[],
        description: "Numeric user ID",
        compare: cmp_uid,
        needs_vcs: false,
        hidden: false,
    },
    #[cfg(unix)]
    SortFieldDef {
        field: SortField::Gid,
        name: "gid",
        aliases: &[],
        description: "Numeric group ID",
        compare: cmp_gid,
        needs_vcs: false,
        hidden: false,
    },

    // ── Time ─────────────────────────────────────────────────

    SortFieldDef {
        field: SortField::ModifiedDate,
        name: "modified",
        aliases: &["mod", "date", "time", "new", "newest"],
        description: "Modification time (newest last)",
        compare: cmp_modified,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::ModifiedAge,
        name: "age",
        aliases: &["old", "oldest"],
        description: "Modification time (newest first)",
        compare: cmp_modified_age,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::ChangedDate,
        name: "changed",
        aliases: &["ch"],
        description: "Status-change time",
        compare: cmp_changed,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::AccessedDate,
        name: "accessed",
        aliases: &["acc"],
        description: "Access time",
        compare: cmp_accessed,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::CreatedDate,
        name: "created",
        aliases: &["cr"],
        description: "Creation time",
        compare: cmp_created,
        needs_vcs: false,
        hidden: false,
    },

    // ── VCS ──────────────────────────────────────────────────

    SortFieldDef {
        field: SortField::VcsStatusSort,
        name: "vcs",
        aliases: &[],
        description: "VCS status (attention-worthy states first)",
        compare: cmp_vcs_fallback,
        needs_vcs: true,
        hidden: false,
    },

    // ── Miscellaneous ────────────────────────────────────────

    #[cfg(unix)]
    SortFieldDef {
        field: SortField::FileInode,
        name: "inode",
        aliases: &[],
        description: "Inode number",
        compare: cmp_inode,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::FileType,
        name: "type",
        aliases: &[],
        description: "File type (directory, file, symlink, …)",
        compare: cmp_type,
        needs_vcs: false,
        hidden: false,
    },
    SortFieldDef {
        field: SortField::Unsorted,
        name: "none",
        aliases: &[],
        description: "No sorting (readdir order)",
        compare: cmp_unsorted,
        needs_vcs: false,
        hidden: false,
    },

    // Hidden alias entries for values accepted by `--sort` but that
    // map to the same SortField as a canonical entry above.  These
    // are lookup-only: they're present so `SortField::from_name`
    // resolves them, and they're added to clap's value list with
    // `hide(true)`.  The parser filters them out when building
    // the visible `--help` list by checking for an empty `name`;
    // here we use a distinct name-alias convention below instead.
];


// ── Lookup functions ────────────────────────────────────────────

impl SortFieldDef {
    /// Look up a registry entry by its `SortField` variant.
    ///
    /// Multiple entries can share the same `SortField` when they
    /// differ only by case parameter (e.g. `Name(AaBbCc)` and
    /// `Name(ABCabc)` are different variants but share the same
    /// enum outer); this returns the first match, which is always
    /// the authoritative entry for `compare_files` dispatch.
    pub fn for_field(field: SortField) -> &'static SortFieldDef {
        SORT_REGISTRY.iter()
            .find(|d| d.field == field)
            .expect("every SortField variant must have a registry entry")
    }

    /// Parse a `--sort=NAME` value.  Returns the `SortField`
    /// variant, or `None` for unrecognised names.
    ///
    /// Matches both canonical names and aliases.
    pub fn field_from_name(s: &str) -> Option<SortField> {
        SORT_REGISTRY.iter()
            .find(|d| d.name == s || d.aliases.contains(&s))
            .map(|d| d.field)
    }

    /// Iterate over all canonical names that should appear in
    /// `--help` (`hidden: false` entries only).
    pub fn visible_canonical_names() -> impl Iterator<Item = &'static str> {
        SORT_REGISTRY.iter().filter(|d| !d.hidden).map(|d| d.name)
    }

    /// Iterate over all names that should be accepted by clap but
    /// hidden from `--help`.  This is the union of:
    ///
    /// 1. canonical names for registry entries marked `hidden: true`
    ///    (e.g. `.name`, `.Name`), and
    /// 2. all alias names from every entry.
    pub fn all_hidden_names() -> impl Iterator<Item = &'static str> {
        let hidden_canonicals = SORT_REGISTRY.iter()
            .filter(|d| d.hidden)
            .map(|d| d.name);
        let all_aliases = SORT_REGISTRY.iter()
            .flat_map(|d| d.aliases.iter().copied());
        hidden_canonicals.chain(all_aliases)
    }
}
