use crate::options::{flags, OptionsError, NumberSource};
use crate::options::parser::MatchedFlags;
use crate::options::vars::{self, Vars};

use crate::output::file_name::{Options, Classify, ShowIcons};


impl Options {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let classify = Classify::deduce(matches);
        let show_icons = ShowIcons::deduce(matches, vars)?;

        Ok(Self { classify, show_icons })
    }
}

impl Classify {
    fn deduce(matches: &MatchedFlags) -> Self {
        match matches.get(flags::CLASSIFY) {
            Some("always")  => Self::AddFileIndicators,
            Some("auto")    => Self::AddFileIndicators,  // TODO: check TTY
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
            Some("auto")    => true,   // TODO: check TTY
            Some("never")   => false,
            _               => false,  // absent = off
        };

        if !show {
            return Ok(Self::Off);
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
