% lx(1)

NAME
====

lx — a modern file lister


SYNOPSIS
========

`lx [options] [files...]`

**lx** is a modern replacement for `ls`. It uses colours for information
by default, helping you distinguish between many types of files. It has
extra features not present in the original `ls`, such as viewing VCS
status for a directory, recursing into directories with a tree view, and
compounding detail levels with `-l`/`-ll`/`-lll`.


DISPLAY OPTIONS
===============

`-1`, `--oneline`
: Display one entry per line.

`-l`, `--long`
: Display extended file metadata as a table. Repeat for more detail:
`-l` (basic), `-ll` (+ group, VCS), `-lll` (+ header, all timestamps,
links, blocks).

`-G`, `--grid`
: Display entries as a grid (default).

`-x`, `--across`
: Sort the grid across, rather than downwards.

`-R`, `--recurse`
: Recurse into directories.

`-T`, `--tree`
: Recurse into directories as a tree.

`-L`, `--level=DEPTH`
: Limit the depth of recursion.

`--classify`[`=WHEN`]
: Display file kind indicators next to file names. WHEN is `always`,
`auto`, or `never`.

`--colour=WHEN`
: When to use terminal colours. WHEN is `always`, `auto`, or `never`.
Alias: `--color`.

`--colour-scale`
: Colour file sizes on a scale. Alias: `--color-scale`.

`--icons`[`=WHEN`]
: Display icons next to file names. WHEN is `always`, `auto`, or `never`.

`--no-icons`
: Don't display icons. Alias for `--icons=never`.


FILTERING AND SORTING OPTIONS
=============================

`-a`, `--all`
: Show hidden and dot files. Use twice (`-aa`) to also show `.` and `..`.

`-d`, `--list-dirs`
: List directories as regular files.

`-D`, `--only-dirs`
: List only directories, not files.

`-f`, `--only-files`
: List only regular files, not directories.

`-r`, `--reverse`
: Reverse the sort order.

`-s`, `--sort=FIELD`
: Which field to sort by.

Valid sort fields are '`name`', '`Name`', '`extension`', '`Extension`',
'`size`', '`modified`', '`changed`', '`accessed`', '`created`',
'`inode`', '`type`', and '`none`'.

The '`modified`' sort field has the aliases '`date`', '`time`', '`mod`',
and '`newest`'.  Its reverse order has the aliases '`age`', '`old`',
and '`oldest`'.  Other aliases: '`ext`' for `extension`, '`Ext`' for
`Extension`, '`ch`' for `changed`, '`acc`' for `accessed`, '`cr`' for
`created`.

Sort fields starting with a capital letter sort uppercase before
lowercase: 'A' then 'B' then 'a' then 'b'.  Fields starting with a
lowercase letter mix them: 'A' then 'a' then 'B' then 'b'.

With `-Z`/`--total-size`, sorting by `size` uses the recursive
directory total instead of the inode size.

`-I`, `--ignore-glob=GLOBS`
: Glob patterns, pipe-separated, of files to ignore.

`--group-dirs=WHEN`
: Group directories before or after other files. WHEN is `first`, `last`,
or `none`. Legacy alias: `--group-directories-first`.


LONG VIEW OPTIONS
=================

These options affect the columns displayed in long view (`-l`):

`-b`, `--binary`
: List file sizes with binary prefixes (KiB, MiB).

`-B`, `--bytes`
: List file sizes in bytes, without prefixes.

`-g`, `--group`
: List each file's group.

`-h`, `--header`
: Add a header row to each column.

`-H`, `--links`
: List each file's number of hard links.

`-i`, `--inode`
: List each file's inode number.

`-m`, `--modified`
: Show the modified timestamp.

`-c`, `--changed`
: Show the changed timestamp.

`-u`, `--accessed`
: Show the accessed timestamp.

`-U`, `--created`
: Show the created timestamp.

`-t`, `--time=FIELD`
: Which timestamp field to display. Fields: `modified`, `changed`,
`accessed`, `created`.

`--time-style=STYLE`
: How to format timestamps. Styles: `default`, `iso`, `long-iso`, `full-iso`.

`-n`, `--numeric`
: List numeric user and group IDs.

`-S`, `--blocks`
: List each file's number of file system blocks.

`-Z`, `--total-size`
: Show total recursive size for directories. When combined with
`--sort=size`, sorting uses the recursive total.

`-@`, `--extended`
: List each file's extended attributes and sizes.

`--octal-permissions`
: List each file's permissions in octal format.


COLUMN VISIBILITY
=================

Every column has both a positive and negative form. Negative flags
suppress columns; positive flags re-enable them (useful for overriding
a personality's defaults).

`--permissions`, `--no-permissions`
: Show or suppress the permissions field.

`--filesize`, `--no-filesize`
: Show or suppress the file size field.

`--user`, `--no-user`
: Show or suppress the user field.

`--no-time`
: Suppress the time field.

`--no-inode`
: Suppress the inode field.

`--no-group`
: Suppress the group field.

`--no-links`
: Suppress the hard links field.

`--no-blocks`
: Suppress the blocks field.


COLUMN AND FORMAT SELECTION
===========================

`--columns=COLS`
: Comma-separated list of columns to display. Overrides the `-l` tier.
Implies long view. Valid names: `perms`, `size`, `user`, `group`,
`links`, `inode`, `blocks`, `octal`, `modified`, `changed`, `accessed`,
`created`, `vcs`.

`--format=NAME`
: Select a named column format. Compiled-in formats: `long`, `long2`,
`long3`. Additional formats may be defined in the config file. Implies
long view.

Precedence: `--columns` > `--format` > `-l` tier > individual flags.


PERSONALITIES
=============

`-p`, `--personality=NAME`
: Apply a named personality, which bundles columns, flags, and settings.
Equivalent to invoking lx via an argv[0] symlink with that name.

Compiled-in personalities: `ll`, `lll`, `la`, `tree`, `ls`.
Additional personalities may be defined in the config file.

When lx is invoked via a symlink whose name matches a personality, that
personality is applied automatically. For example, if `ll` is a symlink
to `lx`, running `ll` is equivalent to `lx -pll`.


VCS INTEGRATION
===============

`--vcs=BACKEND`
: Select the VCS backend. BACKEND is `auto`, `git`, `jj`, or `none`.
Default: `auto` (prefers jj if `.jj/` exists, falls back to git).

`--vcs-status`
: Show per-file VCS status column.

`--vcs-ignore`
: Hide files ignored by VCS.

Legacy aliases: `--git` (for `--vcs-status`), `--git-ignore` (for
`--vcs-ignore`).


CONFIGURATION
=============

lx reads a TOML configuration file from these locations (first found wins):

1. `$LX_CONFIG` — explicit path
2. `~/.lxconfig.toml`
3. `$XDG_CONFIG_HOME/lx/config.toml`
4. `~/Library/Application Support/lx/config.toml` (macOS)

Run `lx --init-config` to generate a commented starter file.


ENVIRONMENT VARIABLES
=====================

`LX_CONFIG`
: Explicit path to the config file.

`LX_COLORS`
: Extended colour scheme. Overrides `LS_COLORS`. Uses two-letter codes
for UI elements (e.g. `ur` for user-read permission, `da` for date).

`LX_DEBUG`
: Enable debug logging. Set to `1` for debug, `trace` for trace level.

`LX_GRID_ROWS`
: Minimum rows before the grid-details view activates.

`LX_ICON_SPACING`
: Number of spaces between an icon and its filename.

`LS_COLORS`
: Standard file-type colour scheme.

`COLUMNS`
: Override terminal width.

`TIME_STYLE`
: Default timestamp style (overridden by `--time-style`).

`NO_COLOR`
: Disable colours. See <https://no-color.org/>.


EXIT STATUSES
=============

0
: Success.

1
: Runtime error (I/O error during operation).

3
: Options error (invalid command-line arguments).


KNOWN BUGS
==========

`--vcs-ignore` does not work with the jj backend.  The jj CLI
currently has no way to report which files are gitignored (as distinct
from untracked).  Workarounds: use `--vcs=git --vcs-ignore`, or use
`-I` glob patterns to exclude specific files.


AUTHOR
======

lx is maintained by Johann Visagie, based on exa by Benjamin Sago.

**Source code:** `https://github.com/wjv/lx`
