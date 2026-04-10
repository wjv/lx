use nu_ansi_term::Style;

use crate::theme::lsc::Pair;


#[derive(Debug, Default, PartialEq)]
pub struct UiStyles {
    pub colourful: bool,

    pub filekinds:  FileKinds,
    pub perms:      Permissions,
    pub size:       Size,
    pub users:      Users,
    pub links:      Links,
    pub vcs:        Git,

    pub punctuation:  Style,
    /// Per-timestamp-column age styles.  All four columns share the
    /// same shape but can be themed independently via the
    /// `date-modified-*` / `date-accessed-*` / `date-changed-*` /
    /// `date-created-*` config keys.  The bulk `date = ...` /
    /// `date-now = ...` / etc. setters fan out to all four via
    /// [`UiStyles::date_for_each`].
    pub date_modified: DateAge,
    pub date_accessed: DateAge,
    pub date_changed:  DateAge,
    pub date_created:  DateAge,
    pub inode:        Style,
    pub blocks:       Style,
    pub header:       Style,
    pub octal:        Style,
    pub flags:        Style,

    pub symlink_path:         Style,
    pub control_char:         Style,
    pub broken_symlink:       Style,
    pub broken_path_overlay:  Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FileKinds {
    pub normal: Style,
    pub directory: Style,
    pub symlink: Style,
    pub pipe: Style,
    pub block_device: Style,
    pub char_device: Style,
    pub socket: Style,
    pub special: Style,
    pub executable: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Permissions {
    pub user_read:          Style,
    pub user_write:         Style,
    pub user_execute_file:  Style,
    pub user_execute_other: Style,

    pub group_read:    Style,
    pub group_write:   Style,
    pub group_execute: Style,

    pub other_read:    Style,
    pub other_write:   Style,
    pub other_execute: Style,

    pub special_user_file: Style,
    pub special_other:     Style,

    pub attribute: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub major: Style,
    pub minor: Style,

    pub number_byte: Style,
    pub number_kilo: Style,
    pub number_mega: Style,
    pub number_giga: Style,
    pub number_huge: Style,

    pub unit_byte: Style,
    pub unit_kilo: Style,
    pub unit_mega: Style,
    pub unit_giga: Style,
    pub unit_huge: Style,
}

/// Age-based timestamp styles.  Six tiers from "just now" to "old",
/// plus a single `flat` colour used when the date column's gradient
/// is disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DateAge {
    pub now:   Style,   // < 1 hour
    pub today: Style,   // < 24 hours
    pub week:  Style,   // < 7 days
    pub month: Style,   // < 30 days
    pub year:  Style,   // < 365 days
    pub old:   Style,   // > 1 year

    /// Flat single-tone colour used when the date column is rendered
    /// without an age gradient.  Mirrors `Size::major`/`Size::minor`,
    /// which fill the same role for the size column.
    pub flat: Style,
}

impl DateAge {
    /// Set all tiers to the same style (bulk setter for `date = ...`).
    /// Also sets `flat` so the bulk setter behaves the way a user
    /// would expect: "make the date column this colour, full stop".
    pub fn set_all(&mut self, style: Style) {
        self.now = style;
        self.today = style;
        self.week = style;
        self.month = style;
        self.year = style;
        self.old = style;
        self.flat = style;
    }

    /// Pick the style for a given age in seconds.
    pub fn for_age(&self, age_secs: u64) -> Style {
        const HOUR: u64 = 3600;
        const DAY: u64 = 86400;
        const WEEK: u64 = 7 * DAY;
        const MONTH: u64 = 30 * DAY;
        const YEAR: u64 = 365 * DAY;

        if age_secs < HOUR       { self.now }
        else if age_secs < DAY   { self.today }
        else if age_secs < WEEK  { self.week }
        else if age_secs < MONTH { self.month }
        else if age_secs < YEAR  { self.year }
        else                     { self.old }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Users {
    pub user_you: Style,
    pub user_someone_else: Style,
    pub group_yours: Style,
    pub group_member: Style,
    pub group_not_yours: Style,
    pub uid_you: Style,
    pub uid_someone_else: Style,
    pub gid_yours: Style,
    pub gid_member: Style,
    pub gid_not_yours: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Links {
    pub normal: Style,
    pub multi_link_file: Style,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Git {
    pub new: Style,
    pub modified: Style,
    pub deleted: Style,
    pub renamed: Style,
    pub typechange: Style,
    pub ignored: Style,
    pub conflicted: Style,
}

impl UiStyles {
    pub fn plain() -> Self {
        Self::default()
    }

    /// Apply a closure to every per-timestamp-column [`DateAge`]
    /// instance.  Used by the bulk `date = ...` / `date-now = ...` /
    /// etc. setters and by the `da` / `dn` / ... `LX_COLORS` codes,
    /// so that theme authors who write a single `date` block see it
    /// applied to every timestamp column.
    ///
    /// Per-column overrides (e.g. `date-modified-now = ...`) write
    /// directly to the named field and do not go through this helper.
    pub(crate) fn date_for_each<F: FnMut(&mut DateAge)>(&mut self, mut f: F) {
        f(&mut self.date_modified);
        f(&mut self.date_accessed);
        f(&mut self.date_changed);
        f(&mut self.date_created);
    }

    /// Collapse the per-tier values of any column whose gradient is
    /// disabled into a single flat colour from the theme.
    ///
    /// - When `gradient.size` is `false`, every `size.number_*` tier
    ///   is overwritten with `size.major` and every `size.unit_*`
    ///   tier with `size.minor`.  These two slots are already
    ///   themeable (`size-major` / `size-minor`, `LX_COLORS` codes
    ///   `df` / `ds`) and serve as the column's "headline" colour
    ///   in non-tiered contexts like the `-CZ` count footer.
    /// - For each timestamp column whose flag is `false`, every age
    ///   tier (`now` through `old`) is overwritten with that column's
    ///   `flat`.  Theme authors set `date-flat` explicitly (or rely
    ///   on the bulk `date = "..."` setter, which also touches
    ///   `flat`).
    ///
    /// Runs once at theme construction so the renderers themselves
    /// stay oblivious to the on/off state.
    pub fn apply_gradient_flags(&mut self, gradient: super::GradientFlags) {
        if !gradient.size {
            self.size.number_byte = self.size.major;
            self.size.number_kilo = self.size.major;
            self.size.number_mega = self.size.major;
            self.size.number_giga = self.size.major;
            self.size.number_huge = self.size.major;
            self.size.unit_byte   = self.size.minor;
            self.size.unit_kilo   = self.size.minor;
            self.size.unit_mega   = self.size.minor;
            self.size.unit_giga   = self.size.minor;
            self.size.unit_huge   = self.size.minor;
        }
        if !gradient.modified { flatten_date_age(&mut self.date_modified); }
        if !gradient.accessed { flatten_date_age(&mut self.date_accessed); }
        if !gradient.changed  { flatten_date_age(&mut self.date_changed);  }
        if !gradient.created  { flatten_date_age(&mut self.date_created);  }
    }
}

/// Collapse a single [`DateAge`] to its `flat` colour: every age
/// tier (`now` through `old`) becomes `flat`.  Used by
/// [`UiStyles::apply_gradient_flags`] when a per-timestamp gradient
/// flag is off.
fn flatten_date_age(d: &mut DateAge) {
    d.now   = d.flat;
    d.today = d.flat;
    d.week  = d.flat;
    d.month = d.flat;
    d.year  = d.flat;
    d.old   = d.flat;
}


impl UiStyles {

    /// Sets a value on this set of colours using one of the keys understood
    /// by the `LS_COLORS` environment variable. Invalid keys set nothing, but
    /// return false.
    pub fn set_ls(&mut self, pair: &Pair<'_>) -> bool {
        match pair.key {
            "di" => self.filekinds.directory    = pair.to_style(),  // DIR
            "ex" => self.filekinds.executable   = pair.to_style(),  // EXEC
            "fi" => self.filekinds.normal       = pair.to_style(),  // FILE
            "pi" => self.filekinds.pipe         = pair.to_style(),  // FIFO
            "so" => self.filekinds.socket       = pair.to_style(),  // SOCK
            "bd" => self.filekinds.block_device = pair.to_style(),  // BLK
            "cd" => self.filekinds.char_device  = pair.to_style(),  // CHR
            "ln" => self.filekinds.symlink      = pair.to_style(),  // LINK
            "or" => self.broken_symlink         = pair.to_style(),  // ORPHAN
             _   => return false,
             // Codes we don’t do anything with:
             // MULTIHARDLINK, DOOR, SETUID, SETGID, CAPABILITY,
             // STICKY_OTHER_WRITABLE, OTHER_WRITABLE, STICKY, MISSING
        }
        true
    }

    /// Sets a value on this set of colours using one of the keys understood
    /// by the `LX_COLORS` environment variable. Invalid keys set nothing,
    /// but return false. This doesn’t take the `LS_COLORS` keys into account,
    /// so `set_ls` should have been run first.
    pub fn set_lx(&mut self, pair: &Pair<'_>) -> bool {
        match pair.key {
            "ur" => self.perms.user_read          = pair.to_style(),
            "uw" => self.perms.user_write         = pair.to_style(),
            "ux" => self.perms.user_execute_file  = pair.to_style(),
            "ue" => self.perms.user_execute_other = pair.to_style(),
            "gr" => self.perms.group_read         = pair.to_style(),
            "gw" => self.perms.group_write        = pair.to_style(),
            "gx" => self.perms.group_execute      = pair.to_style(),
            "tr" => self.perms.other_read         = pair.to_style(),
            "tw" => self.perms.other_write        = pair.to_style(),
            "tx" => self.perms.other_execute      = pair.to_style(),
            "su" => self.perms.special_user_file  = pair.to_style(),
            "sf" => self.perms.special_other      = pair.to_style(),
            "xa" => self.perms.attribute          = pair.to_style(),

            "sn" => self.set_number_style(pair.to_style()),
            "sb" => self.set_unit_style(pair.to_style()),
            "nb" => self.size.number_byte         = pair.to_style(),
            "nk" => self.size.number_kilo         = pair.to_style(),
            "nm" => self.size.number_mega         = pair.to_style(),
            "ng" => self.size.number_giga         = pair.to_style(),
            "nh" => self.size.number_huge         = pair.to_style(),
            "ub" => self.size.unit_byte           = pair.to_style(),
            "uk" => self.size.unit_kilo           = pair.to_style(),
            "um" => self.size.unit_mega           = pair.to_style(),
            "ug" => self.size.unit_giga           = pair.to_style(),
            "uh" => self.size.unit_huge           = pair.to_style(),
            "df" => self.size.major               = pair.to_style(),
            "ds" => self.size.minor               = pair.to_style(),

            "uu" => self.users.user_you           = pair.to_style(),
            "un" => self.users.user_someone_else  = pair.to_style(),
            "gu" => self.users.group_yours        = pair.to_style(),
            "gb" => self.users.group_member        = pair.to_style(),
            "gn" => self.users.group_not_yours    = pair.to_style(),
            // Capital U/G = the numeric ID version of the user/group
            // columns.  Case-sensitive, so these don't collide with the
            // lowercase `uu`/`un`/`gu`/`gn` keys above.
            "Uy" => self.users.uid_you            = pair.to_style(),
            "Un" => self.users.uid_someone_else   = pair.to_style(),
            "Gy" => self.users.gid_yours          = pair.to_style(),
            "Gb" => self.users.gid_member         = pair.to_style(),
            "Gn" => self.users.gid_not_yours      = pair.to_style(),

            "lc" => self.links.normal             = pair.to_style(),
            "lm" => self.links.multi_link_file    = pair.to_style(),

            "ga" => self.vcs.new                  = pair.to_style(),
            "gm" => self.vcs.modified             = pair.to_style(),
            "gd" => self.vcs.deleted              = pair.to_style(),
            "gv" => self.vcs.renamed              = pair.to_style(),
            "gt" => self.vcs.typechange           = pair.to_style(),

            "xx" => self.punctuation              = pair.to_style(),
            // The two-letter `LX_COLORS` codes for date are bulk
            // setters: each fans out to all four timestamp columns.
            // Per-column overrides are config-file only by design.
            "da" => { let s = pair.to_style(); self.date_for_each(|d| d.set_all(s)); }
            "dn" => { let s = pair.to_style(); self.date_for_each(|d| d.now   = s); }
            "dt" => { let s = pair.to_style(); self.date_for_each(|d| d.today = s); }
            "dw" => { let s = pair.to_style(); self.date_for_each(|d| d.week  = s); }
            "dm" => { let s = pair.to_style(); self.date_for_each(|d| d.month = s); }
            "dy" => { let s = pair.to_style(); self.date_for_each(|d| d.year  = s); }
            "do" => { let s = pair.to_style(); self.date_for_each(|d| d.old   = s); }
            "dl" => { let s = pair.to_style(); self.date_for_each(|d| d.flat  = s); }
            "in" => self.inode                    = pair.to_style(),
            "bl" => self.blocks                   = pair.to_style(),
            "hd" => self.header                   = pair.to_style(),
            "lp" => self.symlink_path             = pair.to_style(),
            "cc" => self.control_char             = pair.to_style(),
            "bO" => self.broken_path_overlay      = pair.to_style(),

             _   => return false,
        }

        true
    }

    pub fn set_number_style(&mut self, style: Style) {
        // Set all 5 number tiers AND `major`, so a theme that uses
        // the bulk setter to "fake flat" the size column also gets
        // the right colour when `--no-gradient` collapses the column
        // via `apply_gradient_flags`.  Mirrors the way DateAge::set_all
        // also sets `flat`.
        self.size.number_byte = style;
        self.size.number_kilo = style;
        self.size.number_mega = style;
        self.size.number_giga = style;
        self.size.number_huge = style;
        self.size.major       = style;
    }

    pub fn set_unit_style(&mut self, style: Style) {
        // Same logic as set_number_style: also set `minor` so
        // `--no-gradient` collapses to the bulk-set colour rather
        // than the inherited parent theme's value.
        self.size.unit_byte = style;
        self.size.unit_kilo = style;
        self.size.unit_mega = style;
        self.size.unit_giga = style;
        self.size.unit_huge = style;
        self.size.minor     = style;
    }

    /// Set a UI style from a human-readable config key and value.
    ///
    /// Returns `true` if the key was recognised.  The value is parsed
    /// via `parse_style()`, which accepts named colours, hex, X11
    /// names, modifiers, and raw ANSI codes.
    pub fn set_config(&mut self, key: &str, value: &str) -> bool {
        use super::lsc::parse_style;
        let style = parse_style(value);

        match key {
            // File kinds
            "normal"           => self.filekinds.normal       = style,
            "directory"        => self.filekinds.directory     = style,
            "symlink"          => self.filekinds.symlink       = style,
            "pipe"             => self.filekinds.pipe          = style,
            "block-device"     => self.filekinds.block_device  = style,
            "char-device"      => self.filekinds.char_device   = style,
            "socket"           => self.filekinds.socket        = style,
            "special"          => self.filekinds.special       = style,
            "executable"       => self.filekinds.executable    = style,

            // Permissions.  Both `permissions-*` (canonical, matches
            // the column name) and `perm-*` (legacy short form,
            // documented in lxconfig.toml(5)) are accepted.
            "permissions-user-read"     | "perm-user-read"     => self.perms.user_read          = style,
            "permissions-user-write"    | "perm-user-write"    => self.perms.user_write         = style,
            "permissions-user-execute"  | "perm-user-exec"     => self.perms.user_execute_file  = style,
            "permissions-user-execute-other" | "perm-user-exec-other" => self.perms.user_execute_other = style,
            "permissions-group-read"    | "perm-group-read"    => self.perms.group_read         = style,
            "permissions-group-write"   | "perm-group-write"   => self.perms.group_write        = style,
            "permissions-group-execute" | "perm-group-exec"    => self.perms.group_execute      = style,
            "permissions-other-read"    | "perm-other-read"    => self.perms.other_read         = style,
            "permissions-other-write"   | "perm-other-write"   => self.perms.other_write        = style,
            "permissions-other-execute" | "perm-other-exec"    => self.perms.other_execute      = style,
            "permissions-special-user"  | "perm-special-user"  => self.perms.special_user_file  = style,
            "permissions-special-other" | "perm-special-other" => self.perms.special_other      = style,
            "permissions-attribute"     | "perm-attribute"     => self.perms.attribute          = style,

            // Size (individual magnitudes)
            "size-number-byte" => self.size.number_byte = style,
            "size-number-kilo" => self.size.number_kilo = style,
            "size-number-mega" => self.size.number_mega = style,
            "size-number-giga" => self.size.number_giga = style,
            "size-number-huge" => self.size.number_huge = style,
            "size-unit-byte"   => self.size.unit_byte   = style,
            "size-unit-kilo"   => self.size.unit_kilo   = style,
            "size-unit-mega"   => self.size.unit_mega    = style,
            "size-unit-giga"   => self.size.unit_giga    = style,
            "size-unit-huge"   => self.size.unit_huge    = style,
            // Size (bulk setters)
            "size-number"      => self.set_number_style(style),
            "size-unit"        => self.set_unit_style(style),
            "size-major"       => self.size.major = style,
            "size-minor"       => self.size.minor = style,

            // Users
            "user-you"         => self.users.user_you          = style,
            "user-other"       => self.users.user_someone_else = style,
            "group-yours"      => self.users.group_yours       = style,
            "group-member"     => self.users.group_member      = style,
            "group-other"      => self.users.group_not_yours   = style,
            "uid-you"          => self.users.uid_you           = style,
            "uid-other"        => self.users.uid_someone_else  = style,
            "gid-yours"        => self.users.gid_yours         = style,
            "gid-member"       => self.users.gid_member        = style,
            "gid-other"        => self.users.gid_not_yours     = style,

            // Links
            "links"            => self.links.normal            = style,
            "links-multi"      => self.links.multi_link_file   = style,

            // VCS
            "vcs-new"          => self.vcs.new         = style,
            "vcs-modified"     => self.vcs.modified     = style,
            "vcs-deleted"      => self.vcs.deleted      = style,
            "vcs-renamed"      => self.vcs.renamed      = style,
            "vcs-typechange"   => self.vcs.typechange    = style,
            "vcs-ignored"      => self.vcs.ignored       = style,
            "vcs-conflicted"   => self.vcs.conflicted    = style,

            // UI elements
            "punctuation"      => self.punctuation      = style,
            // Bulk date setters fan out to all four timestamp columns.
            "date"             => self.date_for_each(|d| d.set_all(style)),
            "date-now"         => self.date_for_each(|d| d.now   = style),
            "date-today"       => self.date_for_each(|d| d.today = style),
            "date-week"        => self.date_for_each(|d| d.week  = style),
            "date-month"       => self.date_for_each(|d| d.month = style),
            "date-year"        => self.date_for_each(|d| d.year  = style),
            "date-old"         => self.date_for_each(|d| d.old   = style),
            "date-flat"        => self.date_for_each(|d| d.flat  = style),

            // Per-timestamp-column overrides.  These write directly
            // to the named field, so theme authors can give each
            // displayed timestamp column its own colour.  Order in
            // the theme block matters: write `date = ...` (bulk)
            // before per-column overrides, otherwise the bulk setter
            // will clobber them.
            "date-modified"        => self.date_modified.set_all(style),
            "date-modified-now"    => self.date_modified.now   = style,
            "date-modified-today"  => self.date_modified.today = style,
            "date-modified-week"   => self.date_modified.week  = style,
            "date-modified-month"  => self.date_modified.month = style,
            "date-modified-year"   => self.date_modified.year  = style,
            "date-modified-old"    => self.date_modified.old   = style,
            "date-modified-flat"   => self.date_modified.flat  = style,

            "date-accessed"        => self.date_accessed.set_all(style),
            "date-accessed-now"    => self.date_accessed.now   = style,
            "date-accessed-today"  => self.date_accessed.today = style,
            "date-accessed-week"   => self.date_accessed.week  = style,
            "date-accessed-month"  => self.date_accessed.month = style,
            "date-accessed-year"   => self.date_accessed.year  = style,
            "date-accessed-old"    => self.date_accessed.old   = style,
            "date-accessed-flat"   => self.date_accessed.flat  = style,

            "date-changed"         => self.date_changed.set_all(style),
            "date-changed-now"     => self.date_changed.now   = style,
            "date-changed-today"   => self.date_changed.today = style,
            "date-changed-week"    => self.date_changed.week  = style,
            "date-changed-month"   => self.date_changed.month = style,
            "date-changed-year"    => self.date_changed.year  = style,
            "date-changed-old"     => self.date_changed.old   = style,
            "date-changed-flat"    => self.date_changed.flat  = style,

            "date-created"         => self.date_created.set_all(style),
            "date-created-now"     => self.date_created.now   = style,
            "date-created-today"   => self.date_created.today = style,
            "date-created-week"    => self.date_created.week  = style,
            "date-created-month"   => self.date_created.month = style,
            "date-created-year"    => self.date_created.year  = style,
            "date-created-old"     => self.date_created.old   = style,
            "date-created-flat"    => self.date_created.flat  = style,
            "inode"            => self.inode             = style,
            "blocks"           => self.blocks            = style,
            "header"           => self.header            = style,
            "octal"            => self.octal             = style,
            "flags"            => self.flags             = style,
            "symlink-path"     => self.symlink_path      = style,
            "control-char"     => self.control_char      = style,
            "broken-symlink"   => self.broken_symlink    = style,
            "broken-overlay"   => self.broken_path_overlay = style,

            _ => return false,
        }

        true
    }
}
