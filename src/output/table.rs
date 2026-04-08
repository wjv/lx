use std::cmp::max;
use std::ops::Deref;
use std::sync::LazyLock;
#[cfg(unix)]
use std::sync::{Mutex, MutexGuard};

#[cfg(unix)]
use uzers::UsersCache;

use crate::fs::File;
use crate::fs::feature::VcsCache;
use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;
use crate::theme::Theme;


/// Options for displaying a table.
#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub size_format: SizeFormat,
    pub time_format: TimeFormat,
    pub columns: Vec<Column>,
    /// When true, the size column shows recursive directory sizes.
    pub total_size: bool,
    /// Override the locale's decimal separator (e.g. ".").
    pub decimal_point: Option<String>,
    /// Override the locale's thousands separator (e.g. ",").  Empty = no grouping.
    pub thousands_separator: Option<String>,
}


/// A table contains these.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Column {
    Permissions,
    FileSize,
    Timestamp(TimeType),
    #[cfg(unix)]
    Blocks,
    #[cfg(unix)]
    User,
    #[cfg(unix)]
    Uid,
    #[cfg(unix)]
    Group,
    #[cfg(unix)]
    Gid,
    #[cfg(unix)]
    HardLinks,
    #[cfg(unix)]
    Inode,
    VcsStatus,
    VcsRepos,
    #[cfg(unix)]
    Octal,
    Flags,
}

impl Column {
    /// The canonical name used in `--columns` and config files.
    pub fn name(self) -> &'static str {
        super::column_registry::ColumnDef::for_column(self).name
    }

    /// Parse a column name from `--columns` or a config file.
    /// Returns `None` for unrecognised names.
    pub fn from_name(s: &str) -> Option<Self> {
        super::column_registry::ColumnDef::column_from_name(s)
    }

    /// Return the canonical name used in config files and `--columns`.
    pub fn to_name(self) -> &'static str {
        self.name()
    }
}

/// Each column can pick its own **Alignment**. Usually, numbers are
/// right-aligned, and text is left-aligned.
#[derive(Copy, Clone)]
pub enum Alignment {
    Left,
    Right,
}

impl Column {

    /// Get the alignment this column should use.
    pub fn alignment(self) -> Alignment {
        super::column_registry::ColumnDef::for_column(self).alignment
    }

    /// Get the text that should be printed at the top, when the user elects
    /// to have a header row printed.
    pub fn header(self) -> &'static str {
        super::column_registry::ColumnDef::for_column(self).header
    }
}


/// Formatting options for file sizes.
#[allow(clippy::enum_variant_names)]
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[derive(Default)]
pub enum SizeFormat {

    /// Format the file size using **decimal** prefixes, such as “kilo”,
    /// “mega”, or “giga”.
    #[default]
    DecimalBytes,

    /// Format the file size using **binary** prefixes, such as “kibi”,
    /// “mebi”, or “gibi”.
    BinaryBytes,

    /// Do no formatting and just display the size as a number of bytes.
    JustBytes,
}

/// The types of a file’s time fields. These three fields are standard
/// across most (all?) operating systems.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TimeType {

    /// The file’s modified time (`st_mtime`).
    Modified,

    /// The file’s changed time (`st_ctime`)
    Changed,

    /// The file’s accessed time (`st_atime`).
    Accessed,

    /// The file’s creation time (`btime` or `birthtime`).
    Created,
}

impl TimeType {

    /// Returns the text to use for a column’s heading in the columns output.
    pub fn header(self) -> &'static str {
        match self {
            Self::Modified  => "Date Modified",
            Self::Changed   => "Date Changed",
            Self::Accessed  => "Date Accessed",
            Self::Created   => "Date Created",
        }
    }
}




/// The **environment** struct contains any data that could change between
/// running instances of lx, depending on the user’s computer’s configuration.
///
/// Any environment field should be able to be mocked up for test runs.
pub struct Environment {

    /// Localisation rules for formatting numbers.
    pub numeric: locale::Numeric,

    /// Mapping cache of user IDs to usernames.
    #[cfg(unix)]
    users: Mutex<UsersCache>,
}

impl Environment {
    #[cfg(unix)]
    pub fn lock_users(&self) -> MutexGuard<'_, UsersCache> {
        self.users.lock().unwrap()
    }

    fn load_all() -> Self {
        let numeric = locale::Numeric::load_user_locale()
                             .unwrap_or_else(|_| locale::Numeric::english());

        #[cfg(unix)]
        let users = Mutex::new(UsersCache::new());

        Self { numeric, #[cfg(unix)] users }
    }
}

static ENVIRONMENT: LazyLock<Environment> = LazyLock::new(Environment::load_all);

/// Access the process-wide shared Environment.  Used by the sort
/// comparator to resolve user/group names during `-s user`/`-s group`
/// comparisons.
pub fn environment() -> &'static Environment {
    &ENVIRONMENT
}


pub struct Table<'a> {
    columns: Vec<Column>,
    theme: &'a Theme,
    env: &'a Environment,
    numeric: locale::Numeric,
    widths: TableWidths,
    time_format: TimeFormat,
    size_format: SizeFormat,
    total_size: bool,
    vcs: Option<&'a dyn VcsCache>,
}

#[derive(Clone)]
pub struct Row {
    cells: Vec<TextCell>,
}

impl<'a, 'f> Table<'a> {
    pub fn total_size_active(&self) -> bool {
        self.total_size
    }

    pub fn new(options: &'a Options, vcs: Option<&'a dyn VcsCache>, theme: &'a Theme) -> Table<'a> {
        // Filter out VcsStatus column if no VCS cache is available.
        let columns: Vec<Column> = options.columns.iter()
            .copied()
            .filter(|c| !matches!(c, Column::VcsStatus) || vcs.is_some())
            .collect();
        let widths = TableWidths::zero(columns.len());
        let env = &*ENVIRONMENT;

        // Start with the system locale, then apply personality overrides.
        let mut numeric = env.numeric.clone();
        if let Some(ref dp) = options.decimal_point {
            numeric.decimal_sep = dp.clone();
        }
        if let Some(ref ts) = options.thousands_separator {
            numeric.thousands_sep = ts.clone();
        }

        Table {
            theme,
            widths,
            columns,
            vcs,
            env,
            numeric,
            time_format: options.time_format.clone(),
            size_format: options.size_format,
            total_size: options.total_size,
        }
    }

    pub fn widths(&self) -> &TableWidths {
        &self.widths
    }

    pub fn header_row(&self) -> Row {
        let cells = self.columns.iter()
                        .map(|c| {
                            let name = if *c == Column::VcsStatus {
                                self.vcs.map(super::super::fs::feature::VcsCache::header_name).unwrap_or("VCS")
                            } else {
                                c.header()
                            };
                            TextCell::paint_str(self.theme.ui.header, name)
                        })
                        .collect();

        Row { cells }
    }

    pub fn row_for_file(&self, file: &File<'_>, xattrs: bool) -> Row {
        let cells = self.columns.iter()
                        .map(|c| self.display(file, *c, xattrs))
                        .collect();

        Row { cells }
    }

    pub fn add_widths(&mut self, row: &Row) {
        self.widths.add_widths(row);
    }

    fn display(&self, file: &File<'_>, column: Column, xattrs: bool) -> TextCell {
        use super::column_registry::{ColumnDef, RenderContext};
        let def = ColumnDef::for_column(column);
        let ctx = RenderContext {
            theme: self.theme,
            size_format: self.size_format,
            time_format: &self.time_format,
            env: self.env,
            numeric: &self.numeric,
            vcs: self.vcs,
            total_size: self.total_size,
        };
        (def.render)(&ctx, file, xattrs)
    }

    pub fn render(&self, row: Row) -> TextCell {
        let mut cell = TextCell::default();

        let iter = row.cells.into_iter()
                      .zip(self.widths.iter())
                      .enumerate();

        for (n, (this_cell, width)) in iter {
            let padding = width - *this_cell.width;

            match self.columns[n].alignment() {
                Alignment::Left => {
                    cell.append(this_cell);
                    cell.add_spaces(padding);
                }
                Alignment::Right => {
                    cell.add_spaces(padding);
                    cell.append(this_cell);
                }
            }

            cell.add_spaces(1);
        }

        cell
    }
}


pub struct TableWidths(Vec<usize>);

impl Deref for TableWidths {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TableWidths {
    pub fn zero(count: usize) -> Self {
        Self(vec![0; count])
    }

    pub fn add_widths(&mut self, row: &Row) {
        for (old_width, cell) in self.0.iter_mut().zip(row.cells.iter()) {
            *old_width = max(*old_width, *cell.width);
        }
    }

    pub fn total(&self) -> usize {
        self.0.len() + self.0.iter().sum::<usize>()
    }
}
