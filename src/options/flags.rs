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
pub const DOT_ENTRIES: &str = "dot-entries";
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
pub const NO_DIRS_FIRST: &str = "no-dirs-first";
pub const NO_DIRS_LAST: &str = "no-dirs-last";
pub const ONLY_DIRS: &str = "only-dirs";
pub const ONLY_FILES: &str = "only-files";

// long view options
pub const BINARY: &str = "binary";
pub const BYTES: &str = "bytes";
pub const DECIMAL: &str = "decimal";
pub const SIZE_STYLE: &str = "size-style";
pub const GROUP: &str = "group";
pub const UID: &str = "uid";
pub const GID: &str = "gid";
pub const HEADER: &str = "header";
pub const ICONS: &str = "icons";
pub const INODE: &str = "inode";
pub const LINKS: &str = "links";
pub const MODIFIED: &str = "modified";
pub const CHANGED: &str = "changed";
pub const BLOCKS: &str = "blocks";
pub const TIME_TIER: &str = "time-tier";
pub const ACCESSED: &str = "accessed";
pub const CREATED: &str = "created";
pub const TIME_STYLE: &str = "time-style";

// explicit column enablers (positive counterparts of --no-*)
pub const SHOW_PERMISSIONS: &str = "show-permissions";
pub const SHOW_SIZE: &str = "show-size";
pub const SHOW_USER: &str = "show-user";

// suppressing columns
pub const NO_PERMISSIONS: &str = "no-permissions";
pub const NO_SIZE: &str = "no-size";
pub const NO_USER: &str = "no-user";
pub const NO_TIME: &str = "no-time";
pub const NO_MODIFIED: &str = "no-modified";
pub const NO_CHANGED: &str = "no-changed";
pub const NO_ACCESSED: &str = "no-accessed";
pub const NO_CREATED: &str = "no-created";
pub const NO_ICONS: &str = "no-icons";
pub const NO_INODE: &str = "no-inode";
pub const NO_GROUP: &str = "no-group";
pub const NO_UID: &str = "no-uid";
pub const NO_GID: &str = "no-gid";
pub const NO_LINKS: &str = "no-links";
pub const NO_BLOCKS: &str = "no-blocks";
pub const NO_EXTENDED: &str = "no-extended";
pub const XATTR_INDICATOR: &str = "xattr-indicator";
pub const NO_XATTR_INDICATOR: &str = "no-xattr-indicator";
pub const NO_FLAGS: &str = "no-flags";
pub const NO_HEADER: &str = "no-header";
pub const NO_OCTAL: &str = "no-octal";
pub const NO_COUNT: &str = "no-count";
pub const NO_TOTAL: &str = "no-total";
pub const NO_VCS_STATUS: &str = "no-vcs-status";
pub const NO_VCS_REPOS: &str = "no-vcs-repos";

pub const TOTAL: &str = "total";

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

// gradient on/off (per-column)
pub const GRADIENT: &str = "gradient";
pub const NO_GRADIENT: &str = "no-gradient";

// smooth-gradient interpolation (24-bit themes only)
pub const SMOOTH: &str = "smooth";
pub const NO_SMOOTH: &str = "no-smooth";

// display options
pub const WIDTH: &str = "width";
pub const ABSOLUTE: &str = "absolute";
pub const HYPERLINK: &str = "hyperlink";
pub const QUOTES: &str = "quotes";

// output modifiers
pub const COUNT: &str = "count";

// optional feature options
pub const EXTENDED: &str = "extended";
pub const OCTAL: &str = "octal";
pub const FILE_FLAGS: &str = "flags";
