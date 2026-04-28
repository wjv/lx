use nu_ansi_term::{Color, Style};

use crate::theme::lsc::Pair;
use crate::theme::smooth::{self, SmoothLuts};

/// Returns `true` iff `style`'s foreground is a 24-bit
/// `Color::Rgb(...)` — i.e. exactly the case where Oklab
/// interpolation toward another RGB anchor would produce a
/// faithful result.  Palette colours (`Color::Fixed(n)`,
/// `Color::Cyan`, …) and unset foregrounds return `false`.
///
/// Used by [`Size::is_smoothable`] and [`DateAge::is_smoothable`]
/// to gate smooth-gradient LUT construction.
fn is_rgb_foreground(style: Style) -> bool {
    matches!(style.foreground, Some(Color::Rgb(_, _, _)))
}

#[derive(Debug, Default, PartialEq)]
#[rustfmt::skip]
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

    /// Precomputed smooth-gradient LUTs, populated by
    /// [`UiStyles::apply_gradient_flags`] when the caller sets
    /// `gradient.smooth = true` and the column's anchors are
    /// all 24-bit `Color::Rgb`.  The renderer reads these
    /// through a per-column accessor and falls back to the
    /// discrete per-tier fields on [`Size`] / [`DateAge`] when
    /// the LUT is absent.
    pub smooth_luts: SmoothLuts,
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
#[rustfmt::skip]
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

impl Size {
    /// True iff every `number_*` tier's foreground is a 24-bit
    /// `Color::Rgb(...)`.  This is the precondition for building
    /// a smooth-interpolated LUT for the size column: any palette
    /// or unset anchor disqualifies the whole column, because
    /// interpolating would produce RGB intermediate stops that
    /// would get quantised back to the palette by `nu_ansi_term`
    /// and land unpredictably on tier boundaries.
    ///
    /// The `unit_*` slots are intentionally excluded — the unit
    /// suffix is a small grace note next to the number and stays
    /// discrete regardless of whether smooth mode is on.
    pub(crate) fn is_smoothable(&self) -> bool {
        is_rgb_foreground(self.number_byte)
            && is_rgb_foreground(self.number_kilo)
            && is_rgb_foreground(self.number_mega)
            && is_rgb_foreground(self.number_giga)
            && is_rgb_foreground(self.number_huge)
    }
}

/// Age-based timestamp styles.  Six tiers from "just now" to "old",
/// plus a single `flat` colour used when the date column's gradient
/// is disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[rustfmt::skip]
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

    /// True iff every tier's foreground is a 24-bit
    /// `Color::Rgb(...)`.  Precondition for building a smooth-
    /// interpolated LUT for this timestamp column; see
    /// `Size::is_smoothable` for the rationale.
    ///
    /// The `flat` slot is intentionally excluded — it's the
    /// fallback for when the column's gradient is off, and its
    /// colour has no effect on smoothing.
    pub(crate) fn is_smoothable(&self) -> bool {
        is_rgb_foreground(self.now)
            && is_rgb_foreground(self.today)
            && is_rgb_foreground(self.week)
            && is_rgb_foreground(self.month)
            && is_rgb_foreground(self.year)
            && is_rgb_foreground(self.old)
    }

    /// Pick the style for a given age in seconds.
    #[rustfmt::skip]
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
    /// etc. setters so that theme authors who write a single `date`
    /// block see it applied to every timestamp column.
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
    ///   tier with `size.minor`.  These two slots (`size-major` and
    ///   `size-minor`) are already themeable and serve as the
    ///   column's "headline" colour in non-tiered contexts like
    ///   the `-CZ` count footer.
    /// - For each timestamp column whose flag is `false`, every age
    ///   tier (`now` through `old`) is overwritten with that column's
    ///   `flat`.  Theme authors set `date-flat` explicitly (or rely
    ///   on the bulk `date = "..."` setter, which also touches
    ///   `flat`).
    ///
    /// Runs once at theme construction so the renderers themselves
    /// stay oblivious to the on/off state.
    ///
    /// When `gradient.smooth` is set, this also builds 256-stop
    /// smooth-interpolated LUTs for each gradient-capable column
    /// whose theme anchors are all 24-bit `Color::Rgb`.  The LUT
    /// build runs **before** flattening, so columns with gradients
    /// off don't get a (pointless) LUT of their flat colour, and
    /// columns with gradients on but non-RGB anchors are silently
    /// left in discrete mode.
    pub fn apply_gradient_flags(&mut self, gradient: super::GradientFlags) {
        // Phase 1: build smooth LUTs from the original per-tier
        // anchor colours, before they're flattened.
        if gradient.smooth {
            if gradient.size && self.size.is_smoothable() {
                self.smooth_luts.size =
                    Some(smooth::build_smooth_lut(&smooth::size_anchors(&self.size)));
            }
            if gradient.modified && self.date_modified.is_smoothable() {
                self.smooth_luts.modified = Some(smooth::build_smooth_lut(&smooth::date_anchors(
                    &self.date_modified,
                )));
            }
            if gradient.accessed && self.date_accessed.is_smoothable() {
                self.smooth_luts.accessed = Some(smooth::build_smooth_lut(&smooth::date_anchors(
                    &self.date_accessed,
                )));
            }
            if gradient.changed && self.date_changed.is_smoothable() {
                self.smooth_luts.changed = Some(smooth::build_smooth_lut(&smooth::date_anchors(
                    &self.date_changed,
                )));
            }
            if gradient.created && self.date_created.is_smoothable() {
                self.smooth_luts.created = Some(smooth::build_smooth_lut(&smooth::date_anchors(
                    &self.date_created,
                )));
            }
        }

        // Phase 2: flatten columns whose gradient is off.
        if !gradient.size {
            self.size.number_byte = self.size.major;
            self.size.number_kilo = self.size.major;
            self.size.number_mega = self.size.major;
            self.size.number_giga = self.size.major;
            self.size.number_huge = self.size.major;
            self.size.unit_byte = self.size.minor;
            self.size.unit_kilo = self.size.minor;
            self.size.unit_mega = self.size.minor;
            self.size.unit_giga = self.size.minor;
            self.size.unit_huge = self.size.minor;
        }
        if !gradient.modified {
            flatten_date_age(&mut self.date_modified);
        }
        if !gradient.accessed {
            flatten_date_age(&mut self.date_accessed);
        }
        if !gradient.changed {
            flatten_date_age(&mut self.date_changed);
        }
        if !gradient.created {
            flatten_date_age(&mut self.date_created);
        }
    }
}

/// Collapse a single [`DateAge`] to its `flat` colour: every age
/// tier (`now` through `old`) becomes `flat`.  Used by
/// [`UiStyles::apply_gradient_flags`] when a per-timestamp gradient
/// flag is off.
fn flatten_date_age(d: &mut DateAge) {
    d.now = d.flat;
    d.today = d.flat;
    d.week = d.flat;
    d.month = d.flat;
    d.year = d.flat;
    d.old = d.flat;
}

impl UiStyles {
    /// Sets a value on this set of colours using one of the keys understood
    /// by the `LS_COLORS` environment variable. Invalid keys set nothing, but
    /// return false.
    pub fn set_ls(&mut self, pair: &Pair<'_>) -> bool {
        match pair.key {
            "di" => self.filekinds.directory = pair.to_style(), // DIR
            "ex" => self.filekinds.executable = pair.to_style(), // EXEC
            "fi" => self.filekinds.normal = pair.to_style(),    // FILE
            "pi" => self.filekinds.pipe = pair.to_style(),      // FIFO
            "so" => self.filekinds.socket = pair.to_style(),    // SOCK
            "bd" => self.filekinds.block_device = pair.to_style(), // BLK
            "cd" => self.filekinds.char_device = pair.to_style(), // CHR
            "ln" => self.filekinds.symlink = pair.to_style(),   // LINK
            "or" => self.broken_symlink = pair.to_style(),      // ORPHAN
            _ => return false,
            // Codes we don’t do anything with:
            // MULTIHARDLINK, DOOR, SETUID, SETGID, CAPABILITY,
            // STICKY_OTHER_WRITABLE, OTHER_WRITABLE, STICKY, MISSING
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
        self.size.major = style;
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
        self.size.minor = style;
    }

    /// Set a UI style from a human-readable config key and value.
    ///
    /// Returns `true` if the key was recognised.  The value is parsed
    /// via `parse_style()`, which accepts named colours, hex, X11
    /// names, modifiers, and raw ANSI codes.
    ///
    /// Dispatch goes through the theme key registry
    /// (`super::key_registry`); each recognised key has one entry
    /// that knows whether it writes a single field or fans out via
    /// a bulk setter.
    pub fn set_config(&mut self, key: &str, value: &str) -> bool {
        use super::key_registry::{StyleAccess, ThemeKeyDef};
        use super::lsc::parse_style;

        let Some(def) = ThemeKeyDef::from_name(key) else {
            return false;
        };
        let style = parse_style(value);
        match def.access {
            StyleAccess::Direct { set, .. } | StyleAccess::Bulk { set } => set(self, style),
        }
        true
    }
}

#[cfg(test)]
mod is_smoothable_test {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Style {
        Style::from(Color::Rgb(r, g, b))
    }

    fn fixed(n: u8) -> Style {
        Style::from(Color::Fixed(n))
    }

    #[test]
    fn size_all_rgb_is_smoothable() {
        let size = Size {
            number_byte: rgb(0x10, 0x10, 0x10),
            number_kilo: rgb(0x20, 0x20, 0x20),
            number_mega: rgb(0x40, 0x40, 0x40),
            number_giga: rgb(0x80, 0x80, 0x80),
            number_huge: rgb(0xC0, 0xC0, 0xC0),
            ..Size::default()
        };
        assert!(size.is_smoothable());
    }

    #[test]
    fn size_default_is_not_smoothable() {
        // Default styles have no foreground set.
        assert!(!Size::default().is_smoothable());
    }

    #[test]
    fn size_one_palette_anchor_disqualifies_the_column() {
        let size = Size {
            number_byte: rgb(0x10, 0x10, 0x10),
            number_kilo: rgb(0x20, 0x20, 0x20),
            number_mega: fixed(196), // palette colour
            number_giga: rgb(0x80, 0x80, 0x80),
            number_huge: rgb(0xC0, 0xC0, 0xC0),
            ..Size::default()
        };
        assert!(!size.is_smoothable());
    }

    #[test]
    fn size_one_unset_anchor_disqualifies_the_column() {
        let size = Size {
            number_byte: rgb(0x10, 0x10, 0x10),
            number_kilo: rgb(0x20, 0x20, 0x20),
            number_mega: Style::default(), // foreground is None
            number_giga: rgb(0x80, 0x80, 0x80),
            number_huge: rgb(0xC0, 0xC0, 0xC0),
            ..Size::default()
        };
        assert!(!size.is_smoothable());
    }

    #[test]
    fn size_ignores_unit_slots() {
        // unit_* fields stay discrete by design; having palette
        // colours there must not disable smoothing.
        let size = Size {
            number_byte: rgb(0x10, 0x10, 0x10),
            number_kilo: rgb(0x20, 0x20, 0x20),
            number_mega: rgb(0x40, 0x40, 0x40),
            number_giga: rgb(0x80, 0x80, 0x80),
            number_huge: rgb(0xC0, 0xC0, 0xC0),
            unit_byte: fixed(244),
            unit_kilo: fixed(244),
            unit_mega: fixed(244),
            unit_giga: fixed(244),
            unit_huge: fixed(244),
            ..Size::default()
        };
        assert!(size.is_smoothable());
    }

    #[test]
    fn date_all_rgb_is_smoothable() {
        let date = DateAge {
            now: rgb(0x3D, 0xD7, 0xD7),
            today: rgb(0x3D, 0xD7, 0xD7),
            week: rgb(0x3A, 0xAB, 0xAE),
            month: rgb(0x3B, 0x8E, 0xD8),
            year: rgb(0x88, 0x88, 0x88),
            old: rgb(0x5C, 0x5C, 0x5C),
            flat: Style::default(),
        };
        assert!(date.is_smoothable());
    }

    #[test]
    fn date_default_is_not_smoothable() {
        assert!(!DateAge::default().is_smoothable());
    }

    #[test]
    fn date_one_palette_tier_disqualifies_the_column() {
        let date = DateAge {
            now: rgb(0x3D, 0xD7, 0xD7),
            today: rgb(0x3D, 0xD7, 0xD7),
            week: fixed(30), // palette
            month: rgb(0x3B, 0x8E, 0xD8),
            year: rgb(0x88, 0x88, 0x88),
            old: rgb(0x5C, 0x5C, 0x5C),
            flat: Style::default(),
        };
        assert!(!date.is_smoothable());
    }

    #[test]
    fn date_one_unset_tier_disqualifies_the_column() {
        let date = DateAge {
            now: rgb(0x3D, 0xD7, 0xD7),
            today: Style::default(),
            week: rgb(0x3A, 0xAB, 0xAE),
            month: rgb(0x3B, 0x8E, 0xD8),
            year: rgb(0x88, 0x88, 0x88),
            old: rgb(0x5C, 0x5C, 0x5C),
            flat: Style::default(),
        };
        assert!(!date.is_smoothable());
    }

    #[test]
    fn date_ignores_flat_slot() {
        // `flat` is the no-gradient fallback; its value must not
        // affect whether the tier chain is smoothable.
        let date = DateAge {
            now: rgb(0x3D, 0xD7, 0xD7),
            today: rgb(0x3D, 0xD7, 0xD7),
            week: rgb(0x3A, 0xAB, 0xAE),
            month: rgb(0x3B, 0x8E, 0xD8),
            year: rgb(0x88, 0x88, 0x88),
            old: rgb(0x5C, 0x5C, 0x5C),
            flat: fixed(244), // palette, but doesn't matter
        };
        assert!(date.is_smoothable());
    }
}
