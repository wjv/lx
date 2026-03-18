use crate::fs::feature::xattr;
use crate::options::{flags, OptionsError, NumberSource, Vars};
use crate::options::parser::MatchedFlags;
use crate::output::{View, Mode, TerminalWidth, grid, details};
use crate::output::grid_details::{self, RowThreshold};
use crate::output::file_name::Options as FileStyle;
use crate::output::table::{TimeTypes, SizeFormat, UserFormat, Columns, Options as TableOptions};
use crate::output::time::TimeFormat;


impl View {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let mode = Mode::deduce(matches, vars)?;
        let width = TerminalWidth::deduce(vars)?;
        let file_style = FileStyle::deduce(matches, vars)?;
        Ok(Self { mode, width, file_style })
    }
}


impl Mode {

    /// Determine which viewing mode to use based on the user's options.
    ///
    /// Clap's `overrides_with` ensures that conflicting flags (long/grid/
    /// oneline) resolve to "last flag wins".  Tree has no override
    /// relationships — it combines with long or stands alone, and takes
    /// priority over grid when both are present.
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let long    = matches.has(flags::LONG);
        let tree    = matches.has(flags::TREE);
        let grid    = matches.has(flags::GRID);
        let oneline = matches.has(flags::ONE_LINE);

        // Tree takes priority over grid when combined with long.
        if long && tree {
            let details = details::Options::deduce_long(matches, vars)?;
            return Ok(Self::Details(details));
        }

        if long && grid {
            let details = details::Options::deduce_long(matches, vars)?;
            let grid = grid::Options::deduce(matches);
            let row_threshold = RowThreshold::deduce(vars)?;
            let grid_details = grid_details::Options { grid, details, row_threshold };
            return Ok(Self::GridDetails(grid_details));
        }

        if long {
            let details = details::Options::deduce_long(matches, vars)?;
            return Ok(Self::Details(details));
        }

        // Tree without long: tree-only view (no table columns).
        if tree {
            let details = details::Options::deduce_tree(matches);
            return Ok(Self::Details(details));
        }

        if oneline {
            return Ok(Self::Lines);
        }

        let grid = grid::Options::deduce(matches);
        Ok(Self::Grid(grid))
    }
}


impl grid::Options {
    fn deduce(matches: &MatchedFlags) -> Self {
        Self {
            across: matches.has(flags::ACROSS),
        }
    }
}


impl details::Options {
    fn deduce_tree(matches: &MatchedFlags) -> Self {
        Self {
            table: None,
            header: false,
            xattr: xattr::ENABLED && matches.has(flags::EXTENDED),
        }
    }

    fn deduce_long<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        Ok(Self {
            table: Some(TableOptions::deduce(matches, vars)?),
            header: matches.has(flags::HEADER),
            xattr: xattr::ENABLED && matches.has(flags::EXTENDED),
        })
    }
}


impl TerminalWidth {
    fn deduce<V: Vars>(vars: &V) -> Result<Self, OptionsError> {
        use crate::options::vars;

        if let Some(columns) = vars.get(vars::COLUMNS).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(width) => {
                    Ok(Self::Set(width))
                }
                Err(e) => {
                    let source = NumberSource::Env(vars::COLUMNS);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        }
        else {
            Ok(Self::Automatic)
        }
    }
}


impl RowThreshold {
    fn deduce<V: Vars>(vars: &V) -> Result<Self, OptionsError> {
        use crate::options::vars;

        if let Some(columns) = vars.get(vars::EXA_GRID_ROWS).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(rows) => {
                    Ok(Self::MinimumRows(rows))
                }
                Err(e) => {
                    let source = NumberSource::Env(vars::EXA_GRID_ROWS);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        }
        else {
            Ok(Self::AlwaysGrid)
        }
    }
}


impl TableOptions {
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let time_format = TimeFormat::deduce(matches, vars)?;
        let size_format = SizeFormat::deduce(matches);
        let user_format = UserFormat::deduce(matches);
        let columns = Columns::deduce(matches)?;
        Ok(Self { size_format, time_format, user_format, columns })
    }
}


impl Columns {
    fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        let time_types = TimeTypes::deduce(matches)?;
        let git = matches.has(flags::GIT);

        let blocks = matches.has(flags::BLOCKS);
        let group  = matches.has(flags::GROUP);
        let inode  = matches.has(flags::INODE);
        let links  = matches.has(flags::LINKS);
        let octal  = matches.has(flags::OCTAL);

        let permissions = ! matches.has(flags::NO_PERMISSIONS);
        let filesize =    ! matches.has(flags::NO_FILESIZE);
        let user =        ! matches.has(flags::NO_USER);

        Ok(Self { time_types, inode, links, blocks, group, git, octal, permissions, filesize, user })
    }
}


impl SizeFormat {

    /// Determine which file size to use in the file size column based on
    /// the user's options.
    ///
    /// The default mode is to use the decimal prefixes, as they are the
    /// most commonly-understood, and don't involve trying to parse large
    /// strings of digits in your head. Changing the format to anything else
    /// involves the `--binary` or `--bytes` flags, and these conflict with
    /// each other — Clap's `overrides_with` ensures only the last one wins.
    fn deduce(matches: &MatchedFlags) -> Self {
        if matches.has(flags::BINARY) {
            Self::BinaryBytes
        }
        else if matches.has(flags::BYTES) {
            Self::JustBytes
        }
        else {
            Self::DecimalBytes
        }
    }
}


impl TimeFormat {

    /// Determine how time should be formatted in timestamp columns.
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        // When --time-style is given, Clap has already validated the value.
        if let Some(w) = matches.get(flags::TIME_STYLE) {
            return Ok(Self::from_str(w));
        }

        use crate::options::vars;
        match vars.get(vars::TIME_STYLE) {
            Some(ref t) if ! t.is_empty()  => {
                // Environment variable — not validated by Clap; fall back
                // to the default for unknown values.
                Ok(Self::from_str(&t.to_string_lossy()))
            }
            _ => Ok(Self::DefaultFormat),
        }
    }

    fn from_str(word: &str) -> Self {
        match word {
            "default"  => Self::DefaultFormat,
            "iso"      => Self::ISOFormat,
            "long-iso" => Self::LongISO,
            "full-iso" => Self::FullISO,
            _          => Self::DefaultFormat,
        }
    }
}


impl UserFormat {
    fn deduce(matches: &MatchedFlags) -> Self {
        if matches.has(flags::NUMERIC) { Self::Numeric } else { Self::Name }
    }
}


impl TimeTypes {

    /// Determine which of a file's time fields should be displayed for it
    /// based on the user's options.
    ///
    /// There are two separate ways to pick which fields to show: with a
    /// flag (such as `--modified`) or with a parameter (such as
    /// `--time=modified`). An error is signalled if both ways are used.
    ///
    /// It's valid to show more than one column by passing in more than one
    /// option, but passing *no* options means that the user just wants to
    /// see the default set.
    fn deduce(matches: &MatchedFlags) -> Result<Self, OptionsError> {
        let modified = matches.has(flags::MODIFIED);
        let changed  = matches.has(flags::CHANGED);
        let accessed = matches.has(flags::ACCESSED);
        let created  = matches.has(flags::CREATED);

        let no_time = matches.has(flags::NO_TIME);

        // Clap validates --time values and enforces conflicts with
        // --modified/--changed/--accessed/--created.
        let time_types = if no_time {
            Self { modified: false, changed: false, accessed: false, created: false }
        } else if let Some(word) = matches.get(flags::TIME) {
            match word {
                "mod" | "modified" => Self { modified: true,  changed: false, accessed: false, created: false },
                "ch"  | "changed"  => Self { modified: false, changed: true,  accessed: false, created: false },
                "acc" | "accessed" => Self { modified: false, changed: false, accessed: true,  created: false },
                "cr"  | "created"  => Self { modified: false, changed: false, accessed: false, created: true  },
                _ => unreachable!("Clap rejects invalid --time values"),
            }
        }
        else if modified || changed || accessed || created {
            Self { modified, changed, accessed, created }
        }
        else {
            Self::default()
        };

        Ok(time_types)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    use crate::options::test::parse_for_test;

    macro_rules! test {

        ($name:ident: $type:ident <- $inputs:expr; $result:expr) => {
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf)) {
                    assert_eq!(result, $result);
                }
            }
        };

        ($name:ident: $type:ident <- $inputs:expr, $vars:expr; like $pat:pat) => {
            #[test]
            fn $name() {
                for result in parse_for_test($inputs.as_ref(), |mf| $type::deduce(mf, &$vars)) {
                    println!("Testing {:?}", result);
                    match result {
                        $pat => assert!(true),
                        _    => assert!(false),
                    }
                }
            }
        };
    }


    mod size_formats {
        use super::*;

        // Default behaviour
        test!(empty:   SizeFormat <- [];                       SizeFormat::DecimalBytes);

        // Individual flags
        test!(binary:  SizeFormat <- ["--binary"];             SizeFormat::BinaryBytes);
        test!(bytes:   SizeFormat <- ["--bytes"];              SizeFormat::JustBytes);

        // Overriding
        test!(both_1:  SizeFormat <- ["--binary", "--binary"]; SizeFormat::BinaryBytes);
        test!(both_2:  SizeFormat <- ["--bytes",  "--binary"]; SizeFormat::BinaryBytes);
        test!(both_3:  SizeFormat <- ["--binary", "--bytes"];  SizeFormat::JustBytes);
        test!(both_4:  SizeFormat <- ["--bytes",  "--bytes"];  SizeFormat::JustBytes);
    }


    mod time_formats {
        use super::*;

        // These tests use pattern matching because TimeFormat doesn't
        // implement PartialEq.

        // Default behaviour
        test!(empty:     TimeFormat <- [], None;                            like Ok(TimeFormat::DefaultFormat));

        // Individual settings
        test!(default:   TimeFormat <- ["--time-style=default"], None;      like Ok(TimeFormat::DefaultFormat));
        test!(iso:       TimeFormat <- ["--time-style", "iso"], None;       like Ok(TimeFormat::ISOFormat));
        test!(long_iso:  TimeFormat <- ["--time-style=long-iso"], None;     like Ok(TimeFormat::LongISO));
        test!(full_iso:  TimeFormat <- ["--time-style", "full-iso"], None;  like Ok(TimeFormat::FullISO));

        // Overriding
        test!(actually:  TimeFormat <- ["--time-style=default", "--time-style", "iso"], None;  like Ok(TimeFormat::ISOFormat));
        test!(nevermind: TimeFormat <- ["--time-style", "long-iso", "--time-style=full-iso"], None;  like Ok(TimeFormat::FullISO));

        // Errors — Clap rejects invalid values at parse time
        #[test]
        fn daily() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--time-style=24-hour"]).is_err());
        }

        // `TIME_STYLE` environment variable is defined.
        // If the time-style argument is not given, `TIME_STYLE` is used.
        test!(use_env:     TimeFormat <- [], Some("long-iso".into());  like Ok(TimeFormat::LongISO));

        // If the time-style argument is given, `TIME_STYLE` is overriding.
        test!(override_env:     TimeFormat <- ["--time-style=full-iso"], Some("long-iso".into());  like Ok(TimeFormat::FullISO));
    }


    mod time_types {
        use super::*;

        // Default behaviour
        test!(empty:     TimeTypes <- [];                      Ok(TimeTypes::default()));

        // Modified
        test!(modified:  TimeTypes <- ["--modified"];          Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(m:         TimeTypes <- ["-m"];                  Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(time_mod:  TimeTypes <- ["--time=modified"];     Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
        test!(t_m:       TimeTypes <- ["-tmod"];               Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));

        // Changed
        #[cfg(target_family = "unix")]
        test!(changed:   TimeTypes <- ["--changed"];           Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));
        #[cfg(target_family = "unix")]
        test!(time_ch:   TimeTypes <- ["--time=changed"];      Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));
        #[cfg(target_family = "unix")]
        test!(t_ch:    TimeTypes <- ["-t", "ch"];              Ok(TimeTypes { modified: false, changed: true,  accessed: false, created: false }));

        // Accessed
        test!(acc:       TimeTypes <- ["--accessed"];          Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(a:         TimeTypes <- ["-u"];                  Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(time_acc:  TimeTypes <- ["--time", "accessed"];  Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));
        test!(time_a:    TimeTypes <- ["-t", "acc"];           Ok(TimeTypes { modified: false, changed: false, accessed: true,  created: false }));

        // Created
        test!(cr:        TimeTypes <- ["--created"];           Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(c:         TimeTypes <- ["-U"];                  Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(time_cr:   TimeTypes <- ["--time=created"];      Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));
        test!(t_cr:      TimeTypes <- ["-tcr"];                Ok(TimeTypes { modified: false, changed: false, accessed: false, created: true  }));

        // Multiples
        test!(time_uu:   TimeTypes <- ["-u", "--modified"];    Ok(TimeTypes { modified: true,  changed: false, accessed: true,  created: false }));


        // Errors — Clap rejects invalid values at parse time
        #[test]
        fn time_tea() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--time=tea"]).is_err());
        }
        #[test]
        fn t_ea() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-tea"]).is_err());
        }

        // Overriding
        test!(overridden:   TimeTypes <- ["-tcr", "-tmod"];    Ok(TimeTypes { modified: true,  changed: false, accessed: false, created: false }));
    }


    mod views {
        use super::*;

        use crate::output::grid::Options as GridOptions;


        // Default
        test!(empty:         Mode <- [], None;            like Ok(Mode::Grid(_)));

        // Grid views
        test!(original_g:    Mode <- ["-G"], None;        like Ok(Mode::Grid(GridOptions { across: false, .. })));
        test!(grid:          Mode <- ["--grid"], None;    like Ok(Mode::Grid(GridOptions { across: false, .. })));
        test!(across:        Mode <- ["--across"], None;  like Ok(Mode::Grid(GridOptions { across: true,  .. })));
        test!(gracross:      Mode <- ["-xG"], None;       like Ok(Mode::Grid(GridOptions { across: true,  .. })));

        // Lines views
        test!(lines:         Mode <- ["--oneline"], None;     like Ok(Mode::Lines));
        test!(prima:         Mode <- ["-1"], None;            like Ok(Mode::Lines));

        // Details views
        test!(long:          Mode <- ["--long"], None;    like Ok(Mode::Details(_)));
        test!(ell:           Mode <- ["-l"], None;        like Ok(Mode::Details(_)));

        // Grid-details views
        test!(lid:           Mode <- ["--long", "--grid"], None;  like Ok(Mode::GridDetails(_)));
        test!(leg:           Mode <- ["-lG"], None;               like Ok(Mode::GridDetails(_)));

        // Options that do nothing with --long
        test!(long_across:   Mode <- ["--long", "--across"],   None;  like Ok(Mode::Details(_)));

        // Options that do nothing without --long (no strict mode to catch them)
        test!(just_header:   Mode <- ["--header"],   None;  like Ok(Mode::Grid(_)));
        test!(just_group:    Mode <- ["--group"],    None;  like Ok(Mode::Grid(_)));
        test!(just_inode:    Mode <- ["--inode"],    None;  like Ok(Mode::Grid(_)));
        test!(just_links:    Mode <- ["--links"],    None;  like Ok(Mode::Grid(_)));
        test!(just_blocks:   Mode <- ["--blocks"],   None;  like Ok(Mode::Grid(_)));
        test!(just_binary:   Mode <- ["--binary"],   None;  like Ok(Mode::Grid(_)));
        test!(just_bytes:    Mode <- ["--bytes"],    None;  like Ok(Mode::Grid(_)));
        test!(just_numeric:  Mode <- ["--numeric"],  None;  like Ok(Mode::Grid(_)));

        #[cfg(feature = "git")]
        test!(just_git:      Mode <- ["--git"],    None;  like Ok(Mode::Grid(_)));

        // Contradictions and combinations
        test!(lgo:           Mode <- ["--long", "--grid", "--oneline"], None;  like Ok(Mode::Lines));
        test!(lgt:           Mode <- ["--long", "--grid", "--tree"],    None;  like Ok(Mode::Details(_)));
        test!(tgl:           Mode <- ["--tree", "--grid", "--long"],    None;  like Ok(Mode::Details(_)));
        test!(tlg:           Mode <- ["--tree", "--long", "--grid"],    None;  like Ok(Mode::Details(_)));
        test!(ot:            Mode <- ["--oneline", "--tree"],           None;  like Ok(Mode::Details(_)));
        test!(og:            Mode <- ["--oneline", "--grid"],           None;  like Ok(Mode::Grid(_)));
        test!(tg:            Mode <- ["--tree", "--grid"],              None;  like Ok(Mode::Details(_)));
    }
}
