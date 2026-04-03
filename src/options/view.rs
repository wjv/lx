use crate::fs::feature::xattr;
use crate::options::{flags, OptionsError, NumberSource, Vars};
use crate::options::parser::MatchedFlags;
use crate::output::{View, Mode, TerminalWidth, grid, details};
use crate::output::grid_details::{self, RowThreshold};
use crate::output::file_name::Options as FileStyle;
use crate::output::table::{Column, TimeType, SizeFormat, UserFormat, Options as TableOptions};
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
        let user_format = UserFormat::deduce(matches);
        let columns = deduce_columns(matches, long_count);
        let total_size = matches.has(flags::TOTAL_SIZE) && !matches.has(flags::NO_TOTAL_SIZE);
        Ok(Self { size_format, time_format, user_format, columns, total_size })
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
        // Individual adds and suppression flags still apply.
        apply_individual_adds(matches, &mut columns);
        apply_suppressions(matches, &mut columns);
        return columns;
    }

    // --format: named column set.
    if let Some(fmt_name) = matches.get(flags::FORMAT)
        && let Some(cols) = format_columns(fmt_name) {
            let mut columns = cols;
            apply_individual_adds(matches, &mut columns);
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

    apply_individual_adds(matches, &mut columns);
    apply_timestamp_overrides(matches, &mut columns);
    apply_suppressions(matches, &mut columns);

    columns
}


/// Add columns requested by individual flags (-i, -g, -H, -S, etc.)
/// if not already present.
/// The canonical column ordering.  When a column is added via an
/// individual flag, it is inserted at its canonical position relative
/// to the columns already present — after its nearest canonical
/// predecessor.
const CANONICAL_ORDER: &[Column] = &[
    Column::Inode,
    Column::Octal,
    Column::Permissions,
    Column::Flags,
    Column::HardLinks,
    Column::FileSize,
    Column::Blocks,
    Column::User,
    Column::Group,
    Column::Timestamp(TimeType::Modified),
    Column::Timestamp(TimeType::Changed),
    Column::Timestamp(TimeType::Created),
    Column::Timestamp(TimeType::Accessed),
    Column::VcsStatus,
    Column::VcsRepos,
];

/// Find the canonical insertion position for `col` within `columns`.
///
/// Looks up `col` in `CANONICAL_ORDER`, then finds the last column
/// already present in `columns` that comes *before* `col` in the
/// canonical order.  Inserts after that column.  If no predecessor
/// is present, inserts at position 0 (or appends if `col` is last
/// in the canonical order and nothing follows).
fn canonical_insert_pos(columns: &[Column], col: Column) -> usize {
    let canon_idx = CANONICAL_ORDER.iter()
        .position(|c| *c == col)
        .unwrap_or(CANONICAL_ORDER.len());

    // Find the last column in `columns` whose canonical index is
    // less than `col`'s.  Insert after it.
    let mut best_pos = 0;
    for (i, existing) in columns.iter().enumerate() {
        let existing_idx = CANONICAL_ORDER.iter()
            .position(|c| c == existing)
            .unwrap_or(CANONICAL_ORDER.len());
        if existing_idx < canon_idx {
            best_pos = i + 1;
        }
    }
    best_pos
}

fn apply_individual_adds(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    let adds: &[(bool, Column)] = &[
        (matches.has(flags::INODE),      Column::Inode),
        (matches.has(flags::LINKS),      Column::HardLinks),
        (matches.has(flags::BLOCKS),     Column::Blocks),
        (matches.has(flags::GROUP),      Column::Group),
        (matches.has(flags::OCTAL),      Column::Octal),
        (matches.has(flags::FILE_FLAGS), Column::Flags),
        (matches.has(flags::VCS_STATUS), Column::VcsStatus),
        (matches.has(flags::VCS_REPOS), Column::VcsRepos),
    ];

    for &(enabled, col) in adds {
        if enabled && !columns.contains(&col) {
            let pos = canonical_insert_pos(columns, col);
            columns.insert(pos, col);
        }
    }
}


/// If explicit timestamp flags are given, override the base timestamp set.
fn apply_timestamp_overrides(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    let has_explicit_time = matches.has(flags::MODIFIED) || matches.has(flags::CHANGED)
        || matches.has(flags::ACCESSED) || matches.has(flags::CREATED)
        || matches.get(flags::TIME).is_some();

    if !has_explicit_time {
        return;
    }

    columns.retain(|c| !matches!(c, Column::Timestamp(_)));

    if matches.has(flags::MODIFIED) || matches.get(flags::TIME).is_some_and(|v| v == "modified" || v == "mod") {
        columns.insert(timestamp_insert_pos(columns), Column::Timestamp(TimeType::Modified));
    }
    if matches.has(flags::CHANGED) || matches.get(flags::TIME).is_some_and(|v| v == "changed" || v == "ch") {
        columns.insert(timestamp_insert_pos(columns), Column::Timestamp(TimeType::Changed));
    }
    if matches.has(flags::ACCESSED) || matches.get(flags::TIME).is_some_and(|v| v == "accessed" || v == "acc") {
        columns.insert(timestamp_insert_pos(columns), Column::Timestamp(TimeType::Accessed));
    }
    if matches.has(flags::CREATED) || matches.get(flags::TIME).is_some_and(|v| v == "created" || v == "cr") {
        columns.insert(timestamp_insert_pos(columns), Column::Timestamp(TimeType::Created));
    }
}


/// Apply --no-* suppression flags and --show-* re-enable flags.
fn apply_suppressions(matches: &MatchedFlags, columns: &mut Vec<Column>) {
    if matches.has(flags::NO_PERMISSIONS) && !matches.has(flags::SHOW_PERMISSIONS) {
        columns.retain(|c| *c != Column::Permissions);
    }
    if matches.has(flags::NO_FILESIZE) && !matches.has(flags::SHOW_FILESIZE) {
        columns.retain(|c| *c != Column::FileSize);
    }
    #[cfg(unix)]
    if matches.has(flags::NO_USER) && !matches.has(flags::SHOW_USER) {
        columns.retain(|c| *c != Column::User);
    }
    if matches.has(flags::NO_TIME) {
        columns.retain(|c| !matches!(c, Column::Timestamp(_)));
    }
    #[cfg(unix)]
    if matches.has(flags::NO_INODE)  { columns.retain(|c| *c != Column::Inode); }
    #[cfg(unix)]
    if matches.has(flags::NO_GROUP)  { columns.retain(|c| *c != Column::Group); }
    #[cfg(unix)]
    if matches.has(flags::NO_LINKS)  { columns.retain(|c| *c != Column::HardLinks); }
    #[cfg(unix)]
    if matches.has(flags::NO_BLOCKS) { columns.retain(|c| *c != Column::Blocks); }

    // Re-enable flags
    if matches.has(flags::SHOW_PERMISSIONS) && !columns.contains(&Column::Permissions) {
        columns.insert(0, Column::Permissions);
    }
    if matches.has(flags::SHOW_FILESIZE) && !columns.contains(&Column::FileSize) {
        let pos = columns.iter()
            .position(|c| matches!(c, Column::User | Column::Group | Column::Timestamp(_) | Column::VcsStatus))
            .unwrap_or(columns.len());
        columns.insert(pos, Column::FileSize);
    }
    #[cfg(unix)]
    if matches.has(flags::SHOW_USER) && !columns.contains(&Column::User) {
        let pos = columns.iter()
            .position(|c| matches!(c, Column::Group | Column::Timestamp(_) | Column::VcsStatus))
            .unwrap_or(columns.len());
        columns.insert(pos, Column::User);
    }
}

/// Find the position to insert a timestamp column — after existing
/// timestamps but before `VcsStatus`.
fn timestamp_insert_pos(columns: &[Column]) -> usize {
    // After the last existing timestamp, or before VcsStatus, or at end.
    let last_ts = columns.iter().rposition(|c| matches!(c, Column::Timestamp(_)));
    if let Some(pos) = last_ts {
        return pos + 1;
    }
    columns.iter()
        .position(|c| *c == Column::VcsStatus)
        .unwrap_or(columns.len())
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
            "relative" => Self::Relative,
            s if s.starts_with('+') => Self::Custom(s[1..].to_string()),
            _          => Self::DefaultFormat,
        }
    }
}


impl UserFormat {
    fn deduce(matches: &MatchedFlags) -> Self {
        if matches.has(flags::NUMERIC) { Self::Numeric } else { Self::Name }
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
        fn explicit_accessed() {
            let ts = timestamps(&["-u"], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Accessed)]);
        }

        #[test]
        fn explicit_created() {
            let ts = timestamps(&["-U"], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Created)]);
        }

        #[test]
        fn time_param_modified() {
            let ts = timestamps(&["--time=modified"], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Modified)]);
        }

        #[test]
        fn time_param_accessed() {
            let ts = timestamps(&["-t", "acc"], 1);
            assert_eq!(ts, vec![Column::Timestamp(TimeType::Accessed)]);
        }

        #[test]
        fn multiple_timestamps() {
            let ts = timestamps(&["-u", "--modified"], 1);
            assert!(ts.contains(&Column::Timestamp(TimeType::Modified)));
            assert!(ts.contains(&Column::Timestamp(TimeType::Accessed)));
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

        // Clap rejects invalid --time values
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
