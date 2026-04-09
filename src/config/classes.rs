//! File-type class definitions: `[class]` resolution and the
//! `--show-class` output.

use std::collections::HashMap;

use super::error::ConfigError;
use super::store::config;


/// Return the compiled-in file-type class definitions.
///
/// These correspond to the categories in `src/info/filetype.rs`.
/// Config-defined `[class]` entries override these.
pub fn compiled_classes() -> HashMap<String, Vec<String>> {
    fn gl(exts: &[&str]) -> Vec<String> {
        exts.iter().map(|e| format!("*.{e}")).collect()
    }

    HashMap::from([
        ("image".into(), gl(&[
            "png", "jfi", "jfif", "jif", "jpe", "jpeg", "jpg", "gif", "bmp",
            "tiff", "tif", "ppm", "pgm", "pbm", "pnm", "webp", "raw", "arw",
            "svg", "stl", "eps", "dvi", "ps", "cbr", "jpf", "cbz", "xpm",
            "ico", "cr2", "orf", "nef", "heif", "avif", "jxl", "j2k", "jp2",
            "j2c", "jpx",
        ])),
        ("video".into(), gl(&[
            "avi", "flv", "m2v", "m4v", "mkv", "mov", "mp4", "mpeg",
            "mpg", "ogm", "ogv", "vob", "wmv", "webm", "m2ts", "heic",
        ])),
        ("music".into(), gl(&[
            "aac", "m4a", "mp3", "ogg", "wma", "mka", "opus",
        ])),
        ("lossless".into(), gl(&[
            "alac", "ape", "flac", "wav",
        ])),
        ("crypto".into(), gl(&[
            "asc", "enc", "gpg", "pgp", "sig", "signature", "pfx", "p12",
        ])),
        ("document".into(), gl(&[
            "djvu", "doc", "docx", "dvi", "eml", "eps", "fotd", "key",
            "keynote", "numbers", "odp", "odt", "pages", "pdf", "ppt",
            "pptx", "rtf", "xls", "xlsx",
        ])),
        ("compressed".into(), gl(&[
            "zip", "tar", "Z", "z", "gz", "bz2", "a", "ar", "7z",
            "iso", "dmg", "tc", "rar", "par", "tgz", "xz", "txz",
            "lz", "tlz", "lzma", "deb", "rpm", "zst", "lz4", "cpio",
        ])),
        ("compiled".into(), gl(&[
            "class", "elc", "hi", "o", "pyc", "zwc", "ko",
        ])),
        ("temp".into(), gl(&[
            "tmp", "swp", "swo", "swn", "bak", "bkp", "bk",
        ])),
        ("immediate".into(), vec![
            "Makefile".into(), "Cargo.toml".into(), "SConstruct".into(),
            "CMakeLists.txt".into(), "build.gradle".into(), "pom.xml".into(),
            "Rakefile".into(), "package.json".into(), "Gruntfile.js".into(),
            "Gruntfile.coffee".into(), "BUILD".into(), "BUILD.bazel".into(),
            "WORKSPACE".into(), "build.xml".into(), "Podfile".into(),
            "webpack.config.js".into(), "meson.build".into(),
            "composer.json".into(), "RoboFile.php".into(), "PKGBUILD".into(),
            "Justfile".into(), "Procfile".into(), "Dockerfile".into(),
            "Containerfile".into(), "Vagrantfile".into(), "Brewfile".into(),
            "Gemfile".into(), "Pipfile".into(), "build.sbt".into(),
            "mix.exs".into(), "bsconfig.json".into(), "tsconfig.json".into(),
        ]),
    ])
}

/// Resolve class definitions: config overrides compiled-in defaults.
pub fn resolve_classes() -> HashMap<String, Vec<String>> {
    let mut classes = compiled_classes();
    if let Some(cfg) = config() {
        for (name, patterns) in &cfg.class {
            classes.insert(name.clone(), patterns.clone());
        }
    }
    classes
}


// ── --show-class output ─────────────────────────────────────────

/// Format a single class definition as TOML.
fn format_class_toml(name: &str, patterns: &[String]) -> String {
    // Format as a TOML array that's readable — wrap at ~72 chars.
    let indent = " ".repeat(name.len() + 4); // align continuation lines
    let mut lines = vec![format!("{name} = [")];

    for (i, pat) in patterns.iter().enumerate() {
        let entry = format!("\"{pat}\"");
        let last = lines.last_mut().unwrap();

        if i == 0 {
            last.push_str(&entry);
        } else {
            // Would adding ", entry" exceed 72 chars?
            let trial_len = last.len() + 2 + entry.len();
            if trial_len > 72 {
                last.push(',');
                lines.push(format!("{indent}{entry}"));
            } else {
                last.push_str(", ");
                last.push_str(&entry);
            }
        }
    }
    lines.last_mut().unwrap().push(']');
    lines.join("\n")
}

/// Print a single class definition as copy-pasteable TOML.
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if `name` does not match any
/// compiled-in or user-defined file-type class.
pub fn show_class(name: &str) -> Result<(), ConfigError> {
    let classes = resolve_classes();
    if let Some(patterns) = classes.get(name) {
        println!("[class]");
        println!("{}", format_class_toml(name, patterns));
        Ok(())
    } else {
        let mut names: Vec<_> = classes.keys().collect();
        names.sort();
        let candidates = names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        Err(ConfigError::NotFound {
            kind: "class",
            kind_plural: "classes",
            name: name.to_string(),
            candidates,
        })
    }
}

/// Print all class definitions as copy-pasteable TOML.
pub fn show_class_all() {
    let classes = resolve_classes();
    let mut names: Vec<_> = classes.keys().collect();
    names.sort();

    println!("[class]");
    for name in names {
        println!("{}", format_class_toml(name, &classes[name]));
    }
}
