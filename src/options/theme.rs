use crate::options::{flags, vars, Vars, OptionsError};
use crate::options::parser::MatchedFlags;
use crate::theme::{Options, UseColours, ColourScale, GradientFlags, Definitions};


impl Options {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let use_colours = UseColours::deduce(matches, vars)?;
        let colour_scale = ColourScale::deduce(matches);
        let gradient = GradientFlags::deduce(matches, colour_scale);

        let definitions = if use_colours == UseColours::Never {
                Definitions::default()
            }
            else {
                Definitions::deduce(vars)
            };

        let theme_override = matches.get(flags::THEME).map(String::from);

        Ok(Self { use_colours, colour_scale, gradient, definitions, theme_override })
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


impl ColourScale {
    fn deduce(matches: &MatchedFlags) -> Self {
        match matches.get(flags::COLOR_SCALE) {
            Some("16")   => Self::Scale16,
            Some("256")  => Self::Scale256,
            Some("none") => Self::None,
            _            => Self::None,
        }
    }
}


impl GradientFlags {
    /// Deduce per-column gradient on/off from the CLI flags.
    ///
    /// Precedence (later wins):
    /// 1. Default → all on.
    /// 2. Legacy `--colour-scale` translation: `=none` collapses
    ///    both gradients off; `=16`/`=256` (or bare) leaves both on.
    ///    This bridges old users until commit 3 retires the flag.
    /// 3. `--gradient=...` overrides everything.
    /// 4. `--no-gradient` (counted Arg) overrides everything,
    ///    including a preceding `--gradient=...`, since it sits
    ///    after `--gradient` in the argv ordering when both are
    ///    given.  Modelled here as "if --no-gradient was passed,
    ///    return NONE".
    fn deduce(matches: &MatchedFlags, colour_scale: ColourScale) -> Self {
        // Start from the legacy translation.
        let mut flags = match colour_scale {
            ColourScale::None => Self::NONE,
            ColourScale::Scale16 | ColourScale::Scale256 => Self::ALL,
        };
        // If --colour-scale wasn't given at all, ColourScale::deduce
        // returns None — but that's also "user didn't ask for flat",
        // so we treat it as the default `ALL` (the same as the
        // bare-default fall-through above wants).  Distinguish via
        // matches.get directly.
        if matches.get(flags::COLOR_SCALE).is_none() {
            flags = Self::ALL;
        }
        // --gradient=... wins over the legacy flag.
        if let Some(s) = matches.get(flags::GRADIENT) {
            flags = parse_gradient_value(s);
        }
        // --no-gradient (any count) wins over --gradient.
        if matches.has(flags::NO_GRADIENT) {
            flags = Self::NONE;
        }
        flags
    }
}

/// Parse the value of `--gradient` (already validated by clap's
/// `GradientParser`) into a `GradientFlags`.  Empty / `all` → all
/// on; `none` → all off; comma-separated column names → those
/// columns on, others off.
fn parse_gradient_value(s: &str) -> GradientFlags {
    let mut flags = GradientFlags::NONE;
    for tok in s.split(',') {
        match tok.trim() {
            "" => {} // ignore stray empties
            "none" => return GradientFlags::NONE,
            "all" => return GradientFlags::ALL,
            "size" => flags.size = true,
            "date" => flags.date = true,
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
        let lx = vars.get(vars::LX_COLORS).map(|e| e.to_string_lossy().to_string());
        Self { ls, lx }
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
        lx: &'static str,
        no_color: &'static str,
    }

    impl MockVars {
        fn empty() -> MockVars {
            MockVars {
                ls: "",
                lx: "",
                no_color: "",
            }
        }
        fn with_no_color() -> MockVars {
            MockVars {
                ls: "",
                lx: "",
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
            else if name == vars::LX_COLORS && ! self.lx.is_empty() {
                Some(OsString::from(self.lx))
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

    test!(scale_bare:   ColourScale <- ["--colour-scale"];                ColourScale::Scale16);
    test!(scale_16:     ColourScale <- ["--colour-scale=16"];              ColourScale::Scale16);
    test!(scale_256:    ColourScale <- ["--colour-scale=256"];             ColourScale::Scale256);
    test!(scale_none:   ColourScale <- ["--colour-scale=none"];            ColourScale::None);
    test!(scale_alias:  ColourScale <- ["--color-scale"];                  ColourScale::Scale16);
    test!(scale_absent: ColourScale <- [];                                 ColourScale::None);
}
