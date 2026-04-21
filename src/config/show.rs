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

/// Display the active configuration to stdout.
///
/// Shows the resolved personality, format, theme, style, and classes,
/// indicating for each whether it's compiled-in or from the config file.
pub fn show_config(personality_name: &str, activated_by: &str, cli_theme_override: Option<&str>) {
    // Styling consistent with --help: yellow bold headers, cyan bold
    // literals/names, green values/paths, dimmed for source annotations.
    let heading = Style::new().bold().fg(Color::Yellow);
    let label = Style::new().bold();
    let name = Style::new().bold().fg(Color::Cyan);
    let value = Style::new().fg(Color::Green);
    let dimmed = Style::new().dimmed();

    let config_path = find_config_path();
    let cfg = config();

    println!("{}", heading.paint("lx configuration"));
    println!();

    // Config file.
    match &config_path {
        Some(p) => println!(
            "{} {}",
            label.paint("Config file:"),
            value.paint(p.display().to_string())
        ),
        None => println!("{} {}", label.paint("Config file:"), dimmed.paint("(none)")),
    }
    println!(
        "{} {}",
        label.paint("Config version:"),
        value.paint(CONFIG_VERSION)
    );
    if let Some(cfg) = cfg
        && !cfg.drop_in_paths.is_empty()
    {
        println!(
            "{} {} file(s) from conf.d/",
            label.paint("Drop-ins:"),
            value.paint(cfg.drop_in_paths.len().to_string())
        );
        for p in &cfg.drop_in_paths {
            println!("  {}", dimmed.paint(p.display().to_string()));
        }
    }
    println!();

    // Personality.
    println!(
        "{} {}",
        label.paint("Personality:"),
        name.paint(personality_name)
    );
    let source = if cfg.is_some_and(|c| c.personality.contains_key(personality_name)) {
        "config"
    } else {
        "builtin"
    };
    println!("  {} {}", label.paint("source:"), dimmed.paint(source));
    println!(
        "  {} {}",
        label.paint("activated by:"),
        dimmed.paint(activated_by)
    );

    if let Ok(Some(p)) = resolve_personality(personality_name) {
        if let Some(ref inherits) = p.inherits {
            println!("  {} {}", label.paint("inherits:"), name.paint(inherits));
        }
        if let Some(ref fmt) = p.format {
            println!("  {} {}", label.paint("format:"), name.paint(fmt));
        }
        if let Some(ref cols) = p.columns {
            println!(
                "  {} {}",
                label.paint("columns:"),
                value.paint(cols.to_csv())
            );
        }
        if !p.settings.is_empty() {
            println!("  {}", label.paint("settings:"));
            let mut keys: Vec<_> = p.settings.keys().collect();
            keys.sort();
            for key in keys {
                println!(
                    "    {} = {}",
                    name.paint(key),
                    value.paint(p.settings[key].to_string())
                );
            }
        }
    }
    println!();

    // Theme.  CLI `--theme=NAME` wins over the personality's stored
    // setting; otherwise fall back to the personality's `theme` key.
    let theme_name = cli_theme_override.map(String::from).or_else(|| {
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
    });

    if let Some(ref tname) = theme_name {
        println!("{} {}", label.paint("Theme:"), name.paint(tname));
        let source = if is_builtin_theme(tname) {
            "builtin"
        } else if cfg.is_some_and(|c| c.theme.contains_key(tname)) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", label.paint("source:"), dimmed.paint(source));

        if is_builtin_theme(tname) {
            println!(
                "  {} {} {}",
                label.paint("use-style:"),
                name.paint("exa"),
                dimmed.paint("(implicit)")
            );
        } else if let Some(cfg) = cfg
            && let Some(theme) = cfg.theme.get(tname)
        {
            if let Some(ref inherits) = theme.inherits {
                println!("  {} {}", label.paint("inherits:"), name.paint(inherits));
            }
            if let Some(ref style) = theme.use_style {
                println!("  {} {}", label.paint("use-style:"), name.paint(style));
            }
        }
    } else {
        println!("{} {}", label.paint("Theme:"), dimmed.paint("(none)"));
    }
    println!();

    // Style.
    let style_name = theme_name.as_deref().and_then(|tn| {
        if is_builtin_theme(tn) {
            Some("exa".to_string())
        } else if let Some(cfg) = cfg {
            cfg.theme.get(tn).and_then(|t| t.use_style.clone())
        } else {
            None
        }
    });

    if let Some(ref sname) = style_name {
        println!("{} {}", label.paint("Style:"), name.paint(sname));
        let source = if sname == "exa" {
            "builtin"
        } else if cfg.is_some_and(|c| c.style.contains_key(sname)) {
            "config"
        } else {
            "unknown"
        };
        println!("  {} {}", label.paint("source:"), dimmed.paint(source));

        if let Some(style) = resolve_style(sname) {
            if !style.classes.is_empty() {
                println!("  {}", label.paint("class references:"));
                let mut keys: Vec<_> = style.classes.keys().collect();
                keys.sort();
                for key in keys {
                    println!(
                        "    {} = {}",
                        name.paint(key),
                        value.paint(format!("\"{}\"", style.classes[key]))
                    );
                }
            }
            if !style.patterns.is_empty() {
                println!("  {}", label.paint("file patterns:"));
                let mut keys: Vec<_> = style.patterns.keys().collect();
                keys.sort();
                for key in keys {
                    println!(
                        "    {} = {}",
                        name.paint(format!("\"{key}\"")),
                        value.paint(format!("\"{}\"", style.patterns[key]))
                    );
                }
            }
        }
    } else {
        println!("{} {}", label.paint("Style:"), dimmed.paint("(none)"));
    }
    println!();

    // Classes.
    let classes = resolve_classes();
    println!(
        "{} {} defined",
        label.paint("Classes:"),
        value.paint(classes.len().to_string())
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
            name.paint(cname),
            dimmed.paint(format!("({source})")),
            value.paint(patterns.len().to_string())
        );
    }
    println!();

    // Formats.
    println!("{}", label.paint("Formats:"));
    let compiled = vec!["long", "long2", "long3"];
    for fname in &compiled {
        let source = if cfg.is_some_and(|c| c.format.contains_key(*fname)) {
            "config (overrides builtin)"
        } else {
            "builtin"
        };
        println!("  {}: {}", name.paint(*fname), dimmed.paint(source));
    }
    if let Some(cfg) = cfg {
        for fname in cfg.format.keys() {
            if !compiled.contains(&fname.as_str()) {
                println!("  {}: {}", name.paint(fname), dimmed.paint("config"));
            }
        }
    }
}
