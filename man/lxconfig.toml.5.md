% lxconfig.toml(5)

NAME
====

lxconfig.toml -- configuration file for lx(1)


SYNOPSIS
========

lx searches for a TOML configuration file in these locations (first
found wins):

1. `$LX_CONFIG` -- explicit path override
2. `~/.lxconfig.toml` -- simple home directory location
3. `$XDG_CONFIG_HOME/lx/config.toml` (default `~/.config/lx/config.toml`)
4. `~/Library/Application Support/lx/config.toml` (macOS only)

Run `lx --init-config` to generate a commented starter file at
`~/.lxconfig.toml`.


DESCRIPTION
===========

The lx configuration file is written in TOML. It has the following
top-level sections:

**version**
: Config schema version (required).

**[format]**
: Named column layouts for long view (flat section; each key is a
format name).

**[personality.NAME]**
: Named bundles of format, columns, and settings.

**[theme.NAME]**
: Named colour themes for UI elements.

**[class]**
: File-type class definitions — named lists of glob patterns.

**[style.NAME]**
: Named sets of file colour rules.  Can reference classes by name
and/or match files directly by glob pattern or exact filename.


VERSIONING
==========

Every configuration file must declare a schema version at the top:

    version = "0.5"

This is the **config schema version**, not the lx release version. It
only increments when the config format changes in a way that requires
migration.

| Config version | lx version | Change                                     |
|:--------------:|:----------:|--------------------------------------------|
| *(none)*       | 0.1.x      | Original format with `[defaults]`          |
| `"0.2"`        | 0.2.0+     | Personalities replace `[defaults]`         |
| `"0.3"`        | 0.3.0+     | Classes, flat formats, exa style chain     |
| `"0.4"`        | 0.7.0+     | Conditional overrides (`[[when]]` blocks)  |
| `"0.5"`        | 0.8.0+     | `time` and `numeric` settings removed; timestamps and UID/GID now ordinary columns |
| `"0.6"`        | 0.9.0+     | `[[when]]` env conditions accept TOML arrays and glob patterns |

Versions `"0.3"`, `"0.4"`, and `"0.5"` are still accepted by
lx 0.9+ (0.6 is a strict superset), with caveats: `[[when]]`
blocks in a `"0.3"` config trigger a warning, and the
`time = "..."` and `numeric = ...` settings (valid in 0.3/0.4)
trigger deprecation warnings at load time and are ignored.

If lx encounters a config file with an older version (or no version
field), it refuses to load it and prints a message directing the user
to run:

    lx --upgrade-config

This command migrates the config to the current format (0.1→0.5,
0.2→0.5, 0.3→0.5, and 0.4→0.5 are all supported) and saves the
original as `~/.lxconfig.toml.bak`. The 0.3→0.5 and 0.4→0.5
migrations only bump the version string (and warn if `time = "..."`
or `numeric = ...` are present); earlier migrations also restructure
the file.


FORMATS
=======

A format defines a column layout for long view. Formats are defined as
keys in a flat `[format]` section. Each key is a format name; its value
is either a TOML array or a comma-separated string of column names:

    [format]
    long  = ["permissions", "size", "user", "modified"]
    long2 = ["permissions", "size", "user", "group", "modified", "vcs"]
    long3 = ["permissions", "links", "size", "blocks", "user", "group",
             "modified", "changed", "created", "accessed", "vcs"]
    compact = "permissions,size,modified"

Three compiled-in formats correspond to the `-l` tier system:

| Tier | Flag   | Format name | Columns                                      |
|:----:|--------|-------------|----------------------------------------------|
| 1    | `-l`   | `long`      | permissions, size, user, modified            |
| 2    | `-ll`  | `long2`     | permissions, size, user, group, modified, vcs |
| 3    | `-lll` | `long3`     | permissions, links, size, blocks, user, group, modified, changed, created, accessed, vcs |

Defining a format with a compiled-in name in the config file overrides
the built-in. Custom formats are used via `--format=NAME` or referenced
from personality definitions.

Valid column names
------------------

`permissions` (alias `perms`)
: File permissions.

`size` (alias `filesize`)
: File size.

`user`
: Owning user.

`group`
: Owning group.

`links`
: Hard link count.

`inode`
: Inode number.

`blocks`
: File system block count.

`octal`
: Octal permissions.

`modified`
: Last modification timestamp.

`changed`
: Last status change timestamp.

`accessed`
: Last access timestamp.

`created`
: Creation timestamp.

`vcs`
: VCS status column.


PERSONALITIES
=============

A personality bundles a format, columns, and settings under a name.
Invoke a personality with `--personality=NAME`, `-p NAME`, or via an
argv[0] symlink.

    [personality.ll]
    inherits = "lx"
    format = "long2"
    group-dirs = "first"

Structural fields
-----------------

`inherits`
: Name of a parent personality (see *Inheritance* below).

`format`
: Reference to a named format (looked up in `[format]`).

`columns`
: Inline column list; overrides `format` if both are given. Accepts a
TOML array or comma-separated string.

All other keys in a personality section are **settings** -- each
corresponds to a CLI flag. Boolean flags take `true`/`false`; value
flags take a string or integer.

Setting keys
------------

Display options:

: `oneline` (bool), `long` (bool), `grid` (bool), `across` (bool),
`recurse` (bool), `tree` (bool), `classify` (string: `always`/`auto`/`never`),
`colour` (string: `always`/`auto`/`never`),
`gradient` (string: comma-separated `size`/`date`/`modified`/`accessed`/`changed`/`created`, or `all`/`none`),
`icons` (string: `always`/`auto`/`never`),
`width` (integer), `absolute` (bool), `hyperlink` (string:
`always`/`auto`/`never`), `quotes` (string: `always`/`auto`/`never`),
`theme` (string: named theme), `count` (bool).

Filtering and sorting:

: `all` (bool), `list-dirs` (bool), `level` (integer), `reverse` (bool),
`sort` (string), `group-dirs` (string: `first`/`last`/`none`),
`only-dirs` (bool), `only-files` (bool), `ignore` (string: pipe-separated
globs), `prune` (string: pipe-separated globs),
`symlinks` (string: `show`/`hide`/`follow`).

Long view options:

: `size-style` (string: `decimal`/`binary`/`bytes`),
`binary` (bool), `bytes` (bool), `header` (bool), `inode` (bool),
`links` (bool), `blocks` (bool), `group` (bool), `uid` (bool),
`gid` (bool),
`time-style` (string: `default`/`iso`/`long-iso`/`full-iso`/`relative`/`+FORMAT`),
`modified` (bool), `changed` (bool), `accessed` (bool),
`created` (bool), `total-size` (bool), `extended` (bool),
`octal-permissions` (bool), `flags` (bool),
`permissions` (bool), `filesize` (bool), `user` (bool).

Layout tuning:

: `grid-rows` (integer) — minimum rows before the grid-details view
activates. Equivalent to the `LX_GRID_ROWS` environment variable;
the config key takes precedence.
`icon-spacing` (integer) — number of spaces between an icon and its
filename. Equivalent to `LX_ICON_SPACING`; the config key takes
precedence.

Numeric formatting:

: `decimal-point` (string) — override the locale's decimal separator.
Exactly one character.
`thousands-separator` (string) — override the locale's thousands
separator. Zero or one character; empty string disables grouping.
Both apply to **counts** (file sizes in all `size-style` modes,
`--total-size` totals, `-CZ` summaries, block counts, link counts)
but not to **IDs** (inodes, UID, GID). Always 3-digit grouping.
Setting these in `[personality.default]` makes them global.

Removed in 0.5: `time` (string naming a single timestamp field) and
`numeric` (boolean). The `time` setting is replaced by the individual
`modified`/`changed`/`accessed`/`created` booleans (which are additive
in 0.8+); set `no-time = true` to start from an empty timestamp set.
The `numeric` setting is replaced by the new first-class `uid` and
`gid` columns. For the old `ls -n`-style numeric-only view, use
`uid = true, gid = true, no-user = true, no-group = true` — or better,
use `--columns` to pick exactly the columns you want.

VCS:

: `vcs` (string: `auto`/`git`/`jj`/`none`), `vcs-status` (bool),
`vcs-ignore` (bool), `vcs-repos` (bool).

Negation (override personality defaults):

: `no-permissions` (bool), `no-filesize` (bool), `no-user` (bool),
`no-uid` (bool), `no-gid` (bool), `no-time` (bool),
`no-modified` (bool), `no-changed` (bool), `no-accessed` (bool),
`no-created` (bool), `no-icons` (bool),
`no-inode` (bool), `no-group` (bool), `no-links` (bool),
`no-blocks` (bool), `no-octal` (bool), `no-header` (bool),
`no-count` (bool), `no-total-size` (bool).

`no-time` clears all timestamp columns from the base format as a
bulk shortcut; it runs before individual adds, so combining it
with e.g. `accessed = true` leaves just the accessed column.
`no-modified`/`no-changed`/`no-accessed`/`no-created` suppress
individual timestamp columns after adds have been applied.

Column visibility (positive):

: `permissions` (bool), `filesize` (bool), `user` (bool).

Inheritance
-----------

Personalities support single inheritance via `inherits = "NAME"`. The
child's `format` and `columns` replace the parent's entirely. Settings
are merged key-by-key, with the child's values winning on conflict.

Inheritance chains of arbitrary depth are supported. Cycle detection
prevents infinite loops.

A common pattern is to define a shared base personality:

    default ---+---> lx ---+---> ll ---+---> lt
               |           |           \---> la
               |           \---> lll
               \---> tree

    ls  (standalone -- no inherits)

Example:

    [personality.default]
    colour = "auto"
    time-style = "default"
    group-dirs = "none"
    icons = "never"

    [personality.lx]
    inherits = "default"

    [personality.ll]
    inherits = "lx"
    format = "long2"
    group-dirs = "first"

    [personality.lt]
    inherits = "ll"
    sort = "age"

    [personality.ls]
    grid = true
    across = true

In this example, `lt` inherits `group-dirs = "first"` and
`format = "long2"` from `ll`, which in turn inherits `colour = "auto"`
and the other defaults from `lx` and `default`. The `ls` personality
stands alone -- it has no `inherits` and receives no inherited settings.

Compiled-in personalities (`ll`, `la`, `lll`, `tree`, `ls`) are used as
fallbacks when a name is not defined in the config file.

Conditional overrides
---------------------

A personality's settings can vary based on environment variables using
`[[personality.NAME.when]]` blocks.  Each block specifies conditions
and settings to overlay when all conditions match.

Conditions use `env.VAR = value` where the TOML value type determines
the check:

`env.VAR = "string"`
: Exact string match against the variable's value.  If the string
contains any of `*`, `?`, or `[`, it is matched as a glob pattern
instead (added in 0.6).

`env.VAR = ["string", "string"]`
: TOML array — matches if any element matches.  Each element is
independently matched as a literal or a glob.  Added in 0.6.

`env.VAR = true`
: Variable must be set (to any value, including empty).

`env.VAR = false`
: Variable must be truly unset (not just empty).

All conditions within a single block must match (AND logic).

**Examples:**

    # Icons only in terminals with Nerd Font support.
    [personality.lx]
    icons = "never"

    [[personality.lx.when]]
    env.TERM_PROGRAM = "ghostty"
    icons = "always"

    # Disable colour over SSH.
    [[personality.lx.when]]
    env.SSH_CONNECTION = true
    colour = "never"

    # Auto-select theme tier based on terminal capability.
    # Glob match against TERM, array of accepted COLORTERM values.
    [personality.default]
    theme = "exa"

    [[personality.default.when]]
    env.TERM = "*-256color"
    theme = "lx-256"

    [[personality.default.when]]
    env.COLORTERM = ["truecolor", "24bit"]
    theme = "lx-24bit"

**Evaluation rules:**

- All conditions in a block must match (AND logic).
- Multiple `when` blocks are tried in order; all matching blocks
  apply, with later blocks overriding earlier ones.
- The base personality (without `when`) is the default when no
  block matches.
- `when` blocks are inherited: a parent personality's `when` blocks
  are applied first, then the child's.

Requires `version = "0.4"` or later in the config file.

argv[0] dispatch
----------------

When lx is invoked via a symlink whose name matches a personality, that
personality is applied automatically. For example, if `ll` is a symlink
to `lx`, running `ll` is equivalent to `lx -pll`.


THEMES
======

A theme defines colours for UI elements. Themes are selected through
personalities (`theme = "NAME"`) or the `--theme=NAME` CLI flag.

    [theme.ocean]
    inherits = "exa"
    directory = "bold dodgerblue"
    executable = "bold springgreen"
    symlink = "mediumturquoise"
    date = "steelblue"

Structural fields
-----------------

`inherits`
: Inherit from another theme. The parent's UI keys are applied first;
this theme's keys override. The special name `"exa"` refers to the
compiled-in default theme. Without `inherits`, a theme starts from a
blank slate.

`use-style`
: Name of a `[style.NAME]` set to apply.

UI element keys
---------------

All other keys in a theme section set the colour for a specific UI
element. Each key takes a colour value (see *COLOUR VALUES* below).

**File kinds:**

: `normal`, `directory`, `symlink`, `pipe`, `block-device`,
`char-device`, `socket`, `special`, `executable`.

**Permissions:**

: `permissions-user-read`, `permissions-user-write`,
`permissions-user-execute`, `permissions-user-execute-other`,
`permissions-group-read`, `permissions-group-write`,
`permissions-group-execute`, `permissions-other-read`,
`permissions-other-write`, `permissions-other-execute`,
`permissions-special-user`, `permissions-special-other`,
`permissions-attribute`.

The legacy short form `perm-user-read`, `perm-user-write`,
`perm-user-exec` etc. is also accepted as an alias.

**Size:**

: `size-number-byte`, `size-number-kilo`, `size-number-mega`,
`size-number-giga`, `size-number-huge`, `size-unit-byte`,
`size-unit-kilo`, `size-unit-mega`, `size-unit-giga`, `size-unit-huge`,
`size-number` (bulk: sets all number magnitudes), `size-unit` (bulk:
sets all unit magnitudes), `size-major`, `size-minor`.

**Users:**

: `user-you`, `user-other`, `group-yours`, `group-member`,
`group-other`, `uid-you`, `uid-other`, `gid-yours`, `gid-member`,
`gid-other`.

Group columns distinguish three tiers: `*-yours` (your primary
group), `*-member` (a supplementary group you belong to), and
`*-other` (not your group).  The `member` tier indicates you have
group-permission access to the file.

The `uid-*` and `gid-*` slots style the dedicated `--uid` and
`--gid` columns.  All curated themes and the builtin default set
all identity slots explicitly.

**Links:**

: `links`, `links-multi`.

**VCS:**

: `vcs-new`, `vcs-modified`, `vcs-deleted`, `vcs-renamed`,
`vcs-typechange`, `vcs-ignored`, `vcs-conflicted`.

**UI elements:**

: `punctuation`, `inode`, `blocks`, `header`, `octal`,
`symlink-path`, `control-char`, `broken-symlink`, `broken-overlay`.

**Timestamps (age-based gradient):**

: `date` (bulk setter — sets all tiers at once on every
timestamp column),
`date-now` (< 1 hour), `date-today` (< 24 hours),
`date-week` (< 7 days), `date-month` (< 30 days),
`date-year` (< 365 days), `date-old` (> 1 year),
`date-flat` (the single colour each timestamp column uses
when its gradient is disabled).

Setting `date` alone is backwards compatible — all timestamps
render in the same colour, including the flat fallback.  Setting
individual tiers creates a gradient that shows file age at a
glance, and `date-flat` lets a theme define what colour the
column collapses to when the gradient is turned off.

**Per-timestamp-column overrides:**

: Each of the four timestamp columns (`modified`, `accessed`,
`changed`, `created`) can be themed independently.  For each
column, the same eight slots exist with a `date-<col>-` prefix:

    `date-modified` (bulk for the modified column),
    `date-modified-now`, `date-modified-today`,
    `date-modified-week`, `date-modified-month`,
    `date-modified-year`, `date-modified-old`,
    `date-modified-flat`.

The same eight keys exist for `date-accessed-*`,
`date-changed-*`, and `date-created-*` (32 per-column keys
total).  Per-column overrides write directly to the named
field; only the unprefixed `date-*` keys above fan out to all
four columns at once.

**Order matters.**  `set_config` applies keys in the order they
appear in the theme block, so write the bulk `date = ...`
setter (and any bulk `date-now = ...` etc.) *before*
per-column overrides — otherwise the bulk setters will clobber
them.  Example:

    [theme.example]
    date              = "blue"           # bulk: every column, every tier
    date-now          = "bright cyan"    # bulk: now-tier on every column
    date-modified-now = "bright green"   # only the modified column's now

After `set_config` runs in order, the modified column's `now`
tier is bright green, the other three columns' `now` tiers are
bright cyan, and every other tier on every column is blue.

Per-column overrides are available only via the config file.
The two-letter `LX_COLORS` codes (`da`, `dn`, `dt`, ...) keep
working as bulk setters that fan out to all four columns;
there are no per-column equivalents in `LX_COLORS`.

The compiled-in "exa" theme
---------------------------

The special theme name `"exa"` provides the following defaults.  Use
`inherits = "exa"` and override individual keys to customise:

    [theme.exa]
    # File kinds
    normal = ""
    directory = "bold blue"
    symlink = "cyan"
    pipe = "yellow"
    block-device = "bold yellow"
    char-device = "bold yellow"
    socket = "bold red"
    special = "yellow"
    executable = "bold green"

    # Permissions
    permissions-user-read = "bold yellow"
    permissions-user-write = "bold red"
    permissions-user-execute = "bold underline green"
    permissions-user-execute-other = "bold green"
    permissions-group-read = "yellow"
    permissions-group-write = "red"
    permissions-group-execute = "green"
    permissions-other-read = "yellow"
    permissions-other-write = "red"
    permissions-other-execute = "green"
    permissions-special-user = "purple"
    permissions-special-other = "purple"
    permissions-attribute = ""

    # Size (set per-tier keys size-number-byte..huge for a
    # gradient; see --gradient(1) to switch the gradient on/off)
    size-number = "bold green"
    size-unit = "green"
    size-major = "bold green"
    size-minor = "green"

    # Users and groups
    user-you = "bold yellow"
    user-other = ""
    group-yours = "bold yellow"
    group-other = ""

    # Links
    links = "bold red"
    links-multi = "bold red"

    # VCS status
    vcs-new = "green"
    vcs-modified = "blue"
    vcs-deleted = "red"
    vcs-renamed = "yellow"
    vcs-typechange = "purple"
    vcs-ignored = "dimmed"
    vcs-conflicted = "red"

    # UI elements
    punctuation = "38;5;244"
    date = "blue"
    inode = "purple"
    blocks = "cyan"
    header = "underline"
    octal = "purple"
    symlink-path = "cyan"
    control-char = "red"
    broken-symlink = "red"
    broken-overlay = "underline"


CLASSES
=======

The `[class]` section defines named lists of glob patterns that
represent file-type categories. Classes are referenced from styles
(see below).

    [class]
    source = ["*.rs", "*.py", "*.js", "*.go", "*.c"]
    data   = ["*.csv", "*.json", "*.xml", "*.yaml"]

User-defined classes in `[class]` override any compiled-in
definition of the same name.

Compiled-in class definitions
-----------------------------

lx ships with the following compiled-in classes. To override one,
redefine it in `[class]` with the same name.

    image:
        *.png *.jfi *.jfif *.jif *.jpe *.jpeg *.jpg *.gif *.bmp
        *.tiff *.tif *.ppm *.pgm *.pbm *.pnm *.webp *.raw *.arw
        *.svg *.stl *.eps *.dvi *.ps *.cbr *.jpf *.cbz *.xpm
        *.ico *.cr2 *.orf *.nef *.heif *.avif *.jxl *.j2k *.jp2
        *.j2c *.jpx

    video:
        *.avi *.flv *.m2v *.m4v *.mkv *.mov *.mp4 *.mpeg *.mpg
        *.ogm *.ogv *.vob *.wmv *.webm *.m2ts *.heic

    music:
        *.aac *.m4a *.mp3 *.ogg *.wma *.mka *.opus

    lossless:
        *.alac *.ape *.flac *.wav

    crypto:
        *.asc *.enc *.gpg *.pgp *.sig *.signature *.pfx *.p12

    document:
        *.djvu *.doc *.docx *.dvi *.eml *.eps *.fotd *.key
        *.keynote *.numbers *.odp *.odt *.pages *.pdf *.ppt
        *.pptx *.rtf *.xls *.xlsx

    compressed:
        *.zip *.tar *.Z *.z *.gz *.bz2 *.a *.ar *.7z *.iso *.dmg
        *.tc *.rar *.par *.tgz *.xz *.txz *.lz *.tlz *.lzma
        *.deb *.rpm *.zst *.lz4 *.cpio

    compiled:
        *.class *.elc *.hi *.o *.pyc *.zwc *.ko

    temp:
        *.tmp *.swp *.swo *.swn *.bak *.bkp *.bk

    immediate:
        Makefile Cargo.toml SConstruct CMakeLists.txt build.gradle
        pom.xml Rakefile package.json Gruntfile.js Gruntfile.coffee
        BUILD BUILD.bazel WORKSPACE build.xml Podfile
        webpack.config.js meson.build composer.json RoboFile.php
        PKGBUILD Justfile Procfile Dockerfile Containerfile
        Vagrantfile Brewfile Gemfile Pipfile build.sbt mix.exs
        bsconfig.json tsconfig.json


STYLES
======

Named sets of file colour rules, referenced from themes via
`use-style = "NAME"`.

Styles can reference classes using bare dotted TOML keys, or match
files directly using quoted keys:

`class.NAME = "colour"`
: Reference a named class.  Applies the colour to every pattern in
the class definition.

`"*.ext" = "colour"`
: Glob pattern (quoted key containing metacharacters `*`, `?`, `[`).

`"Makefile" = "colour"`
: Exact filename match (quoted key without metacharacters).

All file pattern keys **must be quoted**. Bare unquoted keys are
reserved for class references.

    [style.exa]
    class.temp       = "38;5;244"
    class.immediate  = "bold underline yellow"
    class.image      = "38;5;133"
    class.video      = "38;5;135"
    class.music      = "38;5;92"
    class.lossless   = "38;5;93"
    class.crypto     = "38;5;109"
    class.document   = "38;5;105"
    class.compressed = "red"
    class.compiled   = "38;5;137"

    [style.dev]
    class.source     = "#ff8700"
    "*.toml"         = "sandybrown"
    "*.md"           = "cornflowerblue"
    "Makefile"       = "bold underline yellow"
    "Cargo.toml"     = "bold #ff8700"

The compiled-in `"exa"` style provides default file-type colouring.
To disable it, use a theme that references a different style (or no
style at all). If two classes have overlapping patterns, the result
is unspecified.


COLOUR VALUES
=============

Colour values in themes and style sets accept a
space-separated string of modifiers and a colour specifier. Tokens may
appear in any order.

**Modifiers:**

: `bold`, `dimmed` (alias `dim`), `italic`, `underline`,
`strikethrough`, `blink`, `reverse`, `hidden`.

**Named ANSI colours:**

: `black`, `red`, `green`, `yellow`, `blue`, `purple` (alias
`magenta`), `cyan`, `white`.

**X11/CSS colour names:**

: The full set of ~148 standard X11 colour names is supported
(case-insensitive). Examples: `tomato`, `cornflowerblue`,
`darkslategray`, `dodgerblue`, `springgreen`, `salmon`, `peru`,
`steelblue`, `wheat`.

**Hex:**

: `#RRGGBB` or `#RGB`. Examples: `#ff8700`, `#f00`.

**256-colour:**

: Raw ANSI 256-colour code: `38;5;NUMBER` (foreground) or
`48;5;NUMBER` (background). Example: `38;5;208`.

**RGB:**

: Raw ANSI true-colour code: `38;2;R;G;B` (foreground) or
`48;2;R;G;B` (background). Example: `38;2;255;135;0`.

Modifiers can be combined with any colour specifier:

    "bold blue"
    "bold underline red"
    "dimmed cyan"
    "bold #ff8700"
    "bold tomato"
    "italic 38;5;208"

A value containing only modifiers (e.g. `"bold underline"`) uses the
terminal's default foreground colour.


PRECEDENCE
==========

Colour settings are resolved in the following order, from lowest to
highest priority:

1. **Built-in defaults** -- the compiled-in exa theme.
2. **LS_COLORS** -- standard file-type colour scheme.
3. **LX_COLORS** -- extended colour scheme (overrides `LS_COLORS`).
4. **Theme** -- config-file theme (selected via personality or
   `--theme`). The theme's UI element keys and style set colours
   override all environment variable settings.

Within a theme, `inherits` is resolved first (parent applied, then
child overrides). Style sets referenced by `use-style` are applied
after UI element keys. The compiled-in "exa" style provides default
file-type colouring; to disable it, use a theme that references a
different style or no style at all.


SEE ALSO
========

lx(1)

**Source code:** `https://github.com/wjv/lx`
