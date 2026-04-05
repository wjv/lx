use crate::fs::feature::xattr;
use crate::options::{flags, OptionsError, NumberSource, Vars};
use crate::options::parser::MatchedFlags;
use crate::output::{View, Mode, TerminalWidth, grid, details};
use crate::output::grid_details::{self, RowThreshold};
use crate::output::file_name::Options as FileStyle;
use crate::output::table::{Column, TimeType, SizeFormat, Options as TableOptions};
use crate::output::time::TimeFormat;


impl View {
    pub fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        let mode = Mode::deduce(matches, vars)?;
        let width = TerminalWidth::deduce(matches, vars)?;
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
        let long_count = matches.count(flags::LONG);
        let has_columns = matches.get(flags::COLUMNS).is_some()
            || matches.get(flags::FORMAT).is_some();
        let long    = long_count > 0 || has_columns;
        let tree    = matches.has(flags::TREE);
        let grid    = matches.has(flags::GRID);
        let oneline = matches.has(flags::ONE_LINE);

        // Tree takes priority over grid when combined with long.
        if long && tree {
            let details = details::Options::deduce_long(matches, vars, long_count)?;
            return Ok(Self::Details(details));
        }

        if long && grid {
            let details = details::Options::deduce_long(matches, vars, long_count)?;
            let grid = grid::Options::deduce(matches);
            let row_threshold = RowThreshold::deduce(vars)?;
            let grid_details = grid_details::Options { grid, details, row_threshold };
            return Ok(Self::GridDetails(grid_details));
        }

        if long {
            let details = details::Options::deduce_long(matches, vars, long_count)?;
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

    fn deduce_long<V: Vars>(matches: &MatchedFlags, vars: &V, long_count: u8) -> Result<Self, OptionsError> {
        Ok(Self {
            table: Some(TableOptions::deduce(matches, vars, long_count)?),
            header: (matches.has(flags::HEADER) || long_count >= 3) && !matches.has(flags::NO_HEADER),
            xattr: xattr::ENABLED && matches.has(flags::EXTENDED),
        })
    }
}


impl TerminalWidth {
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V) -> Result<Self, OptionsError> {
        use crate::options::vars;

        // --width/-w flag takes highest priority.
        if let Some(w) = matches.get_usize(flags::WIDTH) {
            return Ok(Self::Set(w));
        }

        // COLUMNS environment variable.
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

        if let Some(columns) = vars.get(vars::LX_GRID_ROWS).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(rows) => {
                    Ok(Self::MinimumRows(rows))
                }
                Err(e) => {
                    let source = NumberSource::Env(vars::LX_GRID_ROWS);
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
    fn deduce<V: Vars>(matches: &MatchedFlags, vars: &V, long_count: u8) -> Result<Self, OptionsError> {
        let time_format = TimeFormat::deduce(matches, vars)?;
        let size_format = SizeFormat::deduce(matches);
        let columns = deduce_columns(matches, long_count);
        let total_size = matches.has(flags::TOTAL_SIZE) && !matches.has(flags::NO_TOTAL_SIZE);
        Ok(Self { size_format, time_format, columns, total_size })
    }
}


/// Look up a format by name: config-defined first, then compiled-in.
fn format_columns(name: &str) -> Option<Vec<Column>> {
    // Check config-defined formats first.
    if let Some(ref cfg) = *crate::config::CONFIG
        && let Some(columns) = cfg.format.get(name) {
            let cols: Vec<Column> = columns.iter()
                .filter_map(|s| Column::from_name(s))
                .collect();
            if !cols.is_empty() {
                return Some(cols);
            }
        }

    // Compiled-in formats.
    let cols = match name {
        "long" => vec![
            Column::Permissions,
            Column::FileSize,
            #[cfg(unix)]
            Column::User,
            Column::Timestamp(TimeType::Modified),
        ],
        "long2" => vec![
            Column::Permissions,
            Column::FileSize,
            #[cfg(unix)]
            Column::User,
            #[cfg(unix)]
            Column::Group,
            Column::Timestamp(TimeType::Modified),
            Column::VcsStatus,
        ],
        "long3" => vec![
            Column::Permissions,
            #[cfg(unix)]
            Column::HardLinks,
            Column::FileSize,
            #[cfg(unix)]
            Column::Blocks,
            #[cfg(unix)]
            Column::User,
            #[cfg(unix)]
            Column::Group,
            Column::Timestamp(TimeType::Modified),
            Column::Timestamp(TimeType::Changed),
            Column::Timestamp(TimeType::Created),
            Column::Timestamp(TimeType::Accessed),
            Column::VcsStatus,
        ],
        _ => return None,
    };
    Some(cols)
}

/// All available format names: config-defined + compiled-in.
pub fn format_names() -> Vec<String> {
    let mut names: Vec<String> = vec![
        "long".into(), "long2".into(), "long3".into(),
    ];

    if let Some(ref cfg) = *crate::config::CONFIG {
        for name in cfg.format.keys() {
            if !names.iter().any(|n| n == name) {
                names.push(name.clone());
            }
        }
    }

    names
}


/// Build the column list from --columns, --format, the -l tier,
/// individual flags, and positive/negative overrides.
///
/// Precedence: --columns > --format > -l tier > individual flags.
fn deduce_columns(matches: &MatchedFlags, long_count: u8) -> Vec<Column> {
    // --columns: explicit comma-separated column list.
    if let Some(cols_str) = matches.get(flags::COLUMNS) {
        let mut columns = Vec::new();
        for name in cols_str.split(',') {
            let name = name.trim();
            if let Some(col) = Column::from_name(name)
                && !columns.contains(&col) {
                    columns.push(col);
                }
            // Unknown names are silently ignored (could warn in future).
        }
        // Individual adds, `-t` tier, and suppression flags still apply.
        apply_bulk_time_clear(matches, &mut columns);
        apply_individual_adds(matches, &mut columns);
        apply_time_tier(matches, &mut columns);
        apply_suppressions(matches, &mut columns);
        return columns;
    }

    // --format: named column set.
    if let Some(fmt_name) = matches.get(flags::FORMAT)
        && let Some(cols) = format_columns(fmt_name) {
            let mut columns = cols;
            apply_bulk_time_clear(matches, &mut columns);
            apply_individual_adds(matches, &mut columns);
            apply_time_tier(matches, &mut columns);
            apply_suppressions(matches, &mut columns);
            return columns;
        }

    // -l tier: compiled-in format.
    let tier_name = match long_count {
        0 | 1 => "long",
        2     => "long2",
        _     => "long3",
    };
    let mut columns = format_columns(tier_name)
        .expect("compiled-in format always exists");

    apply_bulk_time_clear(matches, &mut columns);
    apply_individual_adds(matches, &mut columns);
    apply_time_tier(matches, &mut columns);
    apply_suppressions(matches, &mut columns);

    columns
}


/// Add columns requested by individual flags (-i, -g, -H, -S, etc.)
/// if not already present.
/// Find the canonical insertion position for `col` within `columns`.
///
/// Uses `canonical_position` from the column registry to determine
/// ordering.  Finds the last column already present in `columns`
/// whose canonical position is less than `col`'s, and inserts after it.
fn canonical_insert_pos(columns: &[Column], col: Column) -> usize {
    use crate::output::column_registry::ColumnDef;

    let canon_pos = ColumnDef::for_column(col).canonical_position;

    let mut best_pos = 0;
    for (i, existing) in columns.iter().enumerate() {
        let existing_pos = ColumnDef::for_column(*existing).canonical_position;
        if existing_pos < canon_pos {
            best_pos = i + 1;
        }
    }
    best_pos
}

/// Add columns whose CLI flags are set, inserting at canonical positions.
/// Driven by the column registry — columns with an `add_flag` are checked.
fn apply_individual_adds(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    use crate::output::column_registry::COLUMN_REGISTRY;

    for def in COLUMN_REGISTRY.iter() {
        if let Some(flag) = def.add_flag {
            if matches.has(flag) && !columns.contains(&def.column) {
                let pos = canonical_insert_pos(columns, def.column);
                columns.insert(pos, def.column);
            }
        }
    }
}


/// `--no-time` clears all timestamp columns the base format brought
/// in.  It runs *before* individual adds and `-t` tiers so that
/// explicit additions (e.g. `--accessed`) survive the clear —
/// the user's intent is "start from no timestamps, then add these".
fn apply_bulk_time_clear(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    if matches.has(flags::NO_TIME) {
        columns.retain(|c| !matches!(c, Column::Timestamp(_)));
    }
}

/// Apply the compounding `-t` timestamp tier.  Counting how many
/// times the flag was passed, add the corresponding timestamp columns
/// at their canonical positions.  `-t` adds modified, `-tt` adds
/// modified + changed, `-ttt` adds all four.  Like other adds,
/// columns that are already present are left alone.
fn apply_time_tier(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    let tier = matches.count(flags::TIME_TIER);
    if tier == 0 {
        return;
    }

    let to_add: &[TimeType] = match tier {
        1 => &[TimeType::Modified],
        2 => &[TimeType::Modified, TimeType::Changed],
        _ => &[
            TimeType::Modified,
            TimeType::Changed,
            TimeType::Created,
            TimeType::Accessed,
        ],
    };

    for tt in to_add {
        let col = Column::Timestamp(*tt);
        if !columns.contains(&col) {
            let pos = canonical_insert_pos(columns, col);
            columns.insert(pos, col);
        }
    }
}


/// Apply --no-* suppression flags and --show-* re-enable flags.
/// Driven by the column registry — columns with suppress/show flags
/// are checked automatically.
///
/// Note: `--no-time` is handled earlier in the pipeline by
/// `apply_bulk_time_clear` so that explicit timestamp adds survive
/// it.  Per-timestamp `--no-modified`/`--no-changed`/etc. are normal
/// registry-driven suppressions that run here.
fn apply_suppressions(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    use crate::output::column_registry::COLUMN_REGISTRY;

    // Registry-driven suppressions.
    for def in COLUMN_REGISTRY.iter() {
        if let Some(flag) = def.suppress_flag {
            let show_overrides = def.show_flag.is_some_and(|sf| matches.has(sf));
            if matches.has(flag) && !show_overrides {
                columns.retain(|c| *c != def.column);
            }
        }
    }

    // Registry-driven re-enables.
    for def in COLUMN_REGISTRY.iter() {
        if let Some(flag) = def.show_flag {
            if matches.has(flag) && !columns.contains(&def.column) {
                let pos = canonical_insert_pos(columns, def.column);
                columns.insert(pos, def.column);
            }
        }
    }
}

impl SizeFormat {

    /// Determine which file size to use in the file size column based on
    /// the user's options.
    ///
    /// The default is decimal prefixes (k, M, G).  Three ways to change it:
    ///
    /// 1. `--size-style=decimal|binary|bytes` — the canonical valued flag.
    /// 2. `--binary` / `-b` — alias for `--size-style=binary`.
    /// 3. `--bytes` / `-B` — alias for `--size-style=bytes`.
    ///
    /// `--size-style` takes precedence when combined with `--binary` or
    /// `--bytes`.  Between `--binary` and `--bytes`, Clap's `overrides_with`
    /// ensures the last one on the command line wins.
    fn deduce(matches: &MatchedFlags) -> Self {
        if let Some(w) = matches.get(flags::SIZE_STYLE) {
            return Self::from_str(w);
        }

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

    fn from_str(word: &str) -> Self {
        match word {
            "binary"  => Self::BinaryBytes,
            "bytes"   => Self::JustBytes,
            "decimal" => Self::DecimalBytes,
            _         => Self::DecimalBytes,
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
            "relative" => Self::Relative,
            s if s.starts_with('+') => Self::Custom(s[1..].to_string()),
            _          => Self::DefaultFormat,
        }
    }
}


// TimeTypes::deduce() has been replaced by deduce_columns() above,
// which builds timestamps directly into the Vec<Column>.


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

        // --size-style valued flag
        test!(style_decimal: SizeFormat <- ["--size-style=decimal"]; SizeFormat::DecimalBytes);
        test!(style_binary:  SizeFormat <- ["--size-style=binary"];  SizeFormat::BinaryBytes);
        test!(style_bytes:   SizeFormat <- ["--size-style=bytes"];   SizeFormat::JustBytes);

        // --size-style overrides last-wins
        test!(style_override: SizeFormat <- ["--size-style=binary", "--size-style=bytes"]; SizeFormat::JustBytes);

        // Legacy boolean flags still work
        test!(binary:  SizeFormat <- ["--binary"];             SizeFormat::BinaryBytes);
        test!(bytes:   SizeFormat <- ["--bytes"];              SizeFormat::JustBytes);

        // Legacy overriding
        test!(both_1:  SizeFormat <- ["--binary", "--binary"]; SizeFormat::BinaryBytes);
        test!(both_2:  SizeFormat <- ["--bytes",  "--binary"]; SizeFormat::BinaryBytes);
        test!(both_3:  SizeFormat <- ["--binary", "--bytes"];  SizeFormat::JustBytes);
        test!(both_4:  SizeFormat <- ["--bytes",  "--bytes"];  SizeFormat::JustBytes);

        // --size-style takes precedence over legacy flags
        test!(style_beats_binary: SizeFormat <- ["--binary", "--size-style=decimal"]; SizeFormat::DecimalBytes);
        test!(style_beats_bytes:  SizeFormat <- ["--bytes",  "--size-style=binary"];  SizeFormat::BinaryBytes);
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

        // New formats
        test!(relative:  TimeFormat <- ["--time-style=relative"], None;  like Ok(TimeFormat::Relative));

        test!(custom:    TimeFormat <- ["--time-style=+%Y-%m-%d"], None;  like Ok(TimeFormat::Custom(_)));

        // Unknown values fall back to default
        test!(unknown:   TimeFormat <- ["--time-style=24-hour"], None;  like Ok(TimeFormat::DefaultFormat));

        // `TIME_STYLE` environment variable is defined.
        // If the time-style argument is not given, `TIME_STYLE` is used.
        test!(use_env:     TimeFormat <- [], Some("long-iso".into());  like Ok(TimeFormat::LongISO));

        // If the time-style argument is given, `TIME_STYLE` is overriding.
        test!(override_env:     TimeFormat <- ["--time-style=full-iso"], Some("long-iso".into());  like Ok(TimeFormat::FullISO));
    }


    mod columns {
        use crate::options::test::parse_for_test;
        use crate::output::table::{Column, TimeType};
        use super::deduce_columns;

        /// Helper: parse flags and return the timestamp columns present.
        fn timestamps(inputs: &[&str], long_count: u8) -> Vec<Column> {
            parse_for_test(inputs, |mf| deduce_columns(mf, long_count))
                .into_iter().next().unwrap()
                .into_iter()
                .filter(|c| matches!(c, Column::Timestamp(_)))
                .collect()
        }

        #[test]
        fn default_has_modified() {
            let ts = timestamps(&[], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Modified)]);
        }

        #[test]
        fn explicit_modified() {
            let ts = timestamps(&["--modified"], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Modified)]);
        }

        #[test]
        fn explicit_accessed_composes() {
            // `-l --accessed` now adds accessed on top of the base
            // `long` format's modified, rather than replacing.
            let ts = timestamps(&["--accessed"], 1);
            assert_eq!(ts, vec![
                Column::Timestamp(TimeType::Modified),
                Column::Timestamp(TimeType::Accessed),
            ]);
        }

        #[test]
        fn explicit_created_composes() {
            let ts = timestamps(&["--created"], 1);
            assert_eq!(ts, vec![
                Column::Timestamp(TimeType::Modified),
                Column::Timestamp(TimeType::Created),
            ]);
        }

        #[test]
        fn multiple_individual_timestamps() {
            let ts = timestamps(&["--accessed", "--modified"], 1);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Accessed)));
        }

        #[test]
        fn time_tier_1_adds_modified() {
            // -t on top of long2 (which has no timestamps in this test context)
            // should ensure modified is present.  In -l (long) it's already
            // there, so `-t` is a no-op in that case.
            let ts = timestamps(&["-t"], 1);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
        }

        #[test]
        fn time_tier_2_adds_changed() {
            let ts = timestamps(&["-tt"], 1);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Changed)));
        }

        #[test]
        fn time_tier_3_adds_all() {
            let ts = timestamps(&["-ttt"], 1);
            assert_eq!(ts.len(), 4);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Changed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Accessed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Created)));
        }

        #[test]
        fn time_tier_composes_with_long2() {
            // `long2` has no timestamps beyond modified; `-tt` should
            // add changed while leaving the rest of the format intact.
            let ts = timestamps(&["-tt"], 2);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Changed)));
        }

        #[test]
        fn tier3_all_timestamps() {
            let ts = timestamps(&[], 3);
            assert_eq!(ts.len(), 4);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Changed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Accessed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Created)));
        }

        #[test]
        fn no_time_suppresses_all() {
            let ts = timestamps(&["--no-time"], 3);
            assert!(ts.is_empty());
        }

        #[test]
        fn no_modified_suppresses_only_modified() {
            let ts = timestamps(&["--no-modified"], 3);
            assert!(!ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Changed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Accessed)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Created)));
        }

        #[test]
        fn tier2_has_vcs_and_group() {
            let cols: Vec<Column> = parse_for_test(&[], |mf| deduce_columns(mf, 2))
                .into_iter().next().unwrap();
            assert!(cols.contains(&Column::VcsStatus));
            assert!(cols.contains(&Column::Group));
        }

        #[test]
        fn tier3_has_links_and_blocks() {
            let cols: Vec<Column> = parse_for_test(&[], |mf| deduce_columns(mf, 3))
                .into_iter().next().unwrap();
            assert!(cols.contains(&Column::HardLinks));
            assert!(cols.contains(&Column::Blocks));
        }

        #[test]
        fn no_group_suppresses() {
            let cols: Vec<Column> = parse_for_test(&["--no-group"], |mf| deduce_columns(mf, 2))
                .into_iter().next().unwrap();
            assert!(!cols.contains(&Column::Group));
        }

        // `--time=X` is removed — the flag no longer exists.
        #[test]
        fn time_equals_rejected() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--time=modified"]).is_err());
        }

        // `-u` is now the short flag for --user (reassigned from --accessed
        // in batch A, attached in batch B).
        #[test]
        fn short_u_is_user() {
            let cmd = crate::options::parser::build_command();
            let m = cmd.try_get_matches_from(["lx", "-l", "-u"])
                .expect("-u should parse as --user");
            assert!(m.get_count(crate::options::flags::SHOW_USER) > 0);
        }

        // `-U` is still unused after batch A: clap should reject it until
        // a future batch reassigns it.
        #[test]
        fn short_upper_u_rejected() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-U"]).is_err());
        }

        // `-z` / `--filesize` / `--size` all parse as the filesize enabler.
        #[test]
        fn short_z_is_filesize() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "-z"]).is_ok());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--size"]).is_ok());
        }

        // `-M` / `--permissions` / `--mode` all parse as the permissions enabler.
        #[test]
        fn short_upper_m_is_permissions() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "-M"]).is_ok());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--mode"]).is_ok());
        }

        // Hidden short-letter negation aliases: --no-u, --no-z, --no-M.
        #[test]
        fn hidden_negation_aliases() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--no-u"]).is_ok());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--no-z"]).is_ok());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--no-M"]).is_ok());
        }

        // Long-form negation aliases: --no-mode, --no-size.
        #[test]
        fn long_negation_aliases() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--no-mode"]).is_ok());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-l", "--no-size"]).is_ok());
        }

        // Verify suppression actually removes the column.
        #[test]
        fn no_z_suppresses_filesize() {
            let cols: Vec<Column> = parse_for_test(&["--no-z"], |mf| deduce_columns(mf, 1))
                .into_iter().next().unwrap();
            assert!(!cols.contains(&Column::FileSize));
        }

        #[test]
        fn no_upper_m_suppresses_permissions() {
            let cols: Vec<Column> = parse_for_test(&["--no-M"], |mf| deduce_columns(mf, 1))
                .into_iter().next().unwrap();
            assert!(!cols.contains(&Column::Permissions));
        }

        #[cfg(unix)]
        #[test]
        fn no_u_suppresses_user() {
            let cols: Vec<Column> = parse_for_test(&["--no-u"], |mf| deduce_columns(mf, 1))
                .into_iter().next().unwrap();
            assert!(!cols.contains(&Column::User));
        }

        // Batch C: --uid and --gid as first-class columns.

        #[cfg(unix)]
        #[test]
        fn uid_adds_column_after_user() {
            let cols: Vec<Column> = parse_for_test(&["--uid"], |mf| deduce_columns(mf, 1))
                .into_iter().next().unwrap();
            assert!(cols.contains(&Column::Uid));
            // Canonical position: uid sits immediately after user.
            let user_idx = cols.iter().position(|c| *c == Column::User);
            let uid_idx = cols.iter().position(|c| *c == Column::Uid);
            assert!(user_idx.is_some() && uid_idx.is_some());
            assert_eq!(uid_idx.unwrap(), user_idx.unwrap() + 1);
        }

        #[cfg(unix)]
        #[test]
        fn gid_adds_column_after_group() {
            let cols: Vec<Column> = parse_for_test(&["-ll", "--gid"], |mf| deduce_columns(mf, 2))
                .into_iter().next().unwrap();
            assert!(cols.contains(&Column::Gid));
            let group_idx = cols.iter().position(|c| *c == Column::Group);
            let gid_idx = cols.iter().position(|c| *c == Column::Gid);
            assert!(group_idx.is_some() && gid_idx.is_some());
            assert_eq!(gid_idx.unwrap(), group_idx.unwrap() + 1);
        }

        #[cfg(unix)]
        #[test]
        fn uid_and_gid_compose() {
            // Adding both alongside user/group gives all four columns,
            // in canonical order: user, uid, group, gid.
            let cols: Vec<Column> = parse_for_test(
                &["-ll", "--uid", "--gid"],
                |mf| deduce_columns(mf, 2),
            ).into_iter().next().unwrap();
            let positions: Vec<usize> = [Column::User, Column::Uid, Column::Group, Column::Gid]
                .iter()
                .map(|c| cols.iter().position(|x| x == c).expect("column present"))
                .collect();
            assert_eq!(positions, (0..4).map(|i| positions[0] + i).collect::<Vec<_>>());
        }

        #[cfg(unix)]
        #[test]
        fn no_uid_suppresses() {
            let cols: Vec<Column> = parse_for_test(
                &["--uid", "--no-uid"],
                |mf| deduce_columns(mf, 1),
            ).into_iter().next().unwrap();
            assert!(!cols.contains(&Column::Uid));
        }

        // `--numeric` and `-n` are gone — clap should reject them.
        #[test]
        fn numeric_removed() {
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "--numeric"]).is_err());
            let cmd = crate::options::parser::build_command();
            assert!(cmd.try_get_matches_from(["lx", "-n"]).is_err());
        }

        #[cfg(unix)]
        #[test]
        fn uid_column_from_name() {
            assert_eq!(Column::from_name("uid"), Some(Column::Uid));
            assert_eq!(Column::from_name("gid"), Some(Column::Gid));
        }
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
        test!(just_uid:      Mode <- ["--uid"],      None;  like Ok(Mode::Grid(_)));
        test!(just_gid:      Mode <- ["--gid"],      None;  like Ok(Mode::Grid(_)));

        #[cfg(feature = "git")]
        test!(just_vcs_status: Mode <- ["--vcs-status"],   None;  like Ok(Mode::Grid(_)));

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
