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

`-C`, `--count`
: Print a summary to stderr showing the number of items displayed.
When combined with `-Z`/`--total-size`, also shows the total size
of displayed items.  In tree views, expanded directories are not
counted towards the total (their children account for themselves);
pruned or depth-limited directories use their recursive total.
The size respects `-b` (bytes) and `-B` (binary) formatting.
Works with all view modes including tree and recursive listings.

`--colour=WHEN`
: When to use terminal colours. WHEN is `always`, `auto`, or `never`.
Alias: `--color`.

`--colour-scale`[`=MODE`]
: Colour file sizes on a scale. MODE is `16` (basic ANSI colours),
`256` (extended palette), or `none`. Default when bare: `16`.
Alias: `--color-scale`.  **Deprecated** in favour of `--gradient`;
`=none` translates to `--gradient=none` and `=16`/`=256` translate
to `--gradient=all`.  Will be removed in a future release.

`--gradient`[`=COLUMNS`]
: Per-column gradient on/off.  COLUMNS is a comma-separated list
of column names that should render with their gradient
(currently `size` and `date`), or one of the special tokens
`none` (no gradients) or `all` (every gradient-capable column,
the default).  Bare `--gradient` is equivalent to `--gradient=all`.

The flat colour each column collapses to when its gradient is
disabled comes from the active theme: `size-major` / `size-minor`
for the size column, `date-flat` for the date column.  See
**lxconfig.toml**(5) for theme keys.

`--no-gradient`
: Hidden alias for `--gradient=none`.  Turns off both the size
and date gradients.

Timestamps are also coloured by age: recent files appear brighter,
older files fade towards grey.  The six tiers (`date-now` through
`date-old`) are theme keys — see **lxconfig.toml**(5).

`--icons`[`=WHEN`]
: Display icons next to file names. WHEN is `always`, `auto`, or `never`.

`--no-icons`
: Don't display icons. Alias for `--icons=never`.

`-w`, `--width=COLS`
: Set the terminal width explicitly, overriding auto-detection and the
`COLUMNS` environment variable.

`-A`, `--absolute`
: Display fully resolved absolute file paths.

`--hyperlink`[`=WHEN`]
: Display file names as clickable OSC 8 hyperlinks. WHEN is `always`,
`auto`, or `never`.

`--quotes`[`=WHEN`]
: Quote file names containing spaces. WHEN is `always`, `auto`, or `never`.

`--theme=NAME`
: Select a named theme from the config file.


FILTERING OPTIONS
=================

`-a`, `--all`
: Show hidden and dot files. Use twice (`-aa`) to also show `.` and `..`.

`-d`, `--list-dirs`
: List directories as regular files.

`-D`, `--only-dirs`
: List only directories, not files.

`-f`, `--only-files`
: List only regular files, not directories.

`-I`, `--ignore=GLOBS`
: Glob patterns, pipe-separated, of files to hide completely.
Alias: `--ignore-glob`.

`-P`, `--prune=GLOBS`
: Glob patterns, pipe-separated, of directories to show but not
recurse into. The directory itself is displayed (with metadata and
`--total-size` if active), but its children are hidden. Only
meaningful with `-T` or `-R`; silently ignored otherwise.
Alias: `--prune-glob`.

`--symlinks=MODE`
: How to handle symbolic links. MODE is `show` (default), `hide`, or
`follow`.  `show` displays symlinks as-is.  `hide` removes symlinks
from listings.  `follow` dereferences symlinks, showing the target's
metadata, and recurses into symlinked directories in `-T`/`-R` mode.
Broken symlinks are always shown regardless of mode.

See also `--vcs-ignore` under **VCS INTEGRATION** for hiding files
matched by repository ignore rules.


SORTING OPTIONS
===============

`-r`, `--reverse`
: Reverse the sort order.

`-s`, `--sort=FIELD`
: Which field to sort by.

Valid sort fields:

- **Name / extension**: '`name`', '`Name`', '`extension`',
  '`Extension`', '`version`'
- **Size and allocation**: '`size`', '`blocks`', '`links`'
- **Ownership and mode**: '`permissions`', '`flags`', '`user`',
  '`User`', '`group`', '`Group`', '`uid`', '`gid`'
- **Time**: '`modified`', '`changed`', '`accessed`', '`created`'
- **VCS**: '`vcs`'
- **Miscellaneous**: '`inode`', '`type`', '`none`'

As a general rule, any column-add long flag is also accepted as a
sort field name.  `--permissions` the column flag has `permissions`
the sort field, `--filesize` has `filesize`, and so on.  Short
aliases are also accepted where the column flag has them.

The '`modified`' sort field has the aliases '`date`', '`time`',
'`mod`', and '`newest`'.  Its reverse order has the aliases '`age`'
and '`oldest`'.  Other aliases:

- '`ext`' / '`Ext`' for '`extension`' / '`Extension`'
- '`filesize`' for '`size`'
- '`mode`', '`octal`' for '`permissions`'
- '`ch`' for '`changed`'
- '`acc`' for '`accessed`'
- '`cr`' for '`created`'

Sort fields starting with a capital letter sort uppercase before
lowercase: 'A' then 'B' then 'a' then 'b'.  Fields starting with a
lowercase letter mix them: 'A' then 'a' then 'B' then 'b'.  This
applies to '`name`'/'`Name`', '`extension`'/'`Extension`', and
'`user`'/'`User`', '`group`'/'`Group`'.

`-s version` is an alias for `-s name` and exists for users
migrating from tools that use a separate flag.  lx's default name
sort already handles embedded numbers naturally (`v1.10` sorts
after `v1.2`) via `natord`.

`-s permissions` sorts by the numeric octal value of the permission
bits, independent of whether the display is symbolic or octal.
`-s flags` sorts on the raw bit pattern of platform file flags
(macOS/BSD `chflags`, Linux `chattr`).

`-s user` and `-s group` sort by resolved owner/group name, falling
back to UID/GID for unresolvable entries.  `-s uid` and `-s gid`
sort by the raw numeric ID regardless of whether a name is
available.

`-s vcs` sorts by VCS status with attention-worthy states (conflicted,
modified, added, etc.) first and unmodified files last.  In grid
and lines views (where no VCS cache is available) it falls back to
name order.  Secondary sort within a status group is by name.

With `-Z`/`--total-size`, sorting by `size` uses the recursive
directory total instead of the inode size.

`--group-dirs=WHEN`
: Group directories before or after other files. WHEN is `first`, `last`,
or `none`.  Directory grouping partitions the listing into two blocks
(directories and non-directories); the primary sort field then orders
entries within each block independently.

`-F`, `--dirs-first`
: Directories first (short for `--group-dirs=first`).
Legacy alias: `--group-directories-first`.

`-J`, `--dirs-last`
: Directories last (short for `--group-dirs=last`).
Legacy alias: `--group-directories-last`.


LONG VIEW OPTIONS
=================

These options affect the columns displayed in long view (`-l`):

`--size-style=STYLE`
: How to display file sizes.  STYLE is `decimal` (default: k, M, G
prefixes), `binary` (KiB, MiB, GiB), or `bytes` (raw count).

`-B`, `--binary`
: Alias for `--size-style=binary`.

`-b`, `--bytes`
: Alias for `--size-style=bytes`.

`-K`, `--decimal`
: Alias for `--size-style=decimal`.  Useful for overriding a
personality that sets `binary` or `bytes` back to the default.

`-g`, `--group`
: List each file's group name.  The group is coloured in three tiers:
your primary group, a supplementary group you belong to, or other.

`--uid`
: Show the numeric user ID column (long-form only). Composes with
`-u`/`--user`: `-lug --uid --gid` shows both names and IDs
side-by-side.

`--gid`
: Show the numeric group ID column (long-form only).

`-h`, `--header`
: Add a header row to each column.

`-H`, `--links`
: List each file's number of hard links.

`-i`, `--inode`
: List each file's inode number.

`-m`, `--modified`
: Add the modified timestamp column to the long listing.

`-c`, `--changed`
: Add the changed timestamp column to the long listing.

`--accessed`
: Add the accessed timestamp column to the long listing.

`--created`
: Add the created timestamp column to the long listing.

`-t`
: Compounding timestamp shortcut. `-t` adds `modified`, `-tt` adds
`modified` and `changed`, `-ttt` adds all four timestamps
(`modified`, `changed`, `created`, `accessed`). Composes with
`-l`, `--format`, and `--columns`. Unlike the individual flags,
`-t` has no long form — its only sensible spelling is the
compounding short.

`--time-style=STYLE`
: How to format timestamps. Built-in styles: `default`, `iso`, `long-iso`,
`full-iso`, `relative`. A custom strftime format can be specified with a
leading `+` (e.g. `--time-style='+%d %b %Y'`). The `relative` style shows
human-friendly durations such as "2 hours ago" or "3 days ago".

`-S`, `--blocks`
: List each file's number of file system blocks.

`-Z`, `--total-size`
: Show total recursive size for directories. When combined with
`--sort=size`, sorting uses the recursive total.

`-@`, `--extended`
: List each file's extended attributes and sizes.

`-o`, `--octal`
: List each file's permissions in octal format. Alias: `--octal-permissions`.

`-O`, `--flags`
: Show platform file flags. On macOS and FreeBSD, these are the flags
set by `chflags(1)` and shown by `ls -lO` (macOS) or `ls -lo` (FreeBSD)
(e.g. `hidden`, `uchg`,
`uappnd`, `nodump`, `uarch`). On Linux, these are the file attributes
set by `chattr(1)` and shown by `lsattr` (e.g. `immutable`, `append`,
`nodump`, `noatime`). Shows `-` when no flags are set. Available via
`--columns=flags`.


COLUMN AND FORMAT SELECTION
===========================

`--columns=COLS`
: Comma-separated list of columns to display. Overrides the `-l`
tier. Implies long view. Valid names: `permissions`, `size`, `user`,
`uid`, `group`, `gid`, `links`, `inode`, `blocks`, `octal`, `flags`,
`modified`, `changed`, `accessed`, `created`, `vcs`. `perms` is
accepted as a backward-compatible alias for `permissions`.

`--format=NAME`
: Select a named column format. Compiled-in formats: `long`, `long2`,
`long3`. Additional formats may be defined in the config file. Implies
long view.

Precedence: `--columns` > `--format` > `-l` tier > individual flags.


COLUMN OVERRIDES
================

Every column and display option has a negative form for overriding
personality defaults.  For flags with a short form, a `--no-X` alias
is also accepted (e.g. `--no-h` for `--no-header`, `--no-Z` for
`--no-total-size`, `--no-C` for `--no-count`, `--no-g` for
`--no-group`, `--no-i` for `--no-inode`, `--no-H` for `--no-links`,
`--no-S` for `--no-blocks`, `--no-o` for `--no-octal`, `--no-u` for
`--no-user`, `--no-z` for `--no-filesize`, `--no-M` for
`--no-permissions`, `--no-m` for `--no-modified`, `--no-c` for
`--no-changed`).

`-M`, `--permissions`, `--no-permissions`
: Show or suppress the permissions field. `--mode`/`--no-mode` are
accepted as long aliases (matching traditional Unix terminology).
`--no-M` is a hidden short-letter alias for `--no-permissions`.

`-z`, `--filesize`, `--no-filesize`
: Show or suppress the file size field. `--size`/`--no-size` are
accepted as long aliases. `--no-z` is a hidden short-letter alias
for `--no-filesize`.

`-u`, `--user`, `--no-user`
: Show or suppress the user field. `--no-u` is a hidden short-letter
alias for `--no-user`.

`--uid`, `--no-uid`
: Show or suppress the numeric user ID column (long-form only).

`--gid`, `--no-gid`
: Show or suppress the numeric group ID column (long-form only).

`--no-time`
: Clear all timestamp columns from the base format. Runs *before*
individual timestamp adds, so `--no-time --accessed` leaves just
the accessed column. Accepts `--no-timestamps` as a hidden alias.

`--no-modified`, `--no-changed`, `--no-accessed`, `--no-created`
: Suppress individual timestamp columns. `--no-m` and `--no-c` are
accepted as hidden short-letter aliases for `--no-modified` and
`--no-changed` respectively. Unlike `--no-time`, these run *after*
individual adds, so they beat an explicit `--modified` on the same
command line.

`--no-inode`
: Suppress the inode field.

`--no-group`
: Suppress the group field.

`--no-links`
: Suppress the hard links field.

`--no-blocks`
: Suppress the blocks field.

`--no-octal`
: Suppress the octal permissions column.

`--no-header`
: Suppress the header row.

`--no-count`
: Suppress the `-C`/`--count` summary.

`--no-total-size`
: Suppress `-Z`/`--total-size`.


PERSONALITIES
=============

Personalities are lx's answer to `alias ll='ls -l'`: a structured,
inheritable, introspectable way to have multiple "modes" for listing
files, without shell configuration.  A personality is a named bundle
of flags and settings, activated either by the name lx is invoked
under (via an `argv[0]` symlink), by the `-p`/`--personality` flag,
by the `LX_PERSONALITY` environment variable, or by the compiled-in
default when bare `lx` is run.

Resolution precedence (highest to lowest): `-p` flag, `argv[0]`
symlink name, `$LX_PERSONALITY`, compiled-in default (`lx`).  When
lx is invoked by its own name, the `argv[0]` step is skipped so
that `$LX_PERSONALITY` can take effect.  An unknown name from `-p`
or `$LX_PERSONALITY` is an error; an unknown `argv[0]` name falls
through silently.

`-p`, `--personality=NAME`
: Apply a named personality for this invocation.  Equivalent to
invoking lx via an `argv[0]` symlink with the same name.

Compiled-in personalities:

- `lx` — the default, used when `lx` is invoked as itself.
- `ll` — long view equivalent to `lx -l`, with VCS status and
  directories grouped first.
- `la` — like `ll`, plus hidden files.
- `lll` — tier-3 long view with header, all timestamps, and extra
  metadata columns.
- `tree` — recursive tree view with directories first.
- `ls` — tries to mimic POSIX `ls`.

Additional personalities may be defined in the config file.  Every
CLI flag has a corresponding config key, so a personality definition
is simply the flags you would type on the command line, spelled out
as TOML.  Personalities support inheritance (`inherits = "NAME"`) to
build on another personality's settings, and may select a theme
(`theme = "NAME"`):

    [personality.work]
    inherits = "ll"
    ignore = "*.tmp|*.bak"
    time-style = "long-iso"
    theme = "catppuccin-mocha"

When lx is invoked via a symlink whose name matches a personality,
that personality is applied automatically.  For example, if `ll` is
a symlink to `lx` on your `$PATH`, running `ll` is equivalent to
`lx -p ll`.  This is the recommended way to use personalities in
daily work — create the symlinks once, and never type `-p` again.

For the full syntax, including every personality setting key, the
inheritance semantics, and the conditional `[[when]]` block
vocabulary for environment-based overrides, see the
**PERSONALITIES** section of **lxconfig.toml**(5).


VCS INTEGRATION
===============

`--vcs=BACKEND`
: Select the VCS backend. BACKEND is `auto`, `git`, `jj`, or `none`.
Default: `auto` (prefers jj if `.jj/` exists, falls back to git).
The `jj` backend requires `lx` to be built with the `jj` feature flag.

`--vcs-status`
: Show per-file VCS status column. The column header shows the active
backend: **Git** or **JJ**.
Status characters: `-` not modified, `M` modified, `A` added (jj) /
`N` new (git), `D` deleted, `R` renamed, `C` copied, `I` ignored,
`U` untracked, `!` conflicted.
**Git** shows two columns (staged + unstaged); when both are the same,
they collapse into one character.
**jj** shows two columns: change status (@ vs @-) and tracking status
(`U` untracked, `I` ignored, space = tracked).

`--vcs-ignore`
: Hide files ignored by VCS and VCS metadata directories (`.git`,
`.jj`). Works with both git and jj backends.

`--vcs-repos`
: Show a per-directory VCS repository indicator column. For each
directory, shows `G` (git repo), `J` (jj repo), or `-` (not a repo).
Git repos also show the current branch name. Useful for scanning
workspace directories containing multiple repositories.

Note: the legacy `--git` and `--git-ignore` flags have been removed.
Use `--vcs-status` and `--vcs-ignore` instead.


CONFIGURATION
=============

lx reads a TOML configuration file from these locations (first found wins):

1. `$LX_CONFIG` — explicit path
2. `~/.lxconfig.toml`
3. `$XDG_CONFIG_HOME/lx/config.toml`
4. `~/Library/Application Support/lx/config.toml` (macOS)

Run `lx --init-config` to generate a commented starter file.

The config file includes a `version` field to track the schema version.
The current version is `"0.5"`.  If you have a legacy config from an
earlier version, run `lx --upgrade-config` to migrate it (migrations
from 0.1 through 0.4 are all supported; a `.bak` of the original is
saved automatically).

For the full config file format — every section type, every key, the
conditional `[[when]]` block syntax for environment-based personality
overrides, and the complete theme/style/class vocabulary — see
**lxconfig.toml**(5).

`--show-config`
: Show a coloured summary of the active configuration and exit.

`--init-config`
: Generate a commented starter config file at `~/.lxconfig.toml`.

`--upgrade-config`
: Upgrade a legacy config file to the current format.

`--save-as=NAME`
: Save the CLI flags from this invocation as a named personality in
`conf.d/NAME.toml`.  The saved definition inherits from the active
personality and contains only the flags you typed on this command line.
If `NAME.toml` already exists, the previous file is backed up to
`NAME.toml.bak`.  Exits after saving without listing files.

The `--dump-*` flags output copy-pasteable TOML definitions. Each
accepts an optional `=NAME` to dump a single definition, or dumps all
when used bare:

`--dump-class`[`=NAME`]
: Dump file-type class definitions.

`--dump-format`[`=NAME`]
: Dump column format definitions.

`--dump-personality`[`=NAME`]
: Dump personality definitions (before inheritance merging).

`--dump-theme`[`=NAME`]
: Dump theme definitions.

`--dump-style`[`=NAME`]
: Dump style definitions.


DROP-IN DIRECTORY
=================

After loading the main config file, lx scans a `conf.d/` directory for
additional TOML fragments.  Each `*.toml` file in the directory is
loaded in **alphabetical order** and merged into the configuration.
Later files override earlier ones by name.

The drop-in directory is searched at:

1. The parent directory of the main config file, plus `conf.d/`
2. `$XDG_CONFIG_HOME/lx/conf.d/` (default: `~/.config/lx/conf.d/`)
3. `~/Library/Application Support/lx/conf.d/` (macOS)

Drop-in files do not need a `version` field.  They may contain any
combination of `[theme.*]`, `[style.*]`, `[class]`, `[personality.*]`,
and `[format.*]` sections.

lx ships with a library of curated themes in the `themes/` directory
of the source tree — copy any of them to `conf.d/` to activate.


SHELL COMPLETIONS
=================

`--completions=SHELL`
: Generate shell completion script for SHELL and write it to stdout.
SHELL is one of `bash`, `zsh`, `fish`, `elvish`, or `powershell`.

Quick activation for the current session:

    # bash
    source <(lx --completions bash)

    # zsh
    source <(lx --completions zsh)

    # fish
    lx --completions fish | source

Or save to the standard location for permanent installation, e.g.:

    lx --completions bash > ~/.local/share/bash-completion/completions/lx
    lx --completions zsh  > ~/.zfunc/_lx
    lx --completions fish > ~/.config/fish/completions/lx.fish

(For zsh, `~/.zfunc` must be in your `$fpath`.)


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
Also settable as `grid-rows` in a personality definition (config takes precedence).

`LX_ICON_SPACING`
: Number of spaces between an icon and its filename.
Also settable as `icon-spacing` in a personality definition (config takes precedence).

`LX_PERSONALITY`
: Session-level personality selection.  Equivalent to `-p NAME` but
lower in precedence than both `-p` and `argv[0]` symlink dispatch.
Useful for setting a default personality per terminal or shell session.

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


FEATURE FLAGS
=============

`lx` has optional feature flags that control which VCS backends are
compiled in.  These are selected at build time with `cargo build
--features`.

`git` (default)
: Git support via the `git2` crate.

`jj` (opt-in)
: Jujutsu support via the `jj-lib` crate.  Adds approximately 5 MB to
the binary size.  Build with: `cargo build --features jj`.


SEE ALSO
========

**lxconfig.toml**(5)


AUTHOR
======

lx is maintained by Johann Visagie, based on exa by Benjamin Sago.

**Source code:** `https://github.com/wjv/lx`
