//! Data-driven column registry.
//!
//! Every column in lx is described by a [`ColumnDef`] entry in the
//! [`COLUMN_REGISTRY`] slice.  The parser, table, view, and config
//! systems derive their behaviour from this registry rather than
//! hand-wiring each column.

use crate::fs::File;
use crate::fs::feature::VcsCache;
use crate::options::flags;
use locale;
use crate::output::cell::TextCell;
use crate::output::table::{Alignment, Column, Environment, SizeFormat, TimeType};
use crate::output::time::TimeFormat;
use crate::theme::Theme;


// ── RenderContext ────────────────────────────────────────────────

/// Everything a column renderer might need.  Passed by reference
/// to each render function; simpler columns ignore the fields they
/// don't use.
pub struct RenderContext<'a> {
    pub theme: &'a Theme,
    pub size_format: SizeFormat,
    pub time_format: &'a TimeFormat,
    pub env: &'a Environment,
    pub numeric: &'a locale::Numeric,
    pub vcs: Option<&'a dyn VcsCache>,
    pub total_size: bool,
}


// ── Render function type ────────────────────────────────────────

/// Signature for column render functions.
pub type RenderFn = fn(&RenderContext<'_>, &File<'_>, bool) -> TextCell;


// ── ColumnDef ───────────────────────────────────────────────────

/// Metadata for a single column.  The registry is a static slice of
/// these; everything else derives from it.
pub struct ColumnDef {
    /// The `Column` enum variant this definition describes.
    pub column: Column,

    /// Canonical name for `--columns` and config files.
    pub name: &'static str,

    /// Alternative names accepted by `from_name()`.
    pub aliases: &'static [&'static str],

    /// Header text shown in the header row.
    pub header: &'static str,

    /// Column alignment.
    pub alignment: Alignment,

    /// Canonical position index (lower = further left).
    pub canonical_position: u16,

    /// CLI flag that adds this column (e.g. `flags::INODE`), if any.
    pub add_flag: Option<&'static str>,

    /// CLI flag that suppresses this column, if any.
    pub suppress_flag: Option<&'static str>,

    /// CLI flag that re-enables this column after suppression, if any.
    pub show_flag: Option<&'static str>,

    /// Function to render this column's cell.
    pub render: RenderFn,
}


// ── Render wrapper functions ────────────────────────────────────
//
// Thin wrappers that bridge the uniform RenderFn signature to the
// existing per-type `.render()` methods.  The underlying methods
// and Colours traits are unchanged.

fn render_permissions(ctx: &RenderContext<'_>, file: &File<'_>, xattrs: bool) -> TextCell {
    use crate::fs::fields as f;
    let pp = f::PermissionsPlus {
        file_type: file.type_char(),
        #[cfg(unix)]
        permissions: file.permissions(),
        #[cfg(windows)]
        attributes: file.attributes(),
        xattrs,
    };
    pp.render(ctx.theme)
}

fn render_size(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    if ctx.total_size {
        file.total_size().render(ctx.theme, ctx.size_format, &ctx.numeric)
    } else {
        file.size().render(ctx.theme, ctx.size_format, &ctx.numeric)
    }
}

#[cfg(unix)]
fn render_hard_links(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.links().render(ctx.theme, &ctx.numeric)
}

#[cfg(unix)]
fn render_inode(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.inode().render(ctx.theme.ui.inode)
}

#[cfg(unix)]
fn render_blocks(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.blocks().render(ctx.theme)
}

#[cfg(unix)]
fn render_user(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.user().render(ctx.theme, &*ctx.env.lock_users())
}

#[cfg(unix)]
fn render_uid(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.user().render_uid(ctx.theme, &*ctx.env.lock_users())
}

#[cfg(unix)]
fn render_group(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.group().render(ctx.theme, &*ctx.env.lock_users())
}

#[cfg(unix)]
fn render_gid(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.group().render_gid(ctx.theme, &*ctx.env.lock_users())
}

fn render_vcs_status(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    let status = ctx.vcs
        .map(|g| g.get(&file.path, file.is_directory()))
        .unwrap_or_default();
    let backend = ctx.vcs
        .map(VcsCache::header_name)
        .unwrap_or("VCS");
    status.render(ctx.theme, backend)
}

fn render_vcs_repos(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.vcs_repo_status().render(ctx.theme)
}

#[cfg(unix)]
fn render_octal(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    use crate::fs::fields as f;
    let op = f::OctalPermissions {
        permissions: file.permissions(),
    };
    op.render(ctx.theme.ui.octal)
}

fn render_flags(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    file.flags().render(ctx.theme.ui.flags)
}

fn render_modified(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    use crate::output::render::TimeRender;
    file.modified_time().render(&ctx.theme.ui.date, ctx.time_format)
}

fn render_changed(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    use crate::output::render::TimeRender;
    file.changed_time().render(&ctx.theme.ui.date, ctx.time_format)
}

fn render_accessed(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    use crate::output::render::TimeRender;
    file.accessed_time().render(&ctx.theme.ui.date, ctx.time_format)
}

fn render_created(ctx: &RenderContext<'_>, file: &File<'_>, _xattrs: bool) -> TextCell {
    use crate::output::render::TimeRender;
    file.created_time().render(&ctx.theme.ui.date, ctx.time_format)
}


// ── The registry ────────────────────────────────────────────────

/// All column definitions.  Platform-gated entries are excluded at
/// compile time via `#[cfg]`.
pub static COLUMN_REGISTRY: &[ColumnDef] = &[
    #[cfg(unix)]
    ColumnDef {
        column: Column::Inode,
        name: "inode",
        aliases: &[],
        header: "inode",
        alignment: Alignment::Right,
        canonical_position: 0,
        add_flag: Some(flags::INODE),
        suppress_flag: Some(flags::NO_INODE),
        show_flag: None,
        render: render_inode,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::Octal,
        name: "octal",
        aliases: &[],
        header: "Octal",
        alignment: Alignment::Left,
        canonical_position: 1,
        add_flag: Some(flags::OCTAL),
        suppress_flag: Some(flags::NO_OCTAL),
        show_flag: None,
        render: render_octal,
    },
    ColumnDef {
        column: Column::Permissions,
        name: "permissions",
        // `perms` kept as an alias for backward compatibility with
        // pre-0.8 configs that use `columns = ["perms", ...]`.
        aliases: &["perms"],
        #[cfg(unix)]
        header: "Permissions",
        #[cfg(windows)]
        header: "Mode",
        alignment: Alignment::Left,
        canonical_position: 2,
        add_flag: None,
        suppress_flag: Some(flags::NO_PERMISSIONS),
        show_flag: Some(flags::SHOW_PERMISSIONS),
        render: render_permissions,
    },
    ColumnDef {
        column: Column::Flags,
        name: "flags",
        aliases: &[],
        header: "Flags",
        alignment: Alignment::Left,
        canonical_position: 3,
        add_flag: Some(flags::FILE_FLAGS),
        suppress_flag: Some(flags::NO_FLAGS),
        show_flag: None,
        render: render_flags,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::HardLinks,
        name: "links",
        aliases: &[],
        header: "Links",
        alignment: Alignment::Right,
        canonical_position: 4,
        add_flag: Some(flags::LINKS),
        suppress_flag: Some(flags::NO_LINKS),
        show_flag: None,
        render: render_hard_links,
    },
    ColumnDef {
        column: Column::FileSize,
        name: "size",
        aliases: &["filesize"],
        header: "Size",
        alignment: Alignment::Right,
        canonical_position: 5,
        add_flag: None,
        suppress_flag: Some(flags::NO_FILESIZE),
        show_flag: Some(flags::SHOW_FILESIZE),
        render: render_size,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::Blocks,
        name: "blocks",
        aliases: &[],
        header: "Blocks",
        alignment: Alignment::Right,
        canonical_position: 6,
        add_flag: Some(flags::BLOCKS),
        suppress_flag: Some(flags::NO_BLOCKS),
        show_flag: None,
        render: render_blocks,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::User,
        name: "user",
        aliases: &[],
        header: "User",
        alignment: Alignment::Left,
        canonical_position: 7,
        add_flag: None,
        suppress_flag: Some(flags::NO_USER),
        show_flag: Some(flags::SHOW_USER),
        render: render_user,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::Uid,
        name: "uid",
        aliases: &[],
        header: "UID",
        alignment: Alignment::Right,
        canonical_position: 8,
        add_flag: Some(flags::UID),
        suppress_flag: Some(flags::NO_UID),
        show_flag: None,
        render: render_uid,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::Group,
        name: "group",
        aliases: &[],
        header: "Group",
        alignment: Alignment::Left,
        canonical_position: 9,
        add_flag: Some(flags::GROUP),
        suppress_flag: Some(flags::NO_GROUP),
        show_flag: None,
        render: render_group,
    },
    #[cfg(unix)]
    ColumnDef {
        column: Column::Gid,
        name: "gid",
        aliases: &[],
        header: "GID",
        alignment: Alignment::Right,
        canonical_position: 10,
        add_flag: Some(flags::GID),
        suppress_flag: Some(flags::NO_GID),
        show_flag: None,
        render: render_gid,
    },
    ColumnDef {
        column: Column::Timestamp(TimeType::Modified),
        name: "modified",
        aliases: &[],
        header: "Date Modified",
        alignment: Alignment::Left,
        canonical_position: 11,
        add_flag: Some(flags::MODIFIED),
        suppress_flag: Some(flags::NO_MODIFIED),
        show_flag: None,
        render: render_modified,
    },
    ColumnDef {
        column: Column::Timestamp(TimeType::Changed),
        name: "changed",
        aliases: &[],
        header: "Date Changed",
        alignment: Alignment::Left,
        canonical_position: 12,
        add_flag: Some(flags::CHANGED),
        suppress_flag: Some(flags::NO_CHANGED),
        show_flag: None,
        render: render_changed,
    },
    ColumnDef {
        column: Column::Timestamp(TimeType::Created),
        name: "created",
        aliases: &[],
        header: "Date Created",
        alignment: Alignment::Left,
        canonical_position: 13,
        add_flag: Some(flags::CREATED),
        suppress_flag: Some(flags::NO_CREATED),
        show_flag: None,
        render: render_created,
    },
    ColumnDef {
        column: Column::Timestamp(TimeType::Accessed),
        name: "accessed",
        aliases: &[],
        header: "Date Accessed",
        alignment: Alignment::Left,
        canonical_position: 14,
        add_flag: Some(flags::ACCESSED),
        suppress_flag: Some(flags::NO_ACCESSED),
        show_flag: None,
        render: render_accessed,
    },
    ColumnDef {
        column: Column::VcsStatus,
        name: "vcs",
        aliases: &[],
        header: "VCS",
        alignment: Alignment::Right,
        canonical_position: 15,
        add_flag: Some(flags::VCS_STATUS),
        suppress_flag: Some(flags::NO_VCS_STATUS),
        show_flag: None,
        render: render_vcs_status,
    },
    ColumnDef {
        column: Column::VcsRepos,
        name: "repos",
        aliases: &[],
        header: "Repo",
        alignment: Alignment::Left,
        canonical_position: 16,
        add_flag: Some(flags::VCS_REPOS),
        suppress_flag: Some(flags::NO_VCS_REPOS),
        show_flag: None,
        render: render_vcs_repos,
    },
];


// ── Lookup functions ────────────────────────────────────────────

impl ColumnDef {
    /// Look up a column definition by its `Column` enum variant.
    pub fn for_column(col: Column) -> &'static ColumnDef {
        COLUMN_REGISTRY.iter()
            .find(|d| d.column == col)
            .expect("every Column variant must have a ColumnDef entry")
    }

    /// Parse a column name (from `--columns` or config).  Returns the
    /// `Column` variant, or `None` for unrecognised names.
    pub fn column_from_name(s: &str) -> Option<Column> {
        COLUMN_REGISTRY.iter()
            .find(|d| d.name == s || d.aliases.contains(&s))
            .map(|d| d.column)
    }

    /// Comma-separated list of all known column canonical names, in
    /// registry order.  Used for "[possible values: ...]" hints in
    /// error messages from `--columns=` parsing.
    pub fn all_names_csv() -> String {
        COLUMN_REGISTRY.iter()
            .map(|d| d.name)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
