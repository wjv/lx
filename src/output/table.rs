use std::cmp::max;
use std::ops::Deref;
use std::sync::LazyLock;
#[cfg(unix)]
use std::sync::{Mutex, MutexGuard};

use log::*;
#[cfg(unix)]
use uzers::UsersCache;

use crate::fs::{File, fields as f};
use crate::fs::feature::VcsCache;
use crate::output::cell::TextCell;
use crate::output::render::TimeRender;
use crate::output::time::TimeFormat;
use crate::theme::Theme;


/// Options for displaying a table.
#[derive(PartialEq, Eq, Debug)]
pub struct Options {
    pub size_format: SizeFormat,
    pub time_format: TimeFormat,
    pub user_format: UserFormat,
    pub columns: Vec<Column>,
    /// When true, the size column shows recursive directory sizes.
    pub total_size: bool,
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
    Group,
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
        match self {
            Self::Permissions       => "perms",
            Self::FileSize          => "size",
            Self::Timestamp(TimeType::Modified) => "modified",
            Self::Timestamp(TimeType::Changed)  => "changed",
            Self::Timestamp(TimeType::Accessed) => "accessed",
            Self::Timestamp(TimeType::Created)  => "created",
            #[cfg(unix)]
            Self::Blocks            => "blocks",
            #[cfg(unix)]
            Self::User              => "user",
            #[cfg(unix)]
            Self::Group             => "group",
            #[cfg(unix)]
            Self::HardLinks         => "links",
            #[cfg(unix)]
            Self::Inode             => "inode",
            Self::VcsStatus         => "vcs",
            Self::VcsRepos          => "repos",
            #[cfg(unix)]
            Self::Octal             => "octal",
            Self::Flags             => "flags",
        }
    }

    /// Parse a column name from `--columns` or a config file.
    /// Returns `None` for unrecognised names.
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "perms" | "permissions" => Some(Self::Permissions),
            "size" | "filesize"     => Some(Self::FileSize),
            "modified"              => Some(Self::Timestamp(TimeType::Modified)),
            "changed"               => Some(Self::Timestamp(TimeType::Changed)),
            "accessed"              => Some(Self::Timestamp(TimeType::Accessed)),
            "created"               => Some(Self::Timestamp(TimeType::Created)),
            #[cfg(unix)]
            "blocks"                => Some(Self::Blocks),
            #[cfg(unix)]
            "user"                  => Some(Self::User),
            #[cfg(unix)]
            "group"                 => Some(Self::Group),
            #[cfg(unix)]
            "links"                 => Some(Self::HardLinks),
            #[cfg(unix)]
            "inode"                 => Some(Self::Inode),
            "vcs"                   => Some(Self::VcsStatus),
            "repos"                 => Some(Self::VcsRepos),
            #[cfg(unix)]
            "octal"                 => Some(Self::Octal),
            "flags"                 => Some(Self::Flags),
            _                       => None,
        }
    }

    /// Return the canonical name used in config files and `--columns`.
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Permissions           => "perms",
            Self::FileSize              => "size",
            Self::Timestamp(t)          => match t {
                TimeType::Modified => "modified",
                TimeType::Changed  => "changed",
                TimeType::Accessed => "accessed",
                TimeType::Created  => "created",
            },
            #[cfg(unix)]
            Self::Blocks                => "blocks",
            #[cfg(unix)]
            Self::User                  => "user",
            #[cfg(unix)]
            Self::Group                 => "group",
            #[cfg(unix)]
            Self::HardLinks             => "links",
            #[cfg(unix)]
            Self::Inode                 => "inode",
            Self::VcsStatus             => "vcs",
            Self::VcsRepos              => "repos",
            #[cfg(unix)]
            Self::Octal                 => "octal",
            Self::Flags                 => "flags",
        }
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
    #[cfg(unix)]
    pub fn alignment(self) -> Alignment {
        match self {
            Self::FileSize   |
            Self::HardLinks  |
            Self::Inode      |
            Self::Blocks     |
            Self::VcsStatus  => Alignment::Right,
            _                => Alignment::Left,
        }
    }

    #[cfg(windows)]
    pub fn alignment(&self) -> Alignment {
        match self {
            Self::FileSize   |
            Self::VcsStatus  => Alignment::Right,
            _                => Alignment::Left,
        }
    }

    /// Get the text that should be printed at the top, when the user elects
    /// to have a header row printed.
    pub fn header(self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::Permissions   => "Permissions",
            #[cfg(windows)]
            Self::Permissions   => "Mode",
            Self::FileSize      => "Size",
            Self::Timestamp(t)  => t.header(),
            #[cfg(unix)]
            Self::Blocks        => "Blocks",
            #[cfg(unix)]
            Self::User          => "User",
            #[cfg(unix)]
            Self::Group         => "Group",
            #[cfg(unix)]
            Self::HardLinks     => "Links",
            #[cfg(unix)]
            Self::Inode         => "inode",
            Self::VcsStatus     => "VCS",
            Self::VcsRepos      => "Repo",
            #[cfg(unix)]
            Self::Octal         => "Octal",
            Self::Flags         => "Flags",
        }
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

/// Formatting options for user and group.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum UserFormat {
    /// The UID / GID
    Numeric,
    /// Show the name
    Name,
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


pub struct Table<'a> {
    columns: Vec<Column>,
    theme: &'a Theme,
    env: &'a Environment,
    widths: TableWidths,
    time_format: TimeFormat,
    size_format: SizeFormat,
    user_format: UserFormat,
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

        Table {
            theme,
            widths,
            columns,
            vcs,
            env,
            time_format: options.time_format.clone(),
            size_format: options.size_format,
            user_format: options.user_format,
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

    fn permissions_plus(&self, file: &File<'_>, xattrs: bool) -> f::PermissionsPlus {
        f::PermissionsPlus {
            file_type: file.type_char(),
            #[cfg(unix)]
            permissions: file.permissions(),
            #[cfg(windows)]
            attributes: file.attributes(),
            xattrs,
        }
    }

    #[cfg(unix)]
    fn octal_permissions(&self, file: &File<'_>) -> f::OctalPermissions {
        f::OctalPermissions {
            permissions: file.permissions(),
        }
    }

    fn display(&self, file: &File<'_>, column: Column, xattrs: bool) -> TextCell {
        match column {
            Column::Permissions => {
                self.permissions_plus(file, xattrs).render(self.theme)
            }
            Column::FileSize => {
                if self.total_size {
                    file.total_size().render(self.theme, self.size_format, &self.env.numeric)
                } else {
                    file.size().render(self.theme, self.size_format, &self.env.numeric)
                }
            }
            #[cfg(unix)]
            Column::HardLinks => {
                file.links().render(self.theme, &self.env.numeric)
            }
            #[cfg(unix)]
            Column::Inode => {
                file.inode().render(self.theme.ui.inode)
            }
            #[cfg(unix)]
            Column::Blocks => {
                file.blocks().render(self.theme)
            }
            #[cfg(unix)]
            Column::User => {
                file.user().render(self.theme, &*self.env.lock_users(), self.user_format)
            }
            #[cfg(unix)]
            Column::Group => {
                file.group().render(self.theme, &*self.env.lock_users(), self.user_format)
            }
            Column::VcsStatus => {
                let backend = self.vcs.map(super::super::fs::feature::VcsCache::header_name).unwrap_or("VCS");
                self.vcs_status(file).render(self.theme, backend)
            }
            Column::VcsRepos => {
                file.vcs_repo_status().render(self.theme)
            }
            #[cfg(unix)]
            Column::Octal => {
                self.octal_permissions(file).render(self.theme.ui.octal)
            }
            Column::Flags => {
                file.flags().render(self.theme.ui.flags)
            }

            Column::Timestamp(TimeType::Modified)  => {
                file.modified_time().render(self.theme.ui.date, &self.time_format)
            }
            Column::Timestamp(TimeType::Changed)   => {
                file.changed_time().render(self.theme.ui.date, &self.time_format)
            }
            Column::Timestamp(TimeType::Created)   => {
                file.created_time().render(self.theme.ui.date, &self.time_format)
            }
            Column::Timestamp(TimeType::Accessed)  => {
                file.accessed_time().render(self.theme.ui.date, &self.time_format)
            }
        }
    }

    fn vcs_status(&self, file: &File<'_>) -> f::VcsFileStatus {
        debug!("Getting VCS status for file {}", file.path.display());

        self.vcs
            .map(|g| g.get(&file.path, file.is_directory()))
            .unwrap_or_default()
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
