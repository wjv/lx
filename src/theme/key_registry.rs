//! Theme key registry: the single source of truth for every named
//! `[theme.NAME]` config key understood by lx.
//!
//! Every recognised key has one [`ThemeKeyDef`] entry.  `set_config`
//! dispatches through the registry; `--dump-theme` walks the registry
//! to emit canonical key names and current values.  Aliases are
//! accepted on input but never emitted on dump.
//!
//! Adding a key:
//! 1. Add the field to the relevant sub-struct in `ui_styles.rs`
//!    (or directly on `UiStyles` for top-level keys).
//! 2. Add a [`ThemeKeyDef`] entry below in the matching family
//!    helper function.
//! 3. Add the value to each compiled-in theme function in
//!    `default_theme.rs`.
//! 4. (If user-visible) document in `man/lxconfig.toml.5`.

use std::sync::LazyLock;

use nu_ansi_term::Style;

use super::ui_styles::UiStyles;

/// Logical grouping for ordering and blank-line separation in
/// `--dump-theme` output.  Variant order is the dump order; keep
/// it stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThemeFamily {
    FileKinds,
    Permissions,
    Size,
    Users,
    Links,
    Vcs,
    Punctuation,
    Date,
    Columns,
    Symlinks,
}

/// How the registry reads or writes the underlying `Style`.
pub enum StyleAccess {
    /// One-to-one mapping with a single `Style` field.  Both
    /// reading (for `--dump-theme`) and writing (for `set_config`)
    /// work.
    Direct {
        // `get` is consumed by `--dump-theme` for compiled-in
        // themes (wjv/lx#14), wired up alongside the
        // dumpable/family-grouping work.
        #[allow(dead_code)]
        get: fn(&UiStyles) -> Style,
        set: fn(&mut UiStyles, Style),
    },

    /// Bulk setter — `size-number`, `date`, `date-modified`, etc. —
    /// fans out to multiple fields.  Setter only; never emitted by
    /// `--dump-theme`, since the same colour is already covered by
    /// the per-field entries it expands to.
    Bulk { set: fn(&mut UiStyles, Style) },
}

/// One entry per named theme key.  See [`THEME_KEY_REGISTRY`].
pub struct ThemeKeyDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    // Read by `--dump-theme` for canonical key grouping (wjv/lx#13),
    // wired up alongside the dumpable iterator below.
    #[allow(dead_code)]
    pub family: ThemeFamily,
    pub access: StyleAccess,
}

impl ThemeKeyDef {
    /// Look up a registry entry by its canonical name or any alias.
    pub fn from_name(name: &str) -> Option<&'static ThemeKeyDef> {
        THEME_KEY_REGISTRY
            .iter()
            .find(|d| d.name == name || d.aliases.contains(&name))
    }

    /// Iterator over all `Direct` entries, in family-then-name order
    /// — the canonical dump order.  Bulk entries are skipped.
    // Consumed by `--dump-theme` for compiled-in themes
    // (wjv/lx#14) once that landing wires it up.
    #[allow(dead_code)]
    pub fn dumpable() -> impl Iterator<Item = &'static ThemeKeyDef> {
        THEME_KEY_REGISTRY
            .iter()
            .filter(|d| matches!(d.access, StyleAccess::Direct { .. }))
    }
}

pub static THEME_KEY_REGISTRY: LazyLock<Vec<ThemeKeyDef>> = LazyLock::new(build_registry);

fn direct(
    name: &'static str,
    aliases: &'static [&'static str],
    family: ThemeFamily,
    get: fn(&UiStyles) -> Style,
    set: fn(&mut UiStyles, Style),
) -> ThemeKeyDef {
    ThemeKeyDef {
        name,
        aliases,
        family,
        access: StyleAccess::Direct { get, set },
    }
}

fn bulk(name: &'static str, family: ThemeFamily, set: fn(&mut UiStyles, Style)) -> ThemeKeyDef {
    ThemeKeyDef {
        name,
        aliases: &[],
        family,
        access: StyleAccess::Bulk { set },
    }
}

#[rustfmt::skip]
fn build_registry() -> Vec<ThemeKeyDef> {
    let mut r = Vec::with_capacity(120);
    file_kinds(&mut r);
    permissions(&mut r);
    size(&mut r);
    users(&mut r);
    links(&mut r);
    vcs(&mut r);
    punctuation(&mut r);
    date(&mut r);
    columns(&mut r);
    symlinks(&mut r);
    r
}

// ── File kinds ───────────────────────────────────────────────────

#[rustfmt::skip]
fn file_kinds(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::FileKinds as F;
    r.push(direct("normal",       &[], F, |u| u.filekinds.normal,       |u, s| u.filekinds.normal = s));
    r.push(direct("directory",    &[], F, |u| u.filekinds.directory,    |u, s| u.filekinds.directory = s));
    r.push(direct("symlink",      &[], F, |u| u.filekinds.symlink,      |u, s| u.filekinds.symlink = s));
    r.push(direct("pipe",         &[], F, |u| u.filekinds.pipe,         |u, s| u.filekinds.pipe = s));
    r.push(direct("block-device", &[], F, |u| u.filekinds.block_device, |u, s| u.filekinds.block_device = s));
    r.push(direct("char-device",  &[], F, |u| u.filekinds.char_device,  |u, s| u.filekinds.char_device = s));
    r.push(direct("socket",       &[], F, |u| u.filekinds.socket,       |u, s| u.filekinds.socket = s));
    r.push(direct("special",      &[], F, |u| u.filekinds.special,      |u, s| u.filekinds.special = s));
    r.push(direct("executable",   &[], F, |u| u.filekinds.executable,   |u, s| u.filekinds.executable = s));
}

// ── Permissions ──────────────────────────────────────────────────
//
// Three accepted prefixes per key: `permissions-*` (canonical, matches
// the column name), `perm-*` (legacy short form), and `mode-*`
// (matches the `--mode` flag alias and the `mode` sort field).

#[rustfmt::skip]
fn permissions(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Permissions as F;

    r.push(direct("permissions-user-read", &["perm-user-read", "mode-user-read"], F,
        |u| u.perms.user_read, |u, s| u.perms.user_read = s));
    r.push(direct("permissions-user-write", &["perm-user-write", "mode-user-write"], F,
        |u| u.perms.user_write, |u, s| u.perms.user_write = s));
    r.push(direct("permissions-user-execute", &["perm-user-exec", "mode-user-exec"], F,
        |u| u.perms.user_execute_file, |u, s| u.perms.user_execute_file = s));
    r.push(direct("permissions-user-execute-other", &["perm-user-exec-other", "mode-user-exec-other"], F,
        |u| u.perms.user_execute_other, |u, s| u.perms.user_execute_other = s));

    r.push(direct("permissions-group-read", &["perm-group-read", "mode-group-read"], F,
        |u| u.perms.group_read, |u, s| u.perms.group_read = s));
    r.push(direct("permissions-group-write", &["perm-group-write", "mode-group-write"], F,
        |u| u.perms.group_write, |u, s| u.perms.group_write = s));
    r.push(direct("permissions-group-execute", &["perm-group-exec", "mode-group-exec"], F,
        |u| u.perms.group_execute, |u, s| u.perms.group_execute = s));

    r.push(direct("permissions-other-read", &["perm-other-read", "mode-other-read"], F,
        |u| u.perms.other_read, |u, s| u.perms.other_read = s));
    r.push(direct("permissions-other-write", &["perm-other-write", "mode-other-write"], F,
        |u| u.perms.other_write, |u, s| u.perms.other_write = s));
    r.push(direct("permissions-other-execute", &["perm-other-exec", "mode-other-exec"], F,
        |u| u.perms.other_execute, |u, s| u.perms.other_execute = s));

    r.push(direct("permissions-special-user", &["perm-special-user", "mode-special-user"], F,
        |u| u.perms.special_user_file, |u, s| u.perms.special_user_file = s));
    r.push(direct("permissions-special-other", &["perm-special-other", "mode-special-other"], F,
        |u| u.perms.special_other, |u, s| u.perms.special_other = s));
    r.push(direct("permissions-attribute", &["perm-attribute", "mode-attribute"], F,
        |u| u.perms.attribute, |u, s| u.perms.attribute = s));
}

// ── Size ─────────────────────────────────────────────────────────

#[rustfmt::skip]
fn size(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Size as F;
    // Bulk setters first; they fan out to the per-tier fields plus
    // the `major`/`minor` flat-fallback colours.
    r.push(bulk("size-number", F, UiStyles::set_number_style));
    r.push(bulk("size-unit",   F, UiStyles::set_unit_style));

    r.push(direct("size-major",       &[], F, |u| u.size.major,       |u, s| u.size.major = s));
    r.push(direct("size-minor",       &[], F, |u| u.size.minor,       |u, s| u.size.minor = s));

    r.push(direct("size-number-byte", &[], F, |u| u.size.number_byte, |u, s| u.size.number_byte = s));
    r.push(direct("size-number-kilo", &[], F, |u| u.size.number_kilo, |u, s| u.size.number_kilo = s));
    r.push(direct("size-number-mega", &[], F, |u| u.size.number_mega, |u, s| u.size.number_mega = s));
    r.push(direct("size-number-giga", &[], F, |u| u.size.number_giga, |u, s| u.size.number_giga = s));
    r.push(direct("size-number-huge", &[], F, |u| u.size.number_huge, |u, s| u.size.number_huge = s));

    r.push(direct("size-unit-byte",   &[], F, |u| u.size.unit_byte,   |u, s| u.size.unit_byte = s));
    r.push(direct("size-unit-kilo",   &[], F, |u| u.size.unit_kilo,   |u, s| u.size.unit_kilo = s));
    r.push(direct("size-unit-mega",   &[], F, |u| u.size.unit_mega,   |u, s| u.size.unit_mega = s));
    r.push(direct("size-unit-giga",   &[], F, |u| u.size.unit_giga,   |u, s| u.size.unit_giga = s));
    r.push(direct("size-unit-huge",   &[], F, |u| u.size.unit_huge,   |u, s| u.size.unit_huge = s));
}

// ── Users (user / group / uid / gid) ─────────────────────────────

#[rustfmt::skip]
fn users(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Users as F;
    r.push(direct("user-you",     &[], F, |u| u.users.user_you,          |u, s| u.users.user_you = s));
    r.push(direct("user-other",   &[], F, |u| u.users.user_someone_else, |u, s| u.users.user_someone_else = s));

    r.push(direct("group-yours",  &[], F, |u| u.users.group_yours,       |u, s| u.users.group_yours = s));
    r.push(direct("group-member", &[], F, |u| u.users.group_member,      |u, s| u.users.group_member = s));
    r.push(direct("group-other",  &[], F, |u| u.users.group_not_yours,   |u, s| u.users.group_not_yours = s));

    r.push(direct("uid-you",      &[], F, |u| u.users.uid_you,           |u, s| u.users.uid_you = s));
    r.push(direct("uid-other",    &[], F, |u| u.users.uid_someone_else,  |u, s| u.users.uid_someone_else = s));

    r.push(direct("gid-yours",    &[], F, |u| u.users.gid_yours,         |u, s| u.users.gid_yours = s));
    r.push(direct("gid-member",   &[], F, |u| u.users.gid_member,        |u, s| u.users.gid_member = s));
    r.push(direct("gid-other",    &[], F, |u| u.users.gid_not_yours,     |u, s| u.users.gid_not_yours = s));
}

// ── Links ────────────────────────────────────────────────────────

#[rustfmt::skip]
fn links(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Links as F;
    r.push(direct("links",       &[], F, |u| u.links.normal,          |u, s| u.links.normal = s));
    r.push(direct("links-multi", &[], F, |u| u.links.multi_link_file, |u, s| u.links.multi_link_file = s));
}

// ── VCS ──────────────────────────────────────────────────────────

#[rustfmt::skip]
fn vcs(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Vcs as F;
    r.push(direct("vcs-new",        &[], F, |u| u.vcs.new,        |u, s| u.vcs.new = s));
    r.push(direct("vcs-modified",   &[], F, |u| u.vcs.modified,   |u, s| u.vcs.modified = s));
    r.push(direct("vcs-deleted",    &[], F, |u| u.vcs.deleted,    |u, s| u.vcs.deleted = s));
    r.push(direct("vcs-renamed",    &[], F, |u| u.vcs.renamed,    |u, s| u.vcs.renamed = s));
    r.push(direct("vcs-typechange", &[], F, |u| u.vcs.typechange, |u, s| u.vcs.typechange = s));
    r.push(direct("vcs-ignored",    &[], F, |u| u.vcs.ignored,    |u, s| u.vcs.ignored = s));
    r.push(direct("vcs-conflicted", &[], F, |u| u.vcs.conflicted, |u, s| u.vcs.conflicted = s));
}

// ── Punctuation ──────────────────────────────────────────────────

#[rustfmt::skip]
fn punctuation(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Punctuation as F;
    r.push(direct("punctuation", &[], F, |u| u.punctuation, |u, s| u.punctuation = s));
}

// ── Date keys ────────────────────────────────────────────────────
//
// Bulk fan-outs first (`date`, `date-now`, …), then per-column bulk
// (`date-modified`, …), then 32 per-column-per-tier direct entries.
// All hand-written: closures with non-runtime field paths can't be
// generated by a loop without a macro, and we keep the file
// macro-free.

#[rustfmt::skip]
fn date(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Date as F;

    // Bulk fan-outs across all four columns.
    r.push(bulk("date",       F, |u, s| u.date_for_each(|d| d.set_all(s))));
    r.push(bulk("date-now",   F, |u, s| u.date_for_each(|d| d.now = s)));
    r.push(bulk("date-today", F, |u, s| u.date_for_each(|d| d.today = s)));
    r.push(bulk("date-week",  F, |u, s| u.date_for_each(|d| d.week = s)));
    r.push(bulk("date-month", F, |u, s| u.date_for_each(|d| d.month = s)));
    r.push(bulk("date-year",  F, |u, s| u.date_for_each(|d| d.year = s)));
    r.push(bulk("date-old",   F, |u, s| u.date_for_each(|d| d.old = s)));
    r.push(bulk("date-flat",  F, |u, s| u.date_for_each(|d| d.flat = s)));

    // Per-column bulk setters (set_all on one DateAge).
    r.push(bulk("date-modified", F, |u, s| u.date_modified.set_all(s)));
    r.push(bulk("date-accessed", F, |u, s| u.date_accessed.set_all(s)));
    r.push(bulk("date-changed",  F, |u, s| u.date_changed.set_all(s)));
    r.push(bulk("date-created",  F, |u, s| u.date_created.set_all(s)));

    // date-modified-*
    r.push(direct("date-modified-now",   &[], F, |u| u.date_modified.now,   |u, s| u.date_modified.now = s));
    r.push(direct("date-modified-today", &[], F, |u| u.date_modified.today, |u, s| u.date_modified.today = s));
    r.push(direct("date-modified-week",  &[], F, |u| u.date_modified.week,  |u, s| u.date_modified.week = s));
    r.push(direct("date-modified-month", &[], F, |u| u.date_modified.month, |u, s| u.date_modified.month = s));
    r.push(direct("date-modified-year",  &[], F, |u| u.date_modified.year,  |u, s| u.date_modified.year = s));
    r.push(direct("date-modified-old",   &[], F, |u| u.date_modified.old,   |u, s| u.date_modified.old = s));
    r.push(direct("date-modified-flat",  &[], F, |u| u.date_modified.flat,  |u, s| u.date_modified.flat = s));

    // date-accessed-*
    r.push(direct("date-accessed-now",   &[], F, |u| u.date_accessed.now,   |u, s| u.date_accessed.now = s));
    r.push(direct("date-accessed-today", &[], F, |u| u.date_accessed.today, |u, s| u.date_accessed.today = s));
    r.push(direct("date-accessed-week",  &[], F, |u| u.date_accessed.week,  |u, s| u.date_accessed.week = s));
    r.push(direct("date-accessed-month", &[], F, |u| u.date_accessed.month, |u, s| u.date_accessed.month = s));
    r.push(direct("date-accessed-year",  &[], F, |u| u.date_accessed.year,  |u, s| u.date_accessed.year = s));
    r.push(direct("date-accessed-old",   &[], F, |u| u.date_accessed.old,   |u, s| u.date_accessed.old = s));
    r.push(direct("date-accessed-flat",  &[], F, |u| u.date_accessed.flat,  |u, s| u.date_accessed.flat = s));

    // date-changed-*
    r.push(direct("date-changed-now",    &[], F, |u| u.date_changed.now,    |u, s| u.date_changed.now = s));
    r.push(direct("date-changed-today",  &[], F, |u| u.date_changed.today,  |u, s| u.date_changed.today = s));
    r.push(direct("date-changed-week",   &[], F, |u| u.date_changed.week,   |u, s| u.date_changed.week = s));
    r.push(direct("date-changed-month",  &[], F, |u| u.date_changed.month,  |u, s| u.date_changed.month = s));
    r.push(direct("date-changed-year",   &[], F, |u| u.date_changed.year,   |u, s| u.date_changed.year = s));
    r.push(direct("date-changed-old",    &[], F, |u| u.date_changed.old,    |u, s| u.date_changed.old = s));
    r.push(direct("date-changed-flat",   &[], F, |u| u.date_changed.flat,  |u, s| u.date_changed.flat = s));

    // date-created-*
    r.push(direct("date-created-now",    &[], F, |u| u.date_created.now,    |u, s| u.date_created.now = s));
    r.push(direct("date-created-today",  &[], F, |u| u.date_created.today,  |u, s| u.date_created.today = s));
    r.push(direct("date-created-week",   &[], F, |u| u.date_created.week,   |u, s| u.date_created.week = s));
    r.push(direct("date-created-month",  &[], F, |u| u.date_created.month,  |u, s| u.date_created.month = s));
    r.push(direct("date-created-year",   &[], F, |u| u.date_created.year,   |u, s| u.date_created.year = s));
    r.push(direct("date-created-old",    &[], F, |u| u.date_created.old,    |u, s| u.date_created.old = s));
    r.push(direct("date-created-flat",   &[], F, |u| u.date_created.flat,   |u, s| u.date_created.flat = s));
}

// ── Top-level column styles ──────────────────────────────────────

#[rustfmt::skip]
fn columns(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Columns as F;
    r.push(direct("inode",  &[], F, |u| u.inode,  |u, s| u.inode = s));
    r.push(direct("blocks", &[], F, |u| u.blocks, |u, s| u.blocks = s));
    r.push(direct("header", &[], F, |u| u.header, |u, s| u.header = s));
    r.push(direct("octal",  &[], F, |u| u.octal,  |u, s| u.octal = s));
    r.push(direct("flags",  &[], F, |u| u.flags,  |u, s| u.flags = s));
}

// ── Symlink and broken-link overlays ─────────────────────────────

#[rustfmt::skip]
fn symlinks(r: &mut Vec<ThemeKeyDef>) {
    use ThemeFamily::Symlinks as F;
    r.push(direct("symlink-path",   &[], F, |u| u.symlink_path,        |u, s| u.symlink_path = s));
    r.push(direct("control-char",   &[], F, |u| u.control_char,        |u, s| u.control_char = s));
    r.push(direct("broken-symlink", &[], F, |u| u.broken_symlink,      |u, s| u.broken_symlink = s));
    r.push(direct("broken-overlay", &[], F, |u| u.broken_path_overlay, |u, s| u.broken_path_overlay = s));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_populated() {
        // Sanity check on the size: ~109 entries today.  Growth
        // is fine; a sudden collapse would mean a section was
        // accidentally dropped.
        assert!(THEME_KEY_REGISTRY.len() >= 100);
    }

    #[test]
    fn lookup_canonical_name() {
        let def = ThemeKeyDef::from_name("permissions-user-read").unwrap();
        assert_eq!(def.name, "permissions-user-read");
    }

    #[test]
    fn lookup_alias() {
        let def = ThemeKeyDef::from_name("perm-user-read").unwrap();
        assert_eq!(def.name, "permissions-user-read");
        let def = ThemeKeyDef::from_name("mode-user-read").unwrap();
        assert_eq!(def.name, "permissions-user-read");
    }

    #[test]
    fn unknown_key_returns_none() {
        assert!(ThemeKeyDef::from_name("not-a-real-key").is_none());
    }

    #[test]
    fn no_duplicate_names_or_aliases() {
        // Every canonical name and alias must be unique across the
        // whole registry, otherwise lookup is ambiguous.
        let mut seen = std::collections::HashSet::new();
        for def in THEME_KEY_REGISTRY.iter() {
            assert!(seen.insert(def.name), "duplicate key name: {}", def.name);
            for alias in def.aliases {
                assert!(seen.insert(alias), "duplicate alias: {}", alias);
            }
        }
    }

    #[test]
    fn dumpable_skips_bulk() {
        let dumpable_count = ThemeKeyDef::dumpable().count();
        let total = THEME_KEY_REGISTRY.len();
        // Bulk entries: 2 size + 8 date fan-outs + 4 date-per-col = 14.
        assert!(dumpable_count < total);
        assert!(dumpable_count > 0);
    }

    #[test]
    fn direct_round_trip() {
        // Set a style on a UiStyles via the registry's setter and
        // read it back via the getter.  Smoke-tests that get/set
        // pairs reach the same field.
        let mut ui = UiStyles::default();
        let style = Style::new().bold();

        for def in ThemeKeyDef::dumpable() {
            if let StyleAccess::Direct { get, set } = def.access {
                set(&mut ui, style);
                assert_eq!(get(&ui), style, "round-trip failed for key '{}'", def.name);
            }
        }
    }
}
