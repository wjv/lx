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

**[format.NAME]**
: Named column layouts for long view.

**[personality.NAME]**
: Named bundles of format, columns, and settings.

**[theme.NAME]**
: Named colour themes for UI elements.

**[extensions.NAME]**
: Named sets of per-extension colours.

**[filenames.NAME]**
: Named sets of per-filename colours.


VERSIONING
==========

Every configuration file must declare a schema version at the top:

    version = "0.2"

This is the **config schema version**, not the lx release version. It
only increments when the config format changes in a way that requires
migration.

| Config version | lx version | Change                                     |
|:--------------:|:----------:|--------------------------------------------|
| *(none)*       | 0.1.x      | Original format with `[defaults]`          |
| `"0.2"`        | 0.2.0+     | Personalities replace `[defaults]`         |

If lx encounters a config file without a version field (or with a
`[defaults]` section), it refuses to load it and prints a message
directing the user to run:

    lx --upgrade-config

This command converts `[defaults]` to `[personality.default]`, adds
`inherits = "default"` to `[personality.lx]`, stamps `version = "0.2"`,
and saves the original as `~/.lxconfig.toml.bak`.


FORMATS
=======

A format defines a column layout for long view. Define formats under
`[format.NAME]`:

    [format.long]
    columns = ["perms", "size", "user", "modified"]

    [format.long2]
    columns = ["perms", "size", "user", "group", "modified", "vcs"]

The `columns` field accepts either a TOML array or a comma-separated
string:

    columns = ["perms", "size", "user", "modified"]
    columns = "perms,size,user,modified"

Three compiled-in formats correspond to the `-l` tier system:

| Tier | Flag   | Format name | Columns                                      |
|:----:|--------|-------------|----------------------------------------------|
| 1    | `-l`   | `long`      | perms, size, user, modified                  |
| 2    | `-ll`  | `long2`     | perms, size, user, group, modified, vcs      |
| 3    | `-lll` | `long3`     | perms, links, size, blocks, user, group, modified, changed, created, accessed, vcs |

Defining a format with a compiled-in name in the config file overrides
the built-in. Custom formats are used via `--format=NAME` or referenced
from personality definitions.

Valid column names
------------------

`perms` (alias `permissions`)
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
: Reference to a named format (looked up in `[format.*]`).

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
`colour` (string: `always`/`auto`/`never`), `colour-scale` (string:
`none`/`16`/`256`), `icons` (string: `always`/`auto`/`never`),
`width` (integer), `absolute` (bool), `hyperlink` (string:
`always`/`auto`/`never`), `quotes` (string: `always`/`auto`/`never`),
`theme` (string: named theme).

Filtering and sorting:

: `all` (bool), `list-dirs` (bool), `level` (integer), `reverse` (bool),
`sort` (string), `group-dirs` (string: `first`/`last`/`none`),
`only-dirs` (bool), `only-files` (bool).

Long view options:

: `binary` (bool), `bytes` (bool), `header` (bool), `inode` (bool),
`links` (bool), `blocks` (bool), `group` (bool), `numeric` (bool),
`time-style` (string: `default`/`iso`/`long-iso`/`full-iso`),
`time` (string), `modified` (bool), `changed` (bool),
`accessed` (bool), `created` (bool), `total-size` (bool),
`extended` (bool), `octal-permissions` (bool).

VCS:

: `vcs` (string: `auto`/`git`/`jj`/`none`), `vcs-status` (bool),
`vcs-ignore` (bool).

Column visibility:

: `permissions` (bool), `filesize` (bool), `user` (bool),
`no-permissions` (bool), `no-filesize` (bool), `no-user` (bool),
`no-time` (bool), `no-icons` (bool), `no-inode` (bool),
`no-group` (bool), `no-links` (bool), `no-blocks` (bool).

Inheritance
-----------

Personalities support single inheritance via `inherits = "NAME"`. The
child's `format` and `columns` replace the parent's entirely. Settings
are merged key-by-key, with the child's values winning on conflict.

Inheritance chains of arbitrary depth are supported. Cycle detection
prevents infinite loops.

A common pattern is to define a shared base personality:

    default ---+---> lx ---+---> ll ---+---> lt
               |           |          \---> la
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

Compiled-in personalities (`ll`, `lll`, `tree`, `ls`) are used as
fallbacks when a name is not defined in the config file.

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

`use-extensions`
: Name of an `[extensions.NAME]` set to apply.

`use-filenames`
: Name of a `[filenames.NAME]` set to apply.

`reset-extensions`
: Boolean. If `true`, discard the built-in file-type extension colour
mappings before applying the theme's own.

UI element keys
---------------

All other keys in a theme section set the colour for a specific UI
element. Each key takes a colour value (see *COLOUR VALUES* below).

**File kinds:**

: `normal`, `directory`, `symlink`, `pipe`, `block-device`,
`char-device`, `socket`, `special`, `executable`.

**Permissions:**

: `perm-user-read`, `perm-user-write`, `perm-user-exec`,
`perm-user-exec-other`, `perm-group-read`, `perm-group-write`,
`perm-group-exec`, `perm-other-read`, `perm-other-write`,
`perm-other-exec`, `perm-special-user`, `perm-special-other`,
`perm-attribute`.

**Size:**

: `size-number-byte`, `size-number-kilo`, `size-number-mega`,
`size-number-giga`, `size-number-huge`, `size-unit-byte`,
`size-unit-kilo`, `size-unit-mega`, `size-unit-giga`, `size-unit-huge`,
`size-number` (bulk: sets all number magnitudes), `size-unit` (bulk:
sets all unit magnitudes), `size-major`, `size-minor`.

**Users:**

: `user-you`, `user-other`, `group-yours`, `group-other`.

**Links:**

: `links`, `links-multi`.

**VCS:**

: `vcs-new`, `vcs-modified`, `vcs-deleted`, `vcs-renamed`,
`vcs-typechange`, `vcs-ignored`, `vcs-conflicted`.

**UI elements:**

: `punctuation`, `date`, `inode`, `blocks`, `header`, `octal`,
`symlink-path`, `control-char`, `broken-symlink`, `broken-overlay`.


EXTENSIONS
==========

Named sets of per-extension colours, referenced from themes via
`use-extensions = "NAME"`. Keys are file extensions (without the leading
dot); values are colour strings.

    [extensions.dev]
    rs = "#ff8700"
    toml = "sandybrown"
    md = "cornflowerblue"
    py = "38;5;33"
    js = "38;5;220"


FILENAMES
=========

Named sets of per-filename colours, referenced from themes via
`use-filenames = "NAME"`. Keys are exact file names; values are colour
strings.

    [filenames.dev]
    Makefile = "bold underline yellow"
    Cargo.toml = "bold #ff8700"
    Dockerfile = "bold deepskyblue"
    README.md = "bold cornflowerblue"


COLOUR VALUES
=============

Colour values in themes, extensions, and filenames accept a
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
   `--theme`). The theme's UI element keys, extension colours, and
   filename colours override all environment variable settings.

Within a theme, `inherits` is resolved first (parent applied, then
child overrides). Extension and filename sets referenced by
`use-extensions` and `use-filenames` are applied after UI element keys.
If `reset-extensions = true`, built-in extension mappings are discarded
before the theme's own extensions are applied.


SEE ALSO
========

lx(1)

**Source code:** `https://github.com/wjv/lx`
