//! The setting → CLI flag mapping table.
//!
//! Personalities and `[[when]]` overrides store their values as a
//! flat key/value map.  At resolution time these are translated into
//! synthetic CLI args via `SETTING_FLAGS`, so the rest of the parser
//! can pretend they came from the command line.

use std::collections::HashMap;
use std::ffi::OsString;

use log::*;

/// How a TOML value maps to a CLI argument.
pub(crate) enum SettingKind {
    /// String value: `--flag=VALUE`
    Str,
    /// Boolean: `true` → `--flag`, `false` → omitted
    Bool,
    /// Integer: `--flag=N`
    Int,
}

/// Maps a config key name to its CLI flag and value kind.
pub(crate) struct SettingDef {
    pub key: &'static str,
    pub flag: &'static str,
    pub kind: SettingKind,
}

/// Master table of all config keys that map to CLI flags.
///
/// Adding a new flag to lx = adding one entry here.
#[rustfmt::skip]
pub(crate) static SETTING_FLAGS: &[SettingDef] = &[
    // display options
    SettingDef { key: "oneline",       flag: "--oneline",        kind: SettingKind::Bool },
    SettingDef { key: "long",          flag: "--long",           kind: SettingKind::Bool },
    SettingDef { key: "grid",          flag: "--grid",           kind: SettingKind::Bool },
    SettingDef { key: "across",        flag: "--across",         kind: SettingKind::Bool },
    SettingDef { key: "recurse",       flag: "--recurse",        kind: SettingKind::Bool },
    SettingDef { key: "tree",          flag: "--tree",           kind: SettingKind::Bool },
    SettingDef { key: "classify",      flag: "--classify",       kind: SettingKind::Str },
    SettingDef { key: "colour",        flag: "--colour",         kind: SettingKind::Str },
    SettingDef { key: "color",         flag: "--colour",         kind: SettingKind::Str },
    SettingDef { key: "icons",         flag: "--icons",          kind: SettingKind::Str },

    // filtering and sorting
    SettingDef { key: "all",           flag: "--all",            kind: SettingKind::Bool },
    SettingDef { key: "dot-entries",   flag: "--dot-entries",    kind: SettingKind::Bool },
    SettingDef { key: "list-dirs",     flag: "--list-dirs",      kind: SettingKind::Bool },
    SettingDef { key: "level",         flag: "--level",          kind: SettingKind::Int },
    SettingDef { key: "reverse",       flag: "--reverse",        kind: SettingKind::Bool },
    SettingDef { key: "sort",          flag: "--sort",           kind: SettingKind::Str },
    SettingDef { key: "group-dirs",    flag: "--group-dirs",     kind: SettingKind::Str },
    SettingDef { key: "only-dirs",     flag: "--only-dirs",      kind: SettingKind::Bool },
    SettingDef { key: "only-files",    flag: "--only-files",     kind: SettingKind::Bool },

    // long view options
    SettingDef { key: "size-style",    flag: "--size-style",     kind: SettingKind::Str },
    SettingDef { key: "binary",        flag: "--binary",         kind: SettingKind::Bool },
    SettingDef { key: "bytes",         flag: "--bytes",          kind: SettingKind::Bool },
    SettingDef { key: "header",        flag: "--header",         kind: SettingKind::Bool },
    SettingDef { key: "inode",         flag: "--inode",          kind: SettingKind::Bool },
    SettingDef { key: "links",         flag: "--links",          kind: SettingKind::Bool },
    SettingDef { key: "blocks",        flag: "--blocks",         kind: SettingKind::Bool },
    SettingDef { key: "group",         flag: "--group",          kind: SettingKind::Bool },
    SettingDef { key: "uid",           flag: "--uid",            kind: SettingKind::Bool },
    SettingDef { key: "gid",           flag: "--gid",            kind: SettingKind::Bool },
    SettingDef { key: "time-style",    flag: "--time-style",     kind: SettingKind::Str },
    SettingDef { key: "modified",      flag: "--modified",       kind: SettingKind::Bool },
    SettingDef { key: "changed",       flag: "--changed",        kind: SettingKind::Bool },
    SettingDef { key: "accessed",      flag: "--accessed",       kind: SettingKind::Bool },
    SettingDef { key: "created",       flag: "--created",        kind: SettingKind::Bool },
    SettingDef { key: "total",         flag: "--total",          kind: SettingKind::Bool },
    // `total-size` kept as a backward-compat alias for the
    // pre-rename canonical name; both produce the same flag.
    SettingDef { key: "total-size",    flag: "--total",          kind: SettingKind::Bool },
    SettingDef { key: "count",         flag: "--count",          kind: SettingKind::Bool },
    SettingDef { key: "extended",      flag: "--extended",       kind: SettingKind::Bool },
    SettingDef { key: "octal",         flag: "--octal",          kind: SettingKind::Bool },
    // `octal-permissions` kept as a backward-compat alias for the
    // pre-rename canonical name; both produce the same flag.
    SettingDef { key: "octal-permissions", flag: "--octal",      kind: SettingKind::Bool },

    SettingDef { key: "flags",         flag: "--flags",          kind: SettingKind::Bool },

    // filtering
    SettingDef { key: "ignore",        flag: "--ignore",         kind: SettingKind::Str },
    SettingDef { key: "prune",         flag: "--prune",          kind: SettingKind::Str },
    SettingDef { key: "symlinks",      flag: "--symlinks",       kind: SettingKind::Str },
    SettingDef { key: "classify",      flag: "--classify",       kind: SettingKind::Str },

    // VCS
    SettingDef { key: "vcs",           flag: "--vcs",            kind: SettingKind::Str },
    SettingDef { key: "vcs-status",    flag: "--vcs-status",     kind: SettingKind::Bool },
    SettingDef { key: "vcs-ignore",    flag: "--vcs-ignore",     kind: SettingKind::Bool },
    SettingDef { key: "vcs-repos",     flag: "--vcs-repos",      kind: SettingKind::Bool },

    // theme
    SettingDef { key: "theme",         flag: "--theme",           kind: SettingKind::Str },
    SettingDef { key: "gradient",      flag: "--gradient",        kind: SettingKind::Str },
    SettingDef { key: "smooth",        flag: "--smooth",          kind: SettingKind::Bool },

    // layout tuning (also settable via LX_GRID_ROWS / LX_ICON_SPACING)
    SettingDef { key: "grid-rows",     flag: "--grid-rows",       kind: SettingKind::Int },
    SettingDef { key: "icon-spacing",  flag: "--icon-spacing",    kind: SettingKind::Int },

    // numeric formatting (config-only, no short flags)
    SettingDef { key: "decimal-point",       flag: "--decimal-point",       kind: SettingKind::Str },
    SettingDef { key: "thousands-separator", flag: "--thousands-separator", kind: SettingKind::Str },

    // display
    SettingDef { key: "width",         flag: "--width",           kind: SettingKind::Int },
    SettingDef { key: "absolute",      flag: "--absolute",        kind: SettingKind::Bool },
    SettingDef { key: "hyperlink",     flag: "--hyperlink",       kind: SettingKind::Str },
    SettingDef { key: "quotes",        flag: "--quotes",          kind: SettingKind::Str },

    // explicit column enablers
    SettingDef { key: "permissions",   flag: "--permissions",    kind: SettingKind::Bool },
    SettingDef { key: "size",          flag: "--size",           kind: SettingKind::Bool },
    // `filesize` kept as a backward-compat alias for the
    // pre-rename canonical name; both produce the same flag.
    SettingDef { key: "filesize",      flag: "--size",           kind: SettingKind::Bool },
    SettingDef { key: "user",          flag: "--user",           kind: SettingKind::Bool },

    // column suppressors
    SettingDef { key: "no-permissions", flag: "--no-permissions", kind: SettingKind::Bool },
    SettingDef { key: "no-size",       flag: "--no-size",        kind: SettingKind::Bool },
    SettingDef { key: "no-filesize",   flag: "--no-size",        kind: SettingKind::Bool },
    SettingDef { key: "no-user",       flag: "--no-user",        kind: SettingKind::Bool },
    SettingDef { key: "no-time",       flag: "--no-time",        kind: SettingKind::Bool },
    SettingDef { key: "no-modified",   flag: "--no-modified",    kind: SettingKind::Bool },
    SettingDef { key: "no-changed",    flag: "--no-changed",     kind: SettingKind::Bool },
    SettingDef { key: "no-accessed",   flag: "--no-accessed",    kind: SettingKind::Bool },
    SettingDef { key: "no-created",    flag: "--no-created",     kind: SettingKind::Bool },
    SettingDef { key: "no-icons",      flag: "--no-icons",       kind: SettingKind::Bool },
    SettingDef { key: "no-inode",      flag: "--no-inode",       kind: SettingKind::Bool },
    SettingDef { key: "no-group",      flag: "--no-group",       kind: SettingKind::Bool },
    SettingDef { key: "no-uid",        flag: "--no-uid",         kind: SettingKind::Bool },
    SettingDef { key: "no-gid",        flag: "--no-gid",         kind: SettingKind::Bool },
    SettingDef { key: "no-links",      flag: "--no-links",       kind: SettingKind::Bool },
    SettingDef { key: "no-blocks",     flag: "--no-blocks",      kind: SettingKind::Bool },
    SettingDef { key: "no-extended",   flag: "--no-extended",    kind: SettingKind::Bool },
    SettingDef { key: "no-flags",      flag: "--no-flags",       kind: SettingKind::Bool },
    SettingDef { key: "no-octal",      flag: "--no-octal",       kind: SettingKind::Bool },
    SettingDef { key: "no-header",     flag: "--no-header",      kind: SettingKind::Bool },
    SettingDef { key: "no-count",      flag: "--no-count",       kind: SettingKind::Bool },
    SettingDef { key: "no-total",      flag: "--no-total",       kind: SettingKind::Bool },
    SettingDef { key: "no-total-size", flag: "--no-total",       kind: SettingKind::Bool },
    SettingDef { key: "no-vcs-status", flag: "--no-vcs-status",  kind: SettingKind::Bool },
    SettingDef { key: "no-vcs-repos",  flag: "--no-vcs-repos",   kind: SettingKind::Bool },
];

/// Look up a setting definition by config key name.
pub(crate) fn find_setting(key: &str) -> Option<&'static SettingDef> {
    SETTING_FLAGS.iter().find(|s| s.key == key)
}

/// Settings that used to exist but were removed in a later config
/// version.  Emits a targeted warning with a migration hint.
/// Returning `None` means "not a known removed setting".
fn removed_setting_hint(key: &str) -> Option<&'static str> {
    match key {
        "time" => Some(
            "the `time` setting was removed in config version 0.5; \
             use `modified`, `changed`, `accessed`, or `created` \
             (each a boolean) to add timestamp columns",
        ),
        "numeric" => Some(
            "the `numeric` setting was removed in config version 0.5; \
             UID and GID are now first-class columns. Use \
             `uid = true, gid = true, no-user = true, no-group = true` \
             for the old `-n` behaviour, or pick whichever columns you want",
        ),
        "colour-scale" | "color-scale" => Some(
            "the `colour-scale` setting was removed in config version 0.6; \
             use `gradient = \"all\"` (default) or \"none\"/\"size\"/\"date\". \
             Run `lx --upgrade-config` to migrate automatically",
        ),
        _ => None,
    }
}

/// Convert a settings map (from `[defaults]` or `[personality.*]`)
/// into synthetic CLI arguments.  Unknown keys are warned about.
pub(super) fn settings_to_args(
    settings: &HashMap<String, toml::Value>,
    context: &str,
) -> Vec<OsString> {
    let mut args = Vec::new();

    for (key, value) in settings {
        let Some(def) = find_setting(key) else {
            if let Some(hint) = removed_setting_hint(key) {
                eprintln!("lx: setting '{key}' in {context}: {hint}");
            } else {
                eprintln!("lx: unknown setting '{key}' in {context}");
            }
            continue;
        };

        match def.kind {
            SettingKind::Bool => {
                let truthy = match value {
                    toml::Value::Boolean(b) => *b,
                    toml::Value::String(s) => s == "true",
                    _ => {
                        warn!("Expected boolean for '{key}' in {context}; ignoring");
                        continue;
                    }
                };
                if truthy {
                    args.push(def.flag.into());
                } else {
                    // `key = false` means "suppress this, even if
                    // inherited".  Look up the corresponding `no-key`
                    // entry and emit its flag.  If no negation exists
                    // (e.g. `tree = false`), the false is a no-op —
                    // view-mode flags don't have suppressors.
                    let neg_key = format!("no-{key}");
                    if let Some(neg_def) = find_setting(&neg_key) {
                        args.push(neg_def.flag.into());
                    }
                }
            }
            SettingKind::Str => {
                let s = match value {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Array(arr) => {
                        let mut parts = Vec::new();
                        for item in arr {
                            if let toml::Value::String(s) = item {
                                parts.push(s.as_str().to_owned());
                            } else {
                                warn!(
                                    "Expected string in array for '{key}' in {context}; ignoring element"
                                );
                            }
                        }
                        if parts.is_empty() {
                            continue;
                        }
                        parts.join("|")
                    }
                    _ => {
                        warn!("Expected string or array for '{key}' in {context}; ignoring");
                        continue;
                    }
                };
                args.push(format!("{}={s}", def.flag).into());
            }
            SettingKind::Int => {
                let n = match value {
                    toml::Value::Integer(n) => *n,
                    toml::Value::String(s) => {
                        if let Ok(n) = s.parse::<i64>() {
                            n
                        } else {
                            warn!("Expected integer for '{key}' in {context}; ignoring");
                            continue;
                        }
                    }
                    _ => {
                        warn!("Expected integer for '{key}' in {context}; ignoring");
                        continue;
                    }
                };
                args.push(format!("{}={n}", def.flag).into());
            }
        }
    }

    args
}
