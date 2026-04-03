//! Flag ID constants — string identifiers for Clap arguments.
//!
//! These are the Clap argument IDs that the deduce functions use to query
//! `MatchedFlags`.  Aliases (e.g. `--colour` for `--color`) share the same
//! ID and don't need separate constants.

// display options
pub const ONE_LINE: &str = "oneline";
pub const LONG: &str = "long";
pub const GRID: &str = "grid";
pub const ACROSS: &str = "across";
pub const RECURSE: &str = "recurse";
pub const TREE: &str = "tree";
pub const CLASSIFY: &str = "classify";
pub const COLOR: &str = "color";
pub const COLOR_SCALE: &str = "color-scale";

// filtering and sorting options
pub const ALL: &str = "all";
pub const LIST_DIRS: &str = "list-dirs";
pub const LEVEL: &str = "level";
pub const REVERSE: &str = "reverse";
pub const SORT: &str = "sort";
pub const IGNORE_GLOB: &str = "ignore-glob";
pub const PRUNE: &str = "prune";
pub const SYMLINKS: &str = "symlinks";
pub const GROUP_DIRS: &str = "group-dirs";
pub const DIRS_FIRST: &str = "group-directories-first";
pub const DIRS_LAST: &str = "group-directories-last";
pub const ONLY_DIRS: &str = "only-dirs";
pub const ONLY_FILES: &str = "only-files";

// long view options
pub const BINARY: &str = "binary";
pub const BYTES: &str = "bytes";
pub const GROUP: &str = "group";
pub const NUMERIC: &str = "numeric";
pub const HEADER: &str = "header";
pub const ICONS: &str = "icons";
pub const INODE: &str = "inode";
pub const LINKS: &str = "links";
pub const MODIFIED: &str = "modified";
pub const CHANGED: &str = "changed";
pub const BLOCKS: &str = "blocks";
pub const TIME: &str = "time";
pub const ACCESSED: &str = "accessed";
pub const CREATED: &str = "created";
pub const TIME_STYLE: &str = "time-style";

// explicit column enablers (positive counterparts of --no-*)
pub const SHOW_PERMISSIONS: &str = "show-permissions";
pub const SHOW_FILESIZE: &str = "show-filesize";
pub const SHOW_USER: &str = "show-user";

// suppressing columns
pub const NO_PERMISSIONS: &str = "no-permissions";
pub const NO_FILESIZE: &str = "no-filesize";
pub const NO_USER: &str = "no-user";
pub const NO_TIME: &str = "no-time";
pub const NO_ICONS: &str = "no-icons";
pub const NO_INODE: &str = "no-inode";
pub const NO_GROUP: &str = "no-group";
pub const NO_LINKS: &str = "no-links";
pub const NO_BLOCKS: &str = "no-blocks";
pub const NO_HEADER: &str = "no-header";
pub const NO_OCTAL: &str = "no-octal";
pub const NO_COUNT: &str = "no-count";
pub const NO_TOTAL_SIZE: &str = "no-total-size";

pub const TOTAL_SIZE: &str = "total-size";

// column / format / personality selection
pub const COLUMNS: &str = "columns";
pub const FORMAT: &str = "format";
pub const PERSONALITY: &str = "personality";

// VCS options
pub const VCS: &str = "vcs";
pub const VCS_STATUS: &str = "vcs-status";
pub const VCS_IGNORE: &str = "vcs-ignore";
pub const VCS_REPOS: &str = "vcs-repos";

// theme selection
pub const THEME: &str = "theme";

// display options
pub const WIDTH: &str = "width";
pub const ABSOLUTE: &str = "absolute";
pub const HYPERLINK: &str = "hyperlink";
pub const QUOTES: &str = "quotes";

// output modifiers
pub const COUNT: &str = "count";

// optional feature options
pub const EXTENDED: &str = "extended";
pub const OCTAL: &str = "octal-permissions";
pub const FILE_FLAGS: &str = "flags";
