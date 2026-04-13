//! Configuration: file discovery, loading, schema, resolution, and the
//! introspection / management commands (`--show-config`, `--dump-*`,
//! `--init-config`, `--upgrade-config`, `--save-as`).
//!
//! lx looks for a TOML config file in these locations (first found wins):
//!
//! 1. `$LX_CONFIG` — explicit path override
//! 2. `~/.lxconfig.toml` — simple home directory location
//! 3. `$XDG_CONFIG_HOME/lx/config.toml` (default `~/.config/lx/config.toml`)
//! 4. `~/Library/Application Support/lx/config.toml` (macOS only)
//!
//! ## Module layout
//!
//! - [`error`] — `ConfigError` and the `IoResultExt` helper trait.
//! - [`store`] — process-wide `OnceLock` storage; `init_config()` and
//!   `config()`.
//! - [`schema`] — on-disk types: `Config`, `PersonalityDef`, `ThemeDef`,
//!   `StyleDef`, `ConditionalOverride`, `StringOrList`, plus version
//!   constants.
//! - [`settings`] — the master `SETTING_FLAGS` table mapping config
//!   keys to CLI flags.
//! - [`load`] — config file discovery, TOML parsing, drop-in directory,
//!   `[[when]]` evaluation helpers.
//! - [`personality`] — `resolve_personality()`, compiled-in personality
//!   defaults, and the `--dump-personality` / `--save-as` output paths.
//! - [`themes`] — `BUILTIN_THEMES` registry and `--dump-theme` output.
//! - [`styles`] — compiled-in `exa` style, `resolve_style()`, and
//!   `--dump-style` output.
//! - [`classes`] — compiled-in file-type classes, `resolve_classes()`,
//!   and `--show-class` output.
//! - [`formats`] — compiled-in column formats, `resolve_formats()`, and
//!   `--show-format` output.
//! - [`init`] — `--init-config`: write the default config file.
//! - [`show`] — `--show-config`: human-readable overview.
//! - [`upgrade`] — `--upgrade-config`: schema version migration.

mod error;
mod store;
mod settings;
mod schema;
mod load;
mod personality;
mod themes;
mod styles;
mod classes;
mod formats;
mod init;
mod show;
mod upgrade;

// ── Public re-exports ───────────────────────────────────────────
//
// Only items reached from outside the config module are re-exported
// here.  Submodule-internal types (PersonalityDef, StringOrList,
// CONFIG_VERSION, BUILTIN_THEMES, etc.) stay accessible to other
// config submodules via `super::` but are not part of the
// crate-level API.

pub use self::error::ConfigError;

pub use self::store::{config, init_config};

pub(crate) use self::settings::{SETTING_FLAGS, SettingKind, find_setting};

pub use self::schema::{Config, StyleDef, ThemeDef};

pub use self::load::find_config_path;

pub use self::personality::{
    all_personality_names,
    dump_personality,
    dump_personality_all,
    resolve_personality,
    save_personality_as,
};

pub use self::themes::{all_theme_names, dump_theme, dump_theme_all, is_builtin_theme};

pub use self::styles::{all_style_names, compiled_exa_style, dump_style, dump_style_all, resolve_style};

pub use self::classes::{all_class_names, resolve_classes, show_class, show_class_all};

pub use self::formats::{show_format, show_format_all};

pub use self::init::{init_config_path, write_init_config};

pub use self::show::show_config;

pub use self::upgrade::upgrade_config;
