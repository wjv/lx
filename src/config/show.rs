//! `--show-config`: human-readable overview of the active configuration.
//!
//! This is the only output path that pulls from *every* part of the
//! config layer (personality, theme, style, classes, formats), so it
//! lives in its own file rather than alongside any one of them.

use nu_ansi_term::{Color, Style};

use super::classes::resolve_classes;
use super::load::find_config_path;
use super::personality::{PersonalitySource, resolve_personality_full};
use super::schema::CONFIG_VERSION;
use super::store::config;
use super::styles::resolve_style;
use super::themes::is_builtin_theme;

/// Shared colour styles for `--show-config` output.
struct Styles {
    heading: Style,
    label: Style,
    name: Style,
    value: Style,
    dimmed: Style,
}

impl Styles {
    fn new() -> Self {
        Self {
            heading: Style::new().bold().fg(Color::Yellow),
            label: Style::new().bold(),
            name: Style::new().bold().fg(Color::Cyan),
            value: Style::new().fg(Color::Green),
            dimmed: Style::new().dimmed(),
        }
    }
}

/// Display the active configuration to stdout.
///
/// Shows the resolved personality, format, theme, style, and classes,
/// indicating for each whether it's compiled-in or from the config file.
pub fn show_config(
    mode: crate::options::ShowConfigMode,
    personality_name: &str,
    activated_by: &str,
    cli_theme_override: Option<&str>,
    implicit_format: Option<&str>,
) {
    use crate::options::ShowConfigMode::{Active, Available, Full};

    let s = Styles::new();
    let config_path = find_config_path();
    let cfg = config();

    println!("{}", s.heading.paint("lx configuration"));
    println!();

    // Active half: where config comes from + what's currently
    // running.
    if matches!(mode, Active | Full) {
        show_config_file(&s, config_path.as_ref(), cfg);
        let theme_name =
            show_personality(&s, personality_name, activated_by, cli_theme_override, cfg);
        show_format(&s, personality_name, implicit_format);
        let style_name = show_theme(&s, theme_name.as_deref(), cfg);
        show_style(&s, style_name.as_deref(), cfg);
    }

    // Divider only when both halves are present.
    if matches!(mode, Full) {
        show_divider(&s);
    }

    // Available half: catalogue of every defined personality,
    // format, theme, style, and class.
    if matches!(mode, Available | Full) {
        show_available_personalities(&s, cfg);
        show_formats(&s, cfg);
        show_available_themes(&s, cfg);
        show_available_styles(&s, cfg);
        show_classes(&s, cfg);
    }
}

/// Dimmed horizontal rule that separates the Active half (top)
/// from the Available catalogue (bottom).
fn show_divider(s: &Styles) {
    println!("{}", s.dimmed.paint("─".repeat(64)));
    println!();
}

// ── Config file section ────────────────────────────────────────

fn show_config_file(
    s: &Styles,
    config_path: Option<&std::path::PathBuf>,
    cfg: Option<&super::schema::Config>,
) {
    match config_path {
        Some(p) => println!(
            "{} {}",
            s.label.paint("Config file:"),
            s.value.paint(p.display().to_string())
        ),
        None => println!(
            "{} {}",
            s.label.paint("Config file:"),
            s.dimmed.paint("(none)")
        ),
    }
    println!(
        "{} {}",
        s.label.paint("Config version:"),
        s.value.paint(CONFIG_VERSION)
    );
    if let Some(cfg) = cfg
        && !cfg.drop_in_paths.is_empty()
    {
        println!(
            "{} {} file(s) from conf.d/",
            s.label.paint("Drop-ins:"),
            s.value.paint(cfg.drop_in_paths.len().to_string())
        );
        for p in &cfg.drop_in_paths {
            println!("  {}", s.dimmed.paint(p.display().to_string()));
        }
    }
    println!();
}

// ── Personality section ────────────────────────────────────────

/// Show personality details.  Returns the resolved theme name (if any)
/// for downstream sections.
fn show_personality(
    s: &Styles,
    personality_name: &str,
    activated_by: &str,
    cli_theme_override: Option<&str>,
    _cfg: Option<&super::schema::Config>,
) -> Option<String> {
    println!(
        "{} {}",
        s.label.paint("Personality:"),
        s.heading.paint(personality_name)
    );
    println!(
        "  {} {}",
        s.label.paint("activated by:"),
        s.dimmed.paint(activated_by)
    );

    let Ok(Some(resolved)) = resolve_personality_full(personality_name) else {
        println!();
        return cli_theme_override.map(String::from);
    };

    // Inheritance chain: leaf to root.
    if !resolved.chain.is_empty() {
        println!("  {}", s.label.paint("inheritance:"));
        for link in &resolved.chain {
            let source_str = match link.source {
                PersonalitySource::Builtin => "builtin",
                PersonalitySource::Config => "config",
                PersonalitySource::ConfigOverridesBuiltin => "config, overrides builtin",
            };

            println!(
                "    {} {}",
                s.heading.paint(format!("\u{25B8} {}", link.name)),
                s.dimmed.paint(format!("({source_str})")),
            );

            // Direct contributions of *this* link (before the
            // merge): format, columns, settings declared in
            // [personality.NAME] itself.  Parents in the chain
            // contribute these too; without them the inheritance
            // display would only show [[when]] blocks, which is
            // misleading.
            if let Some(ref fmt) = link.def.format {
                println!("      {} {}", s.label.paint("format:"), s.name.paint(fmt));
            }
            if let Some(ref cols) = link.def.columns {
                println!(
                    "      {} {}",
                    s.label.paint("columns:"),
                    s.value.paint(cols.to_csv()),
                );
            }
            if !link.def.settings.is_empty() {
                println!("      {}", s.label.paint("settings:"));
                let mut keys: Vec<_> = link.def.settings.keys().collect();
                keys.sort();
                for key in keys {
                    println!(
                        "        {} = {}",
                        s.name.paint(key),
                        s.value.paint(link.def.settings[key].to_string()),
                    );
                }
            }

            // Per-block [[when]] detail: each block lists its
            // conditions (with per-condition match status), the
            // overall match result, and the settings it would
            // apply.  Replaces the bare "N blocks, M active"
            // count: useful as a count, useless for diagnosis.
            for (i, block) in link.def.when.iter().enumerate() {
                let outcomes = block.explain();
                let block_matched = outcomes.iter().all(|o| o.matched);
                // Unmatched blocks render entirely dimmed: the
                // tag, the equals signs, the keys and values.
                // Matched blocks keep the live label/key/value
                // colouring.  "noted, not in effect" should be
                // visually quiet across the whole block.
                let tag_style = if block_matched { s.label } else { s.dimmed };
                let status = if block_matched {
                    s.value.paint("matched")
                } else {
                    s.dimmed.paint("not matched")
                };
                println!(
                    "      {} {} {}",
                    tag_style.paint(format!("[[when]] #{}", i + 1)),
                    s.dimmed.paint("→"),
                    status,
                );
                for outcome in &outcomes {
                    let mark = if outcome.matched { "✓" } else { "✗" };
                    println!(
                        "        {} {}",
                        s.dimmed.paint(mark),
                        s.dimmed.paint(&outcome.description),
                    );
                }
                if !block.settings.is_empty() {
                    let (key_style, eq_style, val_style) = if block_matched {
                        (s.name, Style::default(), s.value)
                    } else {
                        (s.dimmed, s.dimmed, s.dimmed)
                    };
                    let mut keys: Vec<_> = block.settings.keys().collect();
                    keys.sort();
                    for key in keys {
                        let val = &block.settings[key];
                        println!(
                            "        {} {} {} {}",
                            s.dimmed.paint("•"),
                            key_style.paint(key),
                            eq_style.paint("="),
                            val_style.paint(val.to_string()),
                        );
                    }
                }
            }

            // Shadowed-builtin diff: when a user config personality
            // shares a name with a compiled-in one, the user's
            // [personality.NAME] block fully replaces the builtin's
            // — including any [[when]] blocks and settings the
            // builtin had.  Surface what's in the builtin but not
            // in the user's override so the silent-shadowing case
            // (a 0.10 release adding new defaults to a personality
            // the user has overridden) is visible.
            //
            // Conservative semantic: list every key in the builtin
            // that doesn't appear in the user's override (regardless
            // of value); list every [[when]] block in the builtin
            // (the user's `when` array fully replaces the builtin's
            // — there's no partial merge at the [[when]] level).
            // Format/columns get the same key-presence treatment.
            if let Some(ref builtin) = link.shadowed_builtin {
                let shadow_format = if link.def.format.is_none() {
                    builtin.format.clone()
                } else {
                    None
                };
                let shadow_columns = if link.def.columns.is_none() {
                    builtin.columns.clone()
                } else {
                    None
                };
                let mut shadowed_settings: Vec<&String> = builtin
                    .settings
                    .keys()
                    .filter(|k| !link.def.settings.contains_key(*k))
                    .collect();
                shadowed_settings.sort();

                let has_anything = shadow_format.is_some()
                    || shadow_columns.is_some()
                    || !shadowed_settings.is_empty()
                    || !builtin.when.is_empty();

                if has_anything {
                    println!(
                        "      {}",
                        s.dimmed
                            .paint("in the builtin but shadowed by user configuration:"),
                    );
                    if let Some(fmt) = shadow_format {
                        println!(
                            "        {} {} {} {}",
                            s.dimmed.paint("•"),
                            s.dimmed.paint("format"),
                            s.dimmed.paint("="),
                            s.dimmed.paint(format!("\"{fmt}\"")),
                        );
                    }
                    if let Some(cols) = shadow_columns {
                        println!(
                            "        {} {} {} {}",
                            s.dimmed.paint("•"),
                            s.dimmed.paint("columns"),
                            s.dimmed.paint("="),
                            s.dimmed.paint(cols.to_csv()),
                        );
                    }
                    for key in shadowed_settings {
                        let val = &builtin.settings[key];
                        println!(
                            "        {} {} {} {}",
                            s.dimmed.paint("•"),
                            s.dimmed.paint(key),
                            s.dimmed.paint("="),
                            s.dimmed.paint(val.to_string()),
                        );
                    }
                    for (i, block) in builtin.when.iter().enumerate() {
                        let outcomes = block.explain();
                        println!("        {}", s.dimmed.paint(format!("[[when]] #{}", i + 1)));
                        for outcome in &outcomes {
                            let mark = if outcome.matched { "✓" } else { "✗" };
                            println!(
                                "          {} {}",
                                s.dimmed.paint(mark),
                                s.dimmed.paint(&outcome.description),
                            );
                        }
                        let mut keys: Vec<_> = block.settings.keys().collect();
                        keys.sort();
                        for key in keys {
                            let val = &block.settings[key];
                            println!(
                                "          {} {} {} {}",
                                s.dimmed.paint("•"),
                                s.dimmed.paint(key),
                                s.dimmed.paint("="),
                                s.dimmed.paint(val.to_string()),
                            );
                        }
                    }
                }
            }
        }
    }

    let p = &resolved.def;
    // Effective view: what the chain produces after merging
    // (parents → child overrides) and applying any matched
    // [[when]] blocks.  Each row above contributes; these rows
    // are the result.  `effective format:` is the
    // *chain-declared* format only — the implicit `-l` tier and
    // column resolution live in their own top-level Format
    // section (see `show_format`).
    if let Some(ref fmt) = p.format {
        println!(
            "  {} {}",
            s.label.paint("effective format:"),
            s.name.paint(fmt),
        );
    }
    if let Some(ref cols) = p.columns {
        println!(
            "  {} {}",
            s.label.paint("effective columns:"),
            s.value.paint(cols.to_csv()),
        );
    }
    if !p.settings.is_empty() {
        println!("  {}", s.label.paint("effective settings:"));
        let mut keys: Vec<_> = p.settings.keys().collect();
        keys.sort();
        for key in keys {
            println!(
                "    {} = {}",
                s.name.paint(key),
                s.value.paint(p.settings[key].to_string())
            );
        }
    }
    println!();

    // Resolve the theme name for downstream sections.
    cli_theme_override.map(String::from).or_else(|| {
        p.settings.get("theme").and_then(|v| {
            if let toml::Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
    })
}

// ── Format section ─────────────────────────────────────────────

/// Show the active long-view format, if any.
///
/// The format may come from one of two sources:
/// - a `format = "..."` declaration anywhere in the personality
///   chain
/// - the implicit `-l`/`-ll`/`-lll` tier when the user invoked
///   `--show-config` alongside `-l` and no personality declared a
///   format (or columns)
///
/// When neither applies (e.g. a grid-view default invocation),
/// the section is omitted entirely.  The personality may still
/// declare `columns` directly — that's surfaced under the
/// Personality section, not here.
fn show_format(s: &Styles, personality_name: &str, implicit_format: Option<&str>) {
    let resolved = resolve_personality_full(personality_name).ok().flatten();
    let p = resolved.as_ref().map(|r| &r.def);

    // Skip if the personality declares `columns` directly — there
    // is no named format to display, and the columns already
    // appear under Personality.
    if p.is_some_and(|p| p.columns.is_some() && p.format.is_none()) {
        return;
    }

    let (fmt_name, source) = match p.and_then(|p| p.format.as_deref()) {
        Some(name) => (name, "personality".to_string()),
        None => match implicit_format {
            Some(name) => {
                let flag = match name {
                    "long" => "-l",
                    "long2" => "-ll",
                    "long3" => "-lll",
                    _ => "-l",
                };
                (name, format!("implicit, selected by {flag}"))
            }
            None => return,
        },
    };

    let formats = super::formats::resolve_formats();
    let columns = formats.get(fmt_name).map(|cols| cols.join(", "));

    println!("{} {}", s.label.paint("Format:"), s.heading.paint(fmt_name));
    println!("  {} {}", s.label.paint("source:"), s.dimmed.paint(&source));
    if let Some(cols) = columns {
        println!("  {} {}", s.label.paint("columns:"), s.value.paint(cols));
    }
    println!();
}

// ── Theme section ──────────────────────────────────────────────

/// Show theme details.  Returns the resolved style name (if any).
fn show_theme(
    s: &Styles,
    theme_name: Option<&str>,
    cfg: Option<&super::schema::Config>,
) -> Option<String> {
    if let Some(tname) = theme_name {
        println!("{} {}", s.label.paint("Theme:"), s.heading.paint(tname));
        let source = if is_builtin_theme(tname) {
            "builtin"
        } else if cfg.is_some_and(|c| c.theme.contains_key(tname)) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", s.label.paint("source:"), s.dimmed.paint(source));

        if is_builtin_theme(tname) {
            println!(
                "  {} {} {}",
                s.label.paint("use-style:"),
                s.name.paint("exa"),
                s.dimmed.paint("(implicit)")
            );
        } else if let Some(cfg) = cfg
            && let Some(theme) = cfg.theme.get(tname)
        {
            if let Some(ref inherits) = theme.inherits {
                println!(
                    "  {} {}",
                    s.label.paint("inherits:"),
                    s.name.paint(inherits)
                );
            }
            if let Some(ref style) = theme.use_style {
                println!("  {} {}", s.label.paint("use-style:"), s.name.paint(style));
            }
        }
    } else {
        println!("{} {}", s.label.paint("Theme:"), s.dimmed.paint("(none)"));
    }
    println!();

    // Resolve style name for downstream.
    theme_name.and_then(|tn| {
        if is_builtin_theme(tn) {
            Some("exa".to_string())
        } else if let Some(cfg) = cfg {
            cfg.theme.get(tn).and_then(|t| t.use_style.clone())
        } else {
            None
        }
    })
}

// ── Style section ──────────────────────────────────────────────

fn show_style(s: &Styles, style_name: Option<&str>, cfg: Option<&super::schema::Config>) {
    if let Some(sname) = style_name {
        println!("{} {}", s.label.paint("Style:"), s.heading.paint(sname));
        let source = if sname == "exa" {
            "builtin"
        } else if cfg.is_some_and(|c| c.style.contains_key(sname)) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", s.label.paint("source:"), s.dimmed.paint(source));

        if let Some(style) = resolve_style(sname) {
            if !style.classes.is_empty() {
                println!("  {}", s.label.paint("class references:"));
                let mut keys: Vec<_> = style.classes.keys().collect();
                keys.sort();
                for key in keys {
                    println!(
                        "    {} = {}",
                        s.name.paint(key),
                        s.value.paint(format!("\"{}\"", style.classes[key]))
                    );
                }
            }
            if !style.patterns.is_empty() {
                println!("  {}", s.label.paint("file patterns:"));
                let mut keys: Vec<_> = style.patterns.keys().collect();
                keys.sort();
                for key in keys {
                    println!(
                        "    {} = {}",
                        s.name.paint(format!("\"{key}\"")),
                        s.value.paint(format!("\"{}\"", style.patterns[key]))
                    );
                }
            }
        }
    } else {
        println!("{} {}", s.label.paint("Style:"), s.dimmed.paint("(none)"));
    }
    println!();
}

// ── Classes section ────────────────────────────────────────────

fn show_classes(s: &Styles, cfg: Option<&super::schema::Config>) {
    let classes = resolve_classes();
    println!(
        "{} {} defined",
        s.label.paint("Classes:"),
        s.value.paint(classes.len().to_string())
    );
    let mut names: Vec<_> = classes.keys().collect();
    names.sort();
    for cname in names {
        let source = if cfg.is_some_and(|c| c.class.contains_key(cname)) {
            "config"
        } else {
            "builtin"
        };
        let patterns = &classes[cname];
        println!(
            "  {} {}: {} patterns",
            s.name.paint(cname),
            s.dimmed.paint(format!("({source})")),
            s.value.paint(patterns.len().to_string())
        );
    }
    println!();
}

// ── Formats section ────────────────────────────────────────────

fn show_formats(s: &Styles, cfg: Option<&super::schema::Config>) {
    let formats = super::formats::resolve_formats();
    let compiled = ["long", "long2", "long3"];

    let mut names: Vec<String> = compiled.iter().map(|s| (*s).to_string()).collect();
    if let Some(cfg) = cfg {
        for fname in cfg.format.keys() {
            if !names.iter().any(|n| n == fname) {
                names.push(fname.clone());
            }
        }
    }
    names.sort();

    println!(
        "{} {} defined",
        s.label.paint("Formats:"),
        s.value.paint(names.len().to_string())
    );
    for fname in &names {
        let in_compiled = compiled.contains(&fname.as_str());
        let in_config = cfg.is_some_and(|c| c.format.contains_key(fname));
        let source = match (in_config, in_compiled) {
            (true, true) => "config, overrides builtin",
            (true, false) => "config",
            (false, true) => "builtin",
            (false, false) => "?",
        };
        let column_count = formats.get(fname.as_str()).map_or(0, Vec::len);
        let summary = if column_count == 1 {
            "1 column".to_string()
        } else {
            format!("{column_count} columns")
        };
        println!(
            "  {} {}: {}",
            s.name.paint(fname),
            s.dimmed.paint(format!("({source})")),
            s.value.paint(summary),
        );
    }
    println!();
}

// ── Available catalogue: personalities ─────────────────────────

fn show_available_personalities(s: &Styles, cfg: Option<&super::schema::Config>) {
    use super::personality::{is_compiled_personality, personality_description};

    let names = super::personality::all_personality_names();
    println!(
        "{} {} defined",
        s.label.paint("Personalities:"),
        s.value.paint(names.len().to_string())
    );
    for name in &names {
        let in_config = cfg.is_some_and(|c| c.personality.contains_key(name));
        let in_builtin = is_compiled_personality(name);
        let source = match (in_config, in_builtin) {
            (true, true) => "config, overrides builtin",
            (true, false) => "config",
            (false, true) => "builtin",
            (false, false) => "?",
        };
        let desc = personality_description(name);
        match desc.as_deref() {
            Some(d) if !d.is_empty() => println!(
                "  {} {}: {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
                s.value.paint(d),
            ),
            _ => println!(
                "  {} {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
            ),
        }
    }
    println!();
}

// ── Available catalogue: themes ────────────────────────────────

fn show_available_themes(s: &Styles, cfg: Option<&super::schema::Config>) {
    use super::themes::{all_theme_names, builtin_theme_description, is_builtin_theme};

    let names = all_theme_names();
    println!(
        "{} {} defined",
        s.label.paint("Themes:"),
        s.value.paint(names.len().to_string())
    );
    for name in &names {
        let in_config = cfg.is_some_and(|c| c.theme.contains_key(name));
        let in_builtin = is_builtin_theme(name);
        let source = match (in_config, in_builtin) {
            (true, true) => "config, overrides builtin",
            (true, false) => "config",
            (false, true) => "builtin",
            (false, false) => "?",
        };
        // User-defined description wins over builtin when the
        // user shadows a builtin name with their own block.
        let user_desc = cfg
            .and_then(|c| c.theme.get(name))
            .and_then(|t| t.description.as_deref());
        let desc = user_desc.or_else(|| builtin_theme_description(name));
        match desc {
            Some(d) if !d.is_empty() => println!(
                "  {} {}: {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
                s.value.paint(d),
            ),
            _ => println!(
                "  {} {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
            ),
        }
    }
    println!();
}

// ── Available catalogue: styles ────────────────────────────────

fn show_available_styles(s: &Styles, cfg: Option<&super::schema::Config>) {
    let names = super::styles::all_style_names();
    println!(
        "{} {} defined",
        s.label.paint("Styles:"),
        s.value.paint(names.len().to_string())
    );
    for name in &names {
        let in_config = cfg.is_some_and(|c| c.style.contains_key(name));
        let in_builtin = name == "exa";
        let source = match (in_config, in_builtin) {
            (true, true) => "config, overrides builtin",
            (true, false) => "config",
            (false, true) => "builtin",
            (false, false) => "?",
        };
        // Summary: count of class references and patterns, mirroring
        // the Classes section's "N patterns" shape.
        let resolved = resolve_style(name);
        let summary = match resolved {
            Some(style) => {
                let cls = style.classes.len();
                let pat = style.patterns.len();
                format!("{cls} class refs, {pat} patterns")
            }
            None => String::new(),
        };
        if summary.is_empty() {
            println!(
                "  {} {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
            );
        } else {
            println!(
                "  {} {}: {}",
                s.name.paint(name),
                s.dimmed.paint(format!("({source})")),
                s.value.paint(summary),
            );
        }
    }
    println!();
}
