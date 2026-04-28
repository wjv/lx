use nu_ansi_term::Style;

use crate::fs::File;

mod ui_styles;
pub use self::ui_styles::{DateAge, UiStyles};

pub mod key_registry;

mod lsc;
pub use self::lsc::{LSColors, render_style_to_lx};

mod default_theme;

mod error;
pub use self::error::ThemeError;

mod oklab;
mod smooth;
pub use self::smooth::age_to_position;

#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub use_colours: UseColours,

    pub gradient: GradientFlags,

    pub definitions: Definitions,

    /// CLI override for theme selection (`--theme=NAME`).
    pub theme_override: Option<String>,
}

/// Per-column gradient on/off state.
///
/// Each gradient-capable column is either rendered with its full
/// per-tier gradient (`true`) or collapsed to a single flat colour
/// from the theme (`false`).  The four timestamp columns are
/// addressed individually, so users can write
/// `--gradient=modified` (gradient on the modified column only) or
/// `--gradient=size,modified` (size + modified, others flat).  The
/// bulk `--gradient=date` (and the hidden `timestamp` alias) flips
/// all four timestamp flags at once.
///
/// The collapse happens once at theme construction in `to_theme()`
/// via [`UiStyles::apply_gradient_flags`]; the renderers themselves
/// don't know about the on/off state — they just read whatever the
/// theme tells them.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[allow(clippy::struct_excessive_bools)] // one bool per gradient column is the natural shape
pub struct GradientFlags {
    pub size: bool,
    pub modified: bool,
    pub accessed: bool,
    pub changed: bool,
    pub created: bool,

    /// Whether to smooth the gradients into a 256-stop
    /// perceptually-uniform interpolation between the theme's
    /// per-tier anchors.  Opt-in via `--smooth`; has no effect on
    /// columns whose gradient flag is off, or on themes whose
    /// anchors aren't all 24-bit `Color::Rgb`.
    pub smooth: bool,
}

impl GradientFlags {
    /// All gradients on.  This is the default — themes that ship
    /// gradient values are designed to show them.
    pub const ALL: Self = Self {
        size: true,
        modified: true,
        accessed: true,
        changed: true,
        created: true,
        smooth: false,
    };

    /// All gradients off.  Each column collapses to its theme's
    /// flat colour (`size.major`/`size.minor` for size, the
    /// per-column `date_*.flat` for each timestamp).
    pub const NONE: Self = Self {
        size: false,
        modified: false,
        accessed: false,
        changed: false,
        created: false,
        smooth: false,
    };
}

impl Default for GradientFlags {
    fn default() -> Self {
        Self::ALL
    }
}

/// Under what circumstances we should display coloured, rather than plain,
/// output to the terminal.
///
/// By default, we want to display the colours when stdout can display them.
/// Turning them on when output is going to, say, a pipe, would make programs
/// such as `grep` or `more` not work properly. So the `Automatic` mode does
/// this check and only displays colours when they can be truly appreciated.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum UseColours {
    /// Display them even when output isn’t going to a terminal.
    Always,

    /// Display them when output is going to a terminal, but not otherwise.
    Automatic,

    /// Never display them, even when output is going to a terminal.
    Never,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct Definitions {
    pub ls: Option<String>,
}

pub struct Theme {
    pub ui: UiStyles,
    pub exts: Box<dyn FileColours>,
}

impl Theme {
    /// Minimal `Theme` for tests: default `UiStyles` (every field
    /// `Style::default()`) and a no-op `FileColours`.  Tests
    /// override the specific `theme.ui.*` fields they care about
    /// after construction.
    pub fn test_default() -> Self {
        Self {
            ui: UiStyles::default(),
            exts: Box::new(NoFileColours),
        }
    }
}

impl Options {
    #[allow(trivial_casts)] // the `as Box<_>` stuff below warns about this for some reason
    pub fn to_theme(&self, isatty: bool) -> Result<Theme, ThemeError> {
        // Validate the theme name early — even when colours are off,
        // the user should know if they've misspelled a theme name.
        if let Some(ref name) = self.theme_override {
            let empty_cfg = crate::config::Config::default();
            let cfg = crate::config::config().unwrap_or(&empty_cfg);
            Self::validate_theme_name(name, cfg)?;
        }

        if self.use_colours == UseColours::Never
            || (self.use_colours == UseColours::Automatic && !isatty)
        {
            let ui = UiStyles::plain();
            let exts = Box::new(NoFileColours);
            return Ok(Theme { ui, exts });
        }

        // Layer 1: base UI styles.
        // If a theme is selected, start from plain — the theme
        // provides colours (possibly via inherits = "exa").
        // Without a theme, use compiled-in defaults directly.
        let mut ui = if self.theme_override.is_some() {
            UiStyles::plain()
        } else {
            UiStyles::default_theme()
        };

        // Layer 2: LS_COLORS environment variable.
        let mut exts = self.definitions.parse_colour_vars(&mut ui);

        // Layer 4: theme from config or compiled-in personality.
        // The compiled-in "default" personality sets theme = "exa",
        // which applies both UiStyles::default_theme() and the
        // compiled-in exa style.  No magic fallback — the chain is
        // fully explicit: personality → theme → style.
        //
        // apply_config_theme handles the "exa" theme as a special
        // compiled-in case, so it works even without a config file.
        let empty_cfg = crate::config::Config::default();
        let cfg = crate::config::config().unwrap_or(&empty_cfg);
        self.apply_config_theme(cfg, &mut ui, &mut exts)?;

        // Layer 5: collapse gradient columns to flat colours where
        // the user has turned the gradient off.  Runs after the full
        // theme chain is resolved so it sees the final per-tier
        // values, regardless of whether they came from a compiled
        // theme, a config theme, an LS_COLORS override, or any
        // combination.
        ui.apply_gradient_flags(self.gradient);

        let exts: Box<dyn FileColours> = if exts.is_non_empty() {
            Box::new(exts)
        } else {
            Box::new(NoFileColours)
        };

        Ok(Theme { ui, exts })
    }

    /// Validate that a theme name (and its inheritance chain) can be
    /// resolved to a built-in theme or a `[theme.NAME]` config section.
    ///
    /// Returns `ThemeError::Unknown` for misspelled or missing names —
    /// the binary maps that to exit code 3 the same way an unknown
    /// `-p` personality does.  Cycles are accepted here (the chain is
    /// just truncated) because `apply_config_theme()` reports them
    /// with a much more useful chain context.
    fn validate_theme_name(name: &str, cfg: &crate::config::Config) -> Result<(), ThemeError> {
        let mut current = Some(name.to_string());
        let mut visited = Vec::new();

        while let Some(ref tname) = current {
            if visited.contains(tname) {
                return Ok(()); // cycle — apply_config_theme will report it
            }
            visited.push(tname.clone());

            if crate::config::is_builtin_theme(tname) {
                return Ok(()); // builtin, always valid
            } else if let Some(theme) = cfg.theme.get(tname) {
                current = theme.inherits.clone();
            } else {
                return Err(ThemeError::Unknown {
                    name: tname.clone(),
                });
            }
        }

        Ok(())
    }

    /// Apply the selected theme from the config file, resolving
    /// inheritance.
    ///
    /// Theme selection comes from `--theme=NAME` (set by CLI or
    /// personality synthetic args).  The inheritance chain is walked
    /// and themes are applied from root to leaf.  The special name
    /// `"exa"` applies the compiled-in default theme.
    fn apply_config_theme(
        &self,
        cfg: &crate::config::Config,
        ui: &mut UiStyles,
        exts: &mut ExtensionMappings,
    ) -> Result<(), ThemeError> {
        let Some(ref name) = self.theme_override else {
            return Ok(()); // no theme selected
        };

        // Build the inheritance chain: [leaf, ..., root].
        let mut chain: Vec<&crate::config::ThemeDef> = Vec::new();
        let mut visited: Vec<String> = Vec::new();
        let mut current = Some(name.clone());

        while let Some(ref tname) = current {
            if visited.contains(tname) {
                visited.push(tname.clone());
                let chain_str = visited.join(" \u{2192} ");
                return Err(ThemeError::Cycle { chain: chain_str });
            }
            visited.push(tname.clone());

            if tname == "exa" {
                // Special: apply the compiled-in default theme
                // and the compiled-in "exa" style.
                *ui = UiStyles::default_theme();
                let exa_style = crate::config::compiled_exa_style();
                Self::apply_style(&exa_style, cfg, exts);
                current = None;
            } else if tname == "lx-256" {
                // Compiled-in 256-colour theme.  Shares the "exa"
                // style for file class colours.
                *ui = UiStyles::lx_256_theme();
                let exa_style = crate::config::compiled_exa_style();
                Self::apply_style(&exa_style, cfg, exts);
                current = None;
            } else if tname == "lx-24bit" {
                // Compiled-in truecolour theme.  Shares the "exa"
                // style for file class colours.
                *ui = UiStyles::lx_24bit_theme();
                let exa_style = crate::config::compiled_exa_style();
                Self::apply_style(&exa_style, cfg, exts);
                current = None;
            } else if let Some(theme) = cfg.theme.get(tname) {
                chain.push(theme);
                current = theme.inherits.clone();
            } else {
                // Should not reach here — validate_theme_name catches
                // unknown names early — but be defensive in case the
                // chain reaches an inherits target that isn't defined.
                return Err(ThemeError::Unknown {
                    name: tname.clone(),
                });
            }
        }

        // Apply from root (last) to leaf (first).
        for theme in chain.into_iter().rev() {
            Self::apply_theme_def(theme, cfg, ui, exts);
        }

        Ok(())
    }

    /// Apply a single `ThemeDef`'s settings to the UI styles and
    /// extension mappings.
    fn apply_theme_def(
        theme: &crate::config::ThemeDef,
        cfg: &crate::config::Config,
        ui: &mut UiStyles,
        exts: &mut ExtensionMappings,
    ) {
        use log::*;

        // UI element overrides.  The underlying `theme.ui` is a
        // `HashMap<String, String>`, whose iteration order is
        // unspecified — so we sort by a specificity score before
        // applying.  This is load-bearing for the per-timestamp
        // column theme keys: the bulk `date = ...` setter (and the
        // bulk `date-now = ...`, etc.) must run *before* any
        // `date-modified = ...` or `date-modified-now = ...`
        // override, otherwise the bulk setter clobbers the specific
        // one.  See `theme_key_precedence` below for the buckets.
        let mut entries: Vec<(&String, &String)> = theme.ui.iter().collect();
        entries.sort_by_key(|(k, _)| theme_key_precedence(k));
        for (key, value) in entries {
            if !ui.set_config(key, value) {
                warn!("Unknown theme key '{key}'; ignoring");
            }
        }

        // Referenced style set.
        if let Some(ref style_name) = theme.use_style {
            if let Some(style) = crate::config::resolve_style(style_name) {
                Self::apply_style(&style, cfg, exts);
            } else {
                warn!("Style set '{style_name}' not found in config; ignoring");
            }
        }
    }

    /// Apply a style set to the extension mappings.
    ///
    /// Resolves class references (expanding each class's patterns)
    /// and applies file pattern entries directly.
    fn apply_style(
        style: &crate::config::StyleDef,
        _cfg: &crate::config::Config,
        exts: &mut ExtensionMappings,
    ) {
        use log::*;

        // Resolve class references.
        let classes = crate::config::resolve_classes();
        for (class_name, colour_str) in &style.classes {
            if let Some(patterns) = classes.get(class_name) {
                let colour = lsc::parse_style(colour_str);
                for pattern in patterns {
                    match glob::Pattern::new(pattern) {
                        Ok(pat) => exts.add(pat, colour),
                        Err(e) => warn!("Bad pattern '{pattern}' in class '{class_name}': {e}"),
                    }
                }
            } else {
                warn!("Unknown class '{class_name}' in style; ignoring");
            }
        }

        // Apply file patterns (quoted keys).
        for (key, value) in &style.patterns {
            match glob::Pattern::new(key) {
                Ok(pat) => exts.add(pat, lsc::parse_style(value)),
                Err(e) => warn!("Bad style glob '{key}': {e}"),
            }
        }
    }
}

/// Specificity bucket for a theme key.  [`Self::apply_theme_def`]
/// sorts theme entries by this value before applying them, so
/// more generic setters run before more specific ones and the
/// specific ones "win" — which is what a theme author writing
///
/// ```toml
/// date              = "blue"           # bulk
/// date-now          = "bright cyan"    # bulk per-tier
/// date-modified-now = "bright green"   # specific
/// ```
///
/// expects from the fall-through, regardless of the order in
/// which `HashMap` happened to hand the keys back.
///
/// Only the `date*` family currently needs ordering; everything
/// else lands in a single neutral bucket where order is
/// irrelevant (no two keys target the same field).
fn theme_key_precedence(key: &str) -> u8 {
    const COLUMNS: &[&str] = &["modified", "accessed", "changed", "created"];

    // Neutral bucket for all non-date keys.
    const NEUTRAL: u8 = 10;

    // `date` alone — bulk across every column, every tier.
    if key == "date" {
        return 0;
    }

    let Some(rest) = key.strip_prefix("date-") else {
        return NEUTRAL;
    };

    // `date-<col>` → bulk across every tier of one column.
    if COLUMNS.contains(&rest) {
        return 2;
    }

    // `date-<col>-<tier>` → most specific.
    for col in COLUMNS {
        if rest
            .strip_prefix(col)
            .and_then(|r| r.strip_prefix('-'))
            .is_some()
        {
            return 3;
        }
    }

    // Otherwise it's `date-<tier>` — bulk per-tier across every
    // column (e.g. `date-now`, `date-today`, ..., `date-flat`).
    1
}

#[cfg(test)]
mod theme_key_precedence_test {
    use super::theme_key_precedence;

    #[test]
    fn bulk_date_is_most_generic() {
        assert_eq!(theme_key_precedence("date"), 0);
    }

    #[test]
    fn bulk_per_tier_is_next() {
        assert_eq!(theme_key_precedence("date-now"), 1);
        assert_eq!(theme_key_precedence("date-today"), 1);
        assert_eq!(theme_key_precedence("date-flat"), 1);
    }

    #[test]
    fn bulk_per_column_overrides_bulk_per_tier() {
        assert_eq!(theme_key_precedence("date-modified"), 2);
        assert_eq!(theme_key_precedence("date-accessed"), 2);
    }

    #[test]
    fn per_column_per_tier_is_most_specific() {
        assert_eq!(theme_key_precedence("date-modified-now"), 3);
        assert_eq!(theme_key_precedence("date-accessed-flat"), 3);
        assert_eq!(theme_key_precedence("date-created-old"), 3);
    }

    #[test]
    fn non_date_keys_land_in_neutral_bucket() {
        assert_eq!(theme_key_precedence("directory"), 10);
        assert_eq!(theme_key_precedence("size-number-byte"), 10);
        assert_eq!(theme_key_precedence("permissions-user-read"), 10);
    }

    #[test]
    fn buckets_order_correctly() {
        // The whole point of the precedence function: sorting by it
        // must put generic first, specific last.
        assert!(theme_key_precedence("date") < theme_key_precedence("date-now"));
        assert!(theme_key_precedence("date-now") < theme_key_precedence("date-modified"));
        assert!(theme_key_precedence("date-modified") < theme_key_precedence("date-modified-now"));
    }
}

#[cfg(test)]
mod apply_theme_def_test {
    //! End-to-end tests for [`Options::apply_theme_def`], focused on
    //! the precedence fall-through across the `date*` key family.
    //!
    //! These tests go through the real `set_config` code path with a
    //! real `ThemeDef`, so they catch regressions where someone
    //! breaks the sort, introduces a typo in a `set_config` arm, or
    //! re-introduces the "bulk key clobbers per-column override"
    //! bug.

    use super::*;
    use crate::config::{Config, ThemeDef};
    use crate::theme::lsc::parse_style;
    use crate::theme::ui_styles::UiStyles;
    use std::collections::HashMap;

    /// Build a minimal `ThemeDef` from an iterator of `(key, value)`
    /// pairs.  Note that the order in which the pairs are given is
    /// **irrelevant** to what ends up in the resulting `HashMap`, so
    /// these tests implicitly assert that the fix doesn't depend on
    /// insertion order.
    fn theme_def_with<I, K, V>(entries: I) -> ThemeDef
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut ui = HashMap::new();
        for (k, v) in entries {
            ui.insert(k.into(), v.into());
        }
        ThemeDef {
            inherits: None,
            use_style: None,
            ui,
        }
    }

    fn apply(theme: &ThemeDef) -> UiStyles {
        let mut ui = UiStyles::default();
        let mut exts = ExtensionMappings::default();
        let cfg = Config::default();
        Options::apply_theme_def(theme, &cfg, &mut ui, &mut exts);
        ui
    }

    #[test]
    fn per_column_override_wins_over_bulk_date() {
        // The canonical precedence case: `date` sets every tier on
        // every column to blue, then `date-modified-now` overrides
        // just one cell.  Before the precedence-sort fix this was
        // nondeterministic — the bulk setter would sometimes run
        // last and clobber the per-column override.
        let theme = theme_def_with([("date", "blue"), ("date-modified-now", "bright green")]);
        let ui = apply(&theme);

        assert_eq!(ui.date_modified.now, parse_style("bright green"));
        // Every other slot on every column falls through to the
        // bulk setter.
        assert_eq!(ui.date_modified.today, parse_style("blue"));
        assert_eq!(ui.date_accessed.now, parse_style("blue"));
        assert_eq!(ui.date_changed.now, parse_style("blue"));
        assert_eq!(ui.date_created.now, parse_style("blue"));
    }

    #[test]
    fn full_precedence_chain_is_honoured() {
        // The "worked example" from the man page: generic → bulk
        // per-tier → bulk per-column → per-column per-tier, four
        // layers of specificity.  The most specific key must win
        // at every level, regardless of how `HashMap` hands the
        // keys back to `apply_theme_def`.
        let theme = theme_def_with([
            ("date", "blue"),                      // bucket 0
            ("date-now", "bright cyan"),           // bucket 1
            ("date-modified", "white"),            // bucket 2
            ("date-modified-now", "bright green"), // bucket 3
        ]);
        let ui = apply(&theme);

        // modified column:
        //   - `now` tier overridden at bucket 3 → bright green
        //   - every other tier overridden at bucket 2 → white
        assert_eq!(ui.date_modified.now, parse_style("bright green"));
        assert_eq!(ui.date_modified.today, parse_style("white"));
        assert_eq!(ui.date_modified.flat, parse_style("white"));

        // accessed/changed/created columns:
        //   - `now` tier overridden at bucket 1 → bright cyan
        //   - every other tier set at bucket 0 → blue
        assert_eq!(ui.date_accessed.now, parse_style("bright cyan"));
        assert_eq!(ui.date_accessed.today, parse_style("blue"));
        assert_eq!(ui.date_changed.now, parse_style("bright cyan"));
        assert_eq!(ui.date_changed.today, parse_style("blue"));
        assert_eq!(ui.date_created.now, parse_style("bright cyan"));
        assert_eq!(ui.date_created.today, parse_style("blue"));
    }

    #[test]
    fn each_column_can_be_themed_independently() {
        // The solarized-style design: each timestamp column gets
        // its own hue on the hottest tier.  Verifies that
        // per-column writes don't leak between columns.
        let theme = theme_def_with([
            ("date-modified-now", "cyan"),
            ("date-accessed-now", "green"),
            ("date-changed-now", "magenta"),
            ("date-created-now", "red"),
        ]);
        let ui = apply(&theme);

        assert_eq!(ui.date_modified.now, parse_style("cyan"));
        assert_eq!(ui.date_accessed.now, parse_style("green"));
        assert_eq!(ui.date_changed.now, parse_style("magenta"));
        assert_eq!(ui.date_created.now, parse_style("red"));
    }

    #[test]
    fn per_column_bulk_sets_every_tier_on_one_column() {
        // `date-modified = "red"` should set every tier (now,
        // today, week, month, year, old, flat) on the modified
        // column, leaving the other three untouched.
        let theme = theme_def_with([("date-modified", "red")]);
        let ui = apply(&theme);

        let red = parse_style("red");
        assert_eq!(ui.date_modified.now, red);
        assert_eq!(ui.date_modified.today, red);
        assert_eq!(ui.date_modified.week, red);
        assert_eq!(ui.date_modified.month, red);
        assert_eq!(ui.date_modified.year, red);
        assert_eq!(ui.date_modified.old, red);
        assert_eq!(ui.date_modified.flat, red);

        // Untouched columns keep the default (empty) style.
        assert_eq!(ui.date_accessed.now, Style::default());
        assert_eq!(ui.date_changed.now, Style::default());
        assert_eq!(ui.date_created.now, Style::default());
    }

    #[test]
    fn every_per_column_tier_key_parses_and_applies() {
        // Sanity check that all 32 per-column per-tier keys made it
        // into `set_config`.  If someone adds a new column or tier
        // and forgets to wire up a match arm, this catches it.
        let tiers = &["now", "today", "week", "month", "year", "old", "flat"];
        let columns = &["modified", "accessed", "changed", "created"];

        for &col in columns {
            for &tier in tiers {
                let key = format!("date-{col}-{tier}");
                let theme = theme_def_with([(key.clone(), "red")]);
                let ui = apply(&theme);

                let red = parse_style("red");
                let actual = match (col, tier) {
                    ("modified", "now") => ui.date_modified.now,
                    ("modified", "today") => ui.date_modified.today,
                    ("modified", "week") => ui.date_modified.week,
                    ("modified", "month") => ui.date_modified.month,
                    ("modified", "year") => ui.date_modified.year,
                    ("modified", "old") => ui.date_modified.old,
                    ("modified", "flat") => ui.date_modified.flat,
                    ("accessed", "now") => ui.date_accessed.now,
                    ("accessed", "today") => ui.date_accessed.today,
                    ("accessed", "week") => ui.date_accessed.week,
                    ("accessed", "month") => ui.date_accessed.month,
                    ("accessed", "year") => ui.date_accessed.year,
                    ("accessed", "old") => ui.date_accessed.old,
                    ("accessed", "flat") => ui.date_accessed.flat,
                    ("changed", "now") => ui.date_changed.now,
                    ("changed", "today") => ui.date_changed.today,
                    ("changed", "week") => ui.date_changed.week,
                    ("changed", "month") => ui.date_changed.month,
                    ("changed", "year") => ui.date_changed.year,
                    ("changed", "old") => ui.date_changed.old,
                    ("changed", "flat") => ui.date_changed.flat,
                    ("created", "now") => ui.date_created.now,
                    ("created", "today") => ui.date_created.today,
                    ("created", "week") => ui.date_created.week,
                    ("created", "month") => ui.date_created.month,
                    ("created", "year") => ui.date_created.year,
                    ("created", "old") => ui.date_created.old,
                    ("created", "flat") => ui.date_created.flat,
                    _ => unreachable!(),
                };
                assert_eq!(
                    actual, red,
                    "theme key {key:?} did not apply — missing set_config arm?"
                );
            }
        }
    }
}

impl Definitions {
    /// Parse `LS_COLORS` into a pair of outputs: recognised two-letter
    /// file-kind codes modify the mutable `UiStyles`, and any
    /// glob-style entries (e.g. `*.txt=31`) populate the returned
    /// `ExtensionMappings`.
    fn parse_colour_vars(&self, colours: &mut UiStyles) -> ExtensionMappings {
        use log::*;

        let mut exts = ExtensionMappings::default();

        if let Some(lsc) = &self.ls {
            LSColors(lsc).each_pair(|pair| {
                if !colours.set_ls(&pair) {
                    match glob::Pattern::new(pair.key) {
                        Ok(pat) => {
                            exts.add(pat, pair.to_style());
                        }
                        Err(e) => {
                            warn!("Couldn't parse glob pattern {:?}: {}", pair.key, e);
                        }
                    }
                }
            });
        }

        exts
    }
}

pub trait FileColours: std::marker::Sync {
    fn colour_file(&self, file: &File<'_>) -> Option<Style>;
}

#[derive(PartialEq, Debug)]
struct NoFileColours;
impl FileColours for NoFileColours {
    fn colour_file(&self, _file: &File<'_>) -> Option<Style> {
        None
    }
}

// When getting the colour of a file from a *pair* of colourisers, try the
// first one then try the second one. This lets the user provide their own
// file type associations, while falling back to the default set if not set
// explicitly.
impl<A, B> FileColours for (A, B)
where
    A: FileColours,
    B: FileColours,
{
    fn colour_file(&self, file: &File<'_>) -> Option<Style> {
        self.0
            .colour_file(file)
            .or_else(|| self.1.colour_file(file))
    }
}

#[derive(PartialEq, Debug, Default)]
struct ExtensionMappings {
    mappings: Vec<(glob::Pattern, Style)>,
}

// Loop through backwards so that colours specified later in the list override
// colours specified earlier, like we do with options and strict mode

impl FileColours for ExtensionMappings {
    fn colour_file(&self, file: &File<'_>) -> Option<Style> {
        self.mappings
            .iter()
            .rev()
            .find(|t| t.0.matches(&file.name))
            .map(|t| t.1)
    }
}

impl ExtensionMappings {
    fn is_non_empty(&self) -> bool {
        !self.mappings.is_empty()
    }

    fn add(&mut self, pattern: glob::Pattern, style: Style) {
        self.mappings.push((pattern, style));
    }
}

impl Theme {
    /// Style for a file-size number, given the raw byte count and
    /// the unit prefix chosen for display.  Smooth-gradient themes
    /// override the discrete tier lookup with a position-based
    /// LUT bucket.
    pub fn size_style(&self, bytes: u64, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::*;

        if let Some(lut) = self.ui.smooth_luts.size.as_deref() {
            let position = smooth::size_to_position(bytes);
            let bucket = (position * 255.0).round() as usize;
            return lut[bucket.min(255)];
        }

        match prefix {
            Some(Kilo | Kibi) => self.ui.size.number_kilo,
            Some(Mega | Mebi) => self.ui.size.number_mega,
            Some(Giga | Gibi) => self.ui.size.number_giga,
            Some(_) => self.ui.size.number_huge,
            None => self.ui.size.number_byte,
        }
    }

    /// Style for the unit suffix on a file size (`K`, `Mi`, ...).
    pub fn unit_style(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::*;

        match prefix {
            Some(Kilo | Kibi) => self.ui.size.unit_kilo,
            Some(Mega | Mebi) => self.ui.size.unit_mega,
            Some(Giga | Gibi) => self.ui.size.unit_giga,
            Some(_) => self.ui.size.unit_huge,
            None => self.ui.size.unit_byte,
        }
    }
}

impl Theme {
    /// Style to paint a file's name based on its type and any
    /// extension-based override from the active style.
    pub fn colour_file(&self, file: &File<'_>) -> Style {
        self.exts
            .colour_file(file)
            .unwrap_or(self.ui.filekinds.normal)
    }
}

/// Some of the styles are **overlays**: although they have the same attribute
/// set as regular styles (foreground and background colours, bold, underline,
/// etc), they’re intended to be used to *amend* existing styles.
///
/// For example, the target path of a broken symlink is displayed in a red,
/// underlined style by default. Paths can contain control characters, so
/// these control characters need to be underlined too, otherwise it looks
/// weird. So instead of having four separate configurable styles for “link
/// path”, “broken link path”, “control character” and “broken control
/// character”, there are styles for “link path”, “control character”, and
/// “broken link overlay”, the latter of which is just set to override the
/// underline attribute on the other two.
pub fn apply_overlay(mut base: Style, overlay: Style) -> Style {
    if let Some(fg) = overlay.foreground {
        base.foreground = Some(fg);
    }
    if let Some(bg) = overlay.background {
        base.background = Some(bg);
    }

    if overlay.is_bold {
        base.is_bold = true;
    }
    if overlay.is_dimmed {
        base.is_dimmed = true;
    }
    if overlay.is_italic {
        base.is_italic = true;
    }
    if overlay.is_underline {
        base.is_underline = true;
    }
    if overlay.is_blink {
        base.is_blink = true;
    }
    if overlay.is_reverse {
        base.is_reverse = true;
    }
    if overlay.is_hidden {
        base.is_hidden = true;
    }
    if overlay.is_strikethrough {
        base.is_strikethrough = true;
    }

    base
}
// TODO: move this function to the nu_ansi_term crate

#[cfg(test)]
mod customs_test {
    use super::*;
    use crate::theme::ui_styles::UiStyles;
    use nu_ansi_term::Color::*;

    macro_rules! test {
        ($name:ident: ls $ls:expr => colours $expected:ident -> $process_expected:expr) => {
            #[test]
            fn $name() {
                let mut $expected = UiStyles::default();
                $process_expected();

                let definitions = Definitions {
                    ls: Some($ls.into()),
                };

                let mut result = UiStyles::default();
                let _exts = definitions.parse_colour_vars(&mut result);
                assert_eq!($expected, result);
            }
        };
        ($name:ident: ls $ls:expr => exts $mappings:expr) => {
            #[test]
            fn $name() {
                let mappings: Vec<(glob::Pattern, Style)> = $mappings
                    .iter()
                    .map(|t| (glob::Pattern::new(t.0).unwrap(), t.1))
                    .collect();

                let definitions = Definitions {
                    ls: Some($ls.into()),
                };

                let result = definitions.parse_colour_vars(&mut UiStyles::default());
                assert_eq!(ExtensionMappings { mappings }, result);
            }
        };
    }

    // LS_COLORS can affect all of these colours:
    test!(ls_di: ls "di=31" => colours c -> { c.filekinds.directory    = Red.normal();    });
    test!(ls_ex: ls "ex=32" => colours c -> { c.filekinds.executable   = Green.normal();  });
    test!(ls_fi: ls "fi=33" => colours c -> { c.filekinds.normal       = Yellow.normal(); });
    test!(ls_pi: ls "pi=34" => colours c -> { c.filekinds.pipe         = Blue.normal();   });
    test!(ls_so: ls "so=35" => colours c -> { c.filekinds.socket       = Purple.normal(); });
    test!(ls_bd: ls "bd=36" => colours c -> { c.filekinds.block_device = Cyan.normal();   });
    test!(ls_cd: ls "cd=35" => colours c -> { c.filekinds.char_device  = Purple.normal(); });
    test!(ls_ln: ls "ln=34" => colours c -> { c.filekinds.symlink      = Blue.normal();   });
    test!(ls_or: ls "or=33" => colours c -> { c.broken_symlink         = Yellow.normal(); });

    // LS_COLORS treats anything it doesn't recognise as a filename
    // glob — the two-letter codes that used to be lx-specific (uu,
    // un, gu, gn, etc.) now fall through to the extensions map.
    test!(ls_uu: ls "uu=38;5;117" => exts [ ("uu", Fixed(117).normal()) ]);
    test!(ls_un: ls "un=38;5;118" => exts [ ("un", Fixed(118).normal()) ]);
    test!(ls_gu: ls "gu=38;5;119" => exts [ ("gu", Fixed(119).normal()) ]);
    test!(ls_gn: ls "gn=38;5;120" => exts [ ("gn", Fixed(120).normal()) ]);

    // Filename globs:
    test!(ls_txt: ls "*.txt=31"        => exts [ ("*.txt",    Red.normal())             ]);
    test!(ls_mp3: ls "*.mp3=38;5;135"  => exts [ ("*.mp3",    Fixed(135).normal())      ]);
    test!(ls_mak: ls "Makefile=1;32;4" => exts [ ("Makefile", Green.bold().underline()) ]);

    // Values get separated by colons:
    test!(ls_multi: ls "*.txt=31:*.rtf=32" => exts [
        ("*.txt", Red.normal()), ("*.rtf", Green.normal())
    ]);
    test!(ls_five: ls "1*1=31:2*2=32:3*3=1;33:4*4=34;1:5*5=35;4" => exts [
        ("1*1", Red.normal()), ("2*2", Green.normal()), ("3*3", Yellow.bold()),
        ("4*4", Blue.bold()), ("5*5", Purple.underline())
    ]);

    // Later colours override earlier ones (right-to-left):
    test!(ls_overwrite: ls "pi=31:pi=32:pi=33" => colours c -> {
        c.filekinds.pipe = Yellow.normal();
    });
}

#[cfg(test)]
#[cfg(unix)]
mod uid_gid_theme_test {
    use super::*;
    use crate::theme::ui_styles::UiStyles;
    use nu_ansi_term::Color::*;

    fn theme_with(ui: UiStyles) -> Theme {
        Theme {
            ui,
            exts: Box::new(NoFileColours),
        }
    }

    #[test]
    fn theme_exposes_uid_styles() {
        let mut ui = UiStyles::default_theme();
        ui.users.uid_you = Red.normal();
        ui.users.uid_someone_else = Green.normal();

        let theme = theme_with(ui);
        assert_eq!(theme.ui.users.uid_you, Red.normal());
        assert_eq!(theme.ui.users.uid_someone_else, Green.normal());
    }
}
