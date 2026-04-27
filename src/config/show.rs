//! `--show-config`: human-readable overview of the active configuration.
//!
//! This is the only output path that pulls from *every* part of the
//! config layer (personality, theme, style, classes, formats), so it
//! lives in its own file rather than alongside any one of them.

use nu_ansi_term::{Color, Style};

use super::classes::resolve_classes;
use super::load::find_config_path;
use super::personality::resolve_personality;
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
pub fn show_config(personality_name: &str, activated_by: &str, cli_theme_override: Option<&str>) {
    let s = Styles::new();
    let config_path = find_config_path();
    let cfg = config();

    println!("{}", s.heading.paint("lx configuration"));
    println!();

    show_config_file(&s, config_path.as_ref(), cfg);
    let theme_name = show_personality(&s, personality_name, activated_by, cli_theme_override, cfg);
    let style_name = show_theme(&s, theme_name.as_deref(), cfg);
    show_style(&s, style_name.as_deref(), cfg);
    show_classes(&s, cfg);
    show_formats(&s, cfg);
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
    cfg: Option<&super::schema::Config>,
) -> Option<String> {
    println!(
        "{} {}",
        s.label.paint("Personality:"),
        s.name.paint(personality_name)
    );
    let source = if cfg.is_some_and(|c| c.personality.contains_key(personality_name)) {
        "config"
    } else {
        "builtin"
    };
    println!("  {} {}", s.label.paint("source:"), s.dimmed.paint(source));
    println!(
        "  {} {}",
        s.label.paint("activated by:"),
        s.dimmed.paint(activated_by)
    );

    if let Ok(Some(p)) = resolve_personality(personality_name) {
        if let Some(ref inherits) = p.inherits {
            println!(
                "  {} {}",
                s.label.paint("inherits:"),
                s.name.paint(inherits)
            );
        }
        if let Some(ref fmt) = p.format {
            println!("  {} {}", s.label.paint("format:"), s.name.paint(fmt));
        }
        if let Some(ref cols) = p.columns {
            println!(
                "  {} {}",
                s.label.paint("columns:"),
                s.value.paint(cols.to_csv())
            );
        }
        if !p.settings.is_empty() {
            println!("  {}", s.label.paint("settings:"));
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
    }
    println!();

    // Resolve the theme name for downstream sections.
    cli_theme_override.map(String::from).or_else(|| {
        resolve_personality(personality_name)
            .ok()
            .flatten()
            .and_then(|p| {
                p.settings.get("theme").and_then(|v| {
                    if let toml::Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
            })
    })
}

// ── Theme section ──────────────────────────────────────────────

/// Show theme details.  Returns the resolved style name (if any).
fn show_theme(
    s: &Styles,
    theme_name: Option<&str>,
    cfg: Option<&super::schema::Config>,
) -> Option<String> {
    if let Some(tname) = theme_name {
        println!("{} {}", s.label.paint("Theme:"), s.name.paint(tname));
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
        println!("{} {}", s.label.paint("Style:"), s.name.paint(sname));
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
    println!("{}", s.label.paint("Formats:"));
    let compiled = vec!["long", "long2", "long3"];
    for fname in &compiled {
        let source = if cfg.is_some_and(|c| c.format.contains_key(*fname)) {
            "config (overrides builtin)"
        } else {
            "builtin"
        };
        println!("  {}: {}", s.name.paint(*fname), s.dimmed.paint(source));
    }
    if let Some(cfg) = cfg {
        for fname in cfg.format.keys() {
            if !compiled.contains(&fname.as_str()) {
                println!("  {}: {}", s.name.paint(fname), s.dimmed.paint("config"));
            }
        }
    }
}
