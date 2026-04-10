use crate::options::{flags, vars, Vars, OptionsError};
use crate::options::parser::MatchedFlags;
use crate::theme::{Options, UseColours, GradientFlags, Definitions};


impl Options {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let use_colours = UseColours::deduce(matches, vars)?;
        let gradient = GradientFlags::deduce(matches);

        let definitions = if use_colours == UseColours::Never {
                Definitions::default()
            }
            else {
                Definitions::deduce(vars)
            };

        let theme_override = matches.get(flags::THEME).map(String::from);

        Ok(Self { use_colours, gradient, definitions, theme_override })
    }
}


impl UseColours {
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let default_value = match vars.get(vars::NO_COLOR) {
            Some(_) => Self::Never,
            None => Self::Automatic,
        };

        let word = match matches.get(flags::COLOR) {
            Some(w)  => w,
            None => return Ok(default_value),
        };

        // Clap validates the value, so this match is exhaustive over accepted inputs.
        Ok(match word {
            "always"                => Self::Always,
            "auto" | "automatic"    => Self::Automatic,
            "never"                 => Self::Never,
            _                       => unreachable!("Clap rejects invalid --colour values"),
        })
    }
}


impl GradientFlags {
    /// Deduce per-column gradient on/off from the CLI flags.
    ///
    /// Precedence:
    /// 1. Default → all on, smooth off.
    /// 2. `--gradient=...` overrides the default.
    /// 3. `--no-gradient` overrides `--gradient`.
    /// 4. `--smooth` turns smoothing on; `--no-smooth` forces it off.
    ///    Smoothing is independent of the per-column flags — it has
    ///    no effect on columns whose gradient is off.
    fn deduce(matches: &MatchedFlags) -> Self {
        let mut flags = Self::ALL;
        if let Some(s) = matches.get(flags::GRADIENT) {
            flags = parse_gradient_value(s);
        }
        if matches.has(flags::NO_GRADIENT) {
            flags = Self::NONE;
        }
        if matches.has(flags::SMOOTH) {
            flags.smooth = true;
        }
        if matches.has(flags::NO_SMOOTH) {
            flags.smooth = false;
        }
        flags
    }
}

/// Parse the value of `--gradient` (already validated by clap's
/// `GradientParser`) into a `GradientFlags`.  Empty / `all` → all
/// on; `none` → all off; comma-separated column names → those
/// columns on, others off.
///
/// `filesize` is a hidden alias for `size`, and `timestamp` is a
/// hidden alias for `date` — both match the column-add flag
/// spellings.  `date` / `timestamp` are bulk setters that flip all
/// four per-timestamp flags at once; the individual `modified` /
/// `accessed` / `changed` / `created` tokens flip just one.
fn parse_gradient_value(s: &str) -> GradientFlags {
    let mut flags = GradientFlags::NONE;
    for tok in s.split(',') {
        match tok.trim() {
            "" => {} // ignore stray empties
            "none" => return GradientFlags::NONE,
            "all" => return GradientFlags::ALL,
            "size" | "filesize" => flags.size = true,
            "date" | "timestamp" => {
                flags.modified = true;
                flags.accessed = true;
                flags.changed  = true;
                flags.created  = true;
            }
            "modified" => flags.modified = true,
            "accessed" => flags.accessed = true,
            "changed"  => flags.changed  = true,
            "created"  => flags.created  = true,
            // GradientParser already rejected anything else; this is
            // unreachable in practice.
            _ => {}
        }
    }
    flags
}


impl Definitions {
    fn deduce<V: Vars>(vars: &V) -> Self {
        let ls = vars.get(vars::LS_COLORS).map(|e| e.to_string_lossy().to_string());
        Self { ls }
    }
}


#[cfg(test)]
mod terminal_test {
    use super::*;
    use std::ffi::OsString;

    use crate::options::test::parse_for_test;

    macro_rules! test {
        ($name:ident:  $type:ident <- $inputs:expr;  $result:expr) => {
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf)) {
                    assert_eq!(result, $result);
                }
            }
        };

        ($name:ident:  $type:ident <- $inputs:expr, $env:expr;  $result:expr) => {
            #[test]
            fn $name() {
                let env = $env;
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf, &env)) {
                    assert_eq!(result, $result);
                }
            }
        };
    }

    struct MockVars {
        ls: &'static str,
        no_color: &'static str,
    }

    impl MockVars {
        fn empty() -> MockVars {
            MockVars {
                ls: "",
                no_color: "",
            }
        }
        fn with_no_color() -> MockVars {
            MockVars {
                ls: "",
                no_color: "true",
            }
        }
    }

    // Test impl that just returns the value it has.
    impl Vars for MockVars {
        fn get(&self, name: &'static str) -> Option<OsString> {
            if name == vars::LS_COLORS && ! self.ls.is_empty() {
                Some(OsString::from(self.ls))
            }
            else if name == vars::NO_COLOR && ! self.no_color.is_empty() {
                Some(OsString::from(self.no_color))
            }
            else {
                None
            }
        }
    }



    // Default
    test!(empty:         UseColours <- [], MockVars::empty();                     Ok(UseColours::Automatic));
    test!(empty_with_no_color: UseColours <- [], MockVars::with_no_color();       Ok(UseColours::Never));

    // --colour
    test!(u_always:      UseColours <- ["--colour=always"], MockVars::empty();    Ok(UseColours::Always));
    test!(u_auto:        UseColours <- ["--colour", "auto"], MockVars::empty();   Ok(UseColours::Automatic));
    test!(u_never:       UseColours <- ["--colour=never"], MockVars::empty();     Ok(UseColours::Never));

    // --color
    test!(no_u_always:   UseColours <- ["--color", "always"], MockVars::empty();  Ok(UseColours::Always));
    test!(no_u_auto:     UseColours <- ["--color=auto"], MockVars::empty();       Ok(UseColours::Automatic));
    test!(no_u_never:    UseColours <- ["--color", "never"], MockVars::empty();   Ok(UseColours::Never));

    // Errors — Clap rejects invalid values at parse time
    #[test]
    fn no_u_error() {
        let cmd = crate::options::parser::build_command();
        assert!(cmd.try_get_matches_from(["lx", "--color=upstream"]).is_err());
    }
    #[test]
    fn u_error() {
        let cmd = crate::options::parser::build_command();
        assert!(cmd.try_get_matches_from(["lx", "--colour=lovers"]).is_err());
    }

    // Overriding
    test!(overridden_1:  UseColours <- ["--colour=auto", "--colour=never"], MockVars::empty();  Ok(UseColours::Never));
    test!(overridden_2:  UseColours <- ["--color=auto",  "--colour=never"], MockVars::empty();  Ok(UseColours::Never));
    test!(overridden_3:  UseColours <- ["--colour=auto", "--color=never"], MockVars::empty();   Ok(UseColours::Never));
    test!(overridden_4:  UseColours <- ["--color=auto",  "--color=never"], MockVars::empty();   Ok(UseColours::Never));

    // --colour-scale is retired in 0.9; clap rejects any value at
    // parse time with a deprecation pointer to --gradient.  See
    // tests/cli_basics.rs::colour_scale_deprecated for the
    // user-facing assertion.


    // --gradient and --smooth
    test!(gf_default:     GradientFlags <- [];                          GradientFlags::ALL);
    test!(gf_no_gradient: GradientFlags <- ["--no-gradient"];           GradientFlags::NONE);
    test!(gf_size_only:   GradientFlags <- ["--gradient=size"];
        GradientFlags { size: true, modified: false, accessed: false, changed: false, created: false, smooth: false });
    test!(gf_smooth:      GradientFlags <- ["--smooth"];
        GradientFlags { smooth: true, ..GradientFlags::ALL });
    test!(gf_no_smooth_alone:  GradientFlags <- ["--no-smooth"];        GradientFlags::ALL);
    test!(gf_smooth_then_no:   GradientFlags <- ["--smooth", "--no-smooth"]; GradientFlags::ALL);
    test!(gf_smooth_with_gradient_size: GradientFlags <- ["--gradient=size", "--smooth"];
        GradientFlags { size: true, modified: false, accessed: false, changed: false, created: false, smooth: true });
    test!(gf_no_gradient_then_smooth: GradientFlags <- ["--no-gradient", "--smooth"];
        GradientFlags { smooth: true, ..GradientFlags::NONE });
}
