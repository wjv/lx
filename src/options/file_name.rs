use std::io::IsTerminal;

use crate::options::{flags, OptionsError, NumberSource};
use crate::options::parser::MatchedFlags;
use crate::options::vars::{self, Vars};

use crate::output::file_name::{Options, Classify, ShowIcons, Quotes};


impl Options {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let classify = Classify::deduce(matches);
        let show_icons = ShowIcons::deduce(matches, vars)?;
        let absolute = matches.has(flags::ABSOLUTE);
        let hyperlink = match matches.get(flags::HYPERLINK) {
            Some("always") => true,
            Some("auto")   => std::io::stdout().is_terminal(),
            _              => false,
        };
        let quotes = match matches.get(flags::QUOTES) {
            Some("always" | "auto") => Quotes::Always,
            _ => Quotes::Never,
        };

        Ok(Self { classify, show_icons, absolute, hyperlink, quotes })
    }
}

impl Classify {
    fn deduce(matches: &MatchedFlags) -> Self {
        match matches.get(flags::CLASSIFY) {
            Some("always")  => Self::AddFileIndicators,
            Some("auto")    => if std::io::stdout().is_terminal() { Self::AddFileIndicators } else { Self::JustFilenames },
            Some("never")   => Self::JustFilenames,
            _               => Self::JustFilenames,
        }
    }
}

impl ShowIcons {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        // --no-icons always wins.
        if matches.has(flags::NO_ICONS) {
            return Ok(Self::Off);
        }

        // Check --icons=WHEN value.
        let show = match matches.get(flags::ICONS) {
            Some("always")  => true,
            Some("auto")    => std::io::stdout().is_terminal(),
            Some("never")   => false,
            _               => false,  // absent = off
        };

        if !show {
            return Ok(Self::Off);
        }

        // Config/CLI flag takes precedence over environment variable.
        if let Some(spacing) = matches.get("icon-spacing") {
            return match spacing.parse::<u32>() {
                Ok(n) => Ok(Self::On(n)),
                Err(e) => Err(OptionsError::FailedParse(spacing.into(), NumberSource::Arg("icon-spacing"), e)),
            };
        }

        if let Some(columns) = vars.get(vars::LX_ICON_SPACING).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(width) => {
                    Ok(Self::On(width))
                }
                Err(e) => {
                    let source = NumberSource::Env(vars::LX_ICON_SPACING);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        }
        else {
            Ok(Self::On(1))
        }
    }
}
