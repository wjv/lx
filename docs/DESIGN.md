# lx — CLI design principles

This document outlines the design principles behind `lx`'s command-line
interface.

`lx` was born from some frustration with `exa`'s accumulated flag
surface — the kind of interface where you end up building a stack
of shell aliases for daily use. Two complementary approaches
address this:

1. **Make the base UI consistent and approachable** — so you don't
   *need* aliases in the first place.
2. **Replace aliases with something better** — *personalities*: named,
   inheritable, structured bundles of settings.

These turned out to be parallel solutions to the same problem.  A
good base UI means personalities become a power-user tool for presets,
not a crutch for a confusing interface.  Both are better for existing;
they complement one another.

A third layer — **conditional config** — lets personalities adapt to
context (terminal emulator, SSH session) without shell-level scripting.


## Design goals

1. **Consistency over compatibility.**  `lx` is not a drop-in replacement
   for `ls` or `exa`.  Where a consistent design conflicts with legacy
   conventions, consistency wins.

2. **Every flag should have a logical partner.**  If there's a `--foo`,
   there should be a `--no-foo`.  If there's a short flag `-X`, it should
   pair with a related short flag.

3. **No magic.**  `lx` should work the same with or without a config file.
   `--init-config` generates a config that documents the defaults but
   doesn't change them.

4. **Composability.**  Personalities, formats, themes, styles, and classes
   are independent named entities that compose naturally.  Nothing is
   hard-wired to anything else.


## Personalities

Shell aliases have been the standard way to customise `ls` behaviour
for decades.  Personalities are lx's answer to the same need, but
with several advantages over aliases:

- **Structured.**  A personality is a named TOML section, not a
  fragile string of flags.  Every CLI flag has a corresponding
  config key (`--sort=age` → `sort = "age"`).
- **Inheritable.**  Personalities form a tree.  `la` inherits from
  `ll` and adds `all = true`.  Change `ll` and `la` follows.
- **Discoverable.**  `--show-config` reveals the active personality
  and its resolved settings.  Shell aliases are opaque.
- **argv[0]-dispatched.**  Create a symlink and the personality
  activates automatically — no shell configuration needed.

An irony of the design: if the base CLI flags are consistent and
approachable enough (which is the goal of the flag redesign), the
*need* for personalities diminishes.  The base UI becomes usable
without a layer of aliases on top.  Personalities then become a
power-user tool for presets (`du`, `tree`) rather than a crutch
for a confusing interface.

### Design decisions

- **No implicit root.**  Personality inheritance is always explicit
  (`inherits = "NAME"`).  There is no magic base personality that
  everything inherits from — the user wires the tree however they
  like.
- **Config wins over compiled-in.**  If a config file defines a
  personality with the same name as a compiled-in one, the config
  version takes priority.
- **`lx` is a personality.**  When invoked as `lx`, the `lx`
  personality is applied (which inherits from `default`).  This
  means the user can customise bare `lx` the same way as any
  other personality.


## The orthogonal CLI

The 0.8 refactor reshaped `lx`'s flag surface around a single idea:
every user-visible knob should have a predictable shape.  This
section is the user-facing summary; the rest of the document
expands on the principles it introduces.

### Four disjoint classes of flag

`lx`'s long-view flags fall into four categories, each with a
distinct job:

1. **Column selectors** add or remove a column from the listing.
   `--inode`, `--permissions`, `--filesize`, `--user`, `--uid`,
   `--group`, `--gid`, `--links`, `--blocks`, `--octal`, `--flags`,
   `--modified`, `--changed`, `--accessed`, `--created`,
   `--vcs-status`, `--vcs-repos`.  Every one has a matching
   `--no-*` negation.
2. **Column display modifiers** change how an *already visible*
   column is rendered without adding or removing anything.
   `--binary` / `--bytes` / `--total-size` reshape the size
   column; `--time-style` reshapes timestamps.  They're no-ops
   if the column they'd affect isn't in the list.
3. **File name modifiers** change how the filename column itself
   is rendered: `--icons`, `--classify`, `--hyperlink`, `--quotes`,
   `--absolute`.  The filename column is always present, in every
   view mode.
4. **Framing** adds structure around the table.  `--header` /
   `-h` shows a header row; `--count` / `-C` prints an item count
   (and, with `-Z`, a total size) to stderr after the listing.

Keeping these classes separate is what lets the CLI be predictable.
A column selector never changes rendering; a modifier never adds
or removes a column; framing never touches column content.

### Every column is addable, suppressible, and sortable

The orthogonal rule is: **if a column makes sense to display, it
makes sense to sort on.**  There's no reason to privilege "size"
and "modified" over "blocks" and "user" — that gap was inherited
from `exa`, not a principled decision.  With it closed, every
metadata column has the same four-flag shape:

| Role            | Shape              |
|-----------------|--------------------|
| Add             | `--COLUMN` / short |
| Suppress        | `--no-COLUMN` / `--no-X` (hidden short alias) |
| Sort ascending  | `-s COLUMN`        |
| Sort descending | `-rs COLUMN`       |

So `-ls blocks` is a long listing sorted by block count, and
`-l --columns=user,uid,name -s uid` is an audit view sorted by
numeric UID.  The full sort vocabulary — including the version
sort, VCS status grouping, and case-sensitive variants — is
listed in [`docs/GUIDE.md`](GUIDE.md#sorting).

### `=WHEN` flags

Flags that control conditional behaviour share a `=WHEN`
vocabulary: `always`, `auto`, `never`.  `--colour`, `--icons`,
`--classify`, `--hyperlink`, and `--quotes` all follow it.

`auto` checks whether stdout is a terminal: enabled on a TTY,
disabled when piped.  (`--quotes=auto` is the exception — quoting
is useful in both contexts, so it behaves like `always`.)

### Compounding shortcuts: `-l` and `-t`

Two flags compound by repetition, but they operate on orthogonal
axes.

**`-l` / `-ll` / `-lll` — detail tiers.**  Each tier selects one
of three named formats (`long`, `long2`, `long3`), each a different
bundle of columns.  It's a *format shortcut*, not a column toggle.
The formats are ordinary and can be redefined in `[format]`.

**`-t` / `-tt` / `-ttt` — timestamp tiers.**  Each tier adds a set
of timestamp columns.  `-t` adds `modified`; `-tt` adds `modified`
and `changed`; `-ttt` adds all four (`modified`, `changed`,
`created`, `accessed`).  Unlike `-l`, `-t` composes with whatever
format you're already using — it desugars to individual add flags.

`-l` answers *"what does the table look like?"*, `-t` answers
*"which timestamps go in it?"*.  They compose: `-ll -tt` gives
you the tier-2 long view with two timestamp columns on top.

### The precedence pipeline

Column selection is deterministic.  The same flags always produce
the same column list.  There are three layers.

**1. Base list.**  Exactly one source chooses the starting set of
columns, in strict precedence order:

1. `--columns=COLS` — explicit, user-ordered list.  Highest
   precedence; nothing else defines a base.
2. `--format=NAME` — look up a named format.
3. `-l` tier — `long`, `long2`, or `long3` depending on repetition
   count.

If `-l` is combined with `--format` or `--columns`, the tier is
ignored; the higher-precedence source supplies the column list.

**2. Additions.**  Individual column flags (`-i`, `-o`, `-H`,
`-S`, `-O`, `-m`, `-c`, `--uid`, etc.) insert their column into
the list if not already present.  Each column has a **canonical
position**, and insertion respects that order regardless of the
order the flags appeared on the command line:

```text
inode → octal → permissions → flags → links → filesize → blocks →
user → uid → group → gid → modified → changed → created → accessed →
vcs-status → vcs-repos → name
```

So `-l -i -o` always produces `inode, octal, permissions, filesize,
user, modified`, not whatever order you happened to type.  For full
user control over column order, use `--columns=`.

**3. Suppressions.**  After adds, `--no-X` and `--no-*` flags
remove columns from the list.  On the same command line,
`--show-X` beats `--no-X`.

### The one exception: `--no-time`

`--no-time` is a bulk shortcut — it clears all four timestamp
columns at once — and it runs *before* individual adds, not after.
That's so explicit additions survive it:

```sh
lx -l --no-time --accessed
```

means "clear the defaults, then add accessed", not "clear
everything including the accessed I just asked for".  Every
per-column suppression still runs after adds in the usual way;
only the bulk clear is promoted to run earlier.

### `--no-X` short aliases

Every column short flag has a hidden `--no-X` alias.  If you've
memorised `-Z` for total sizes, `--no-Z` is the obvious way to
suppress it.  These aliases are hidden from `--help` (power users
discover them naturally) and documented in the man page.

### Directory grouping

`--group-dirs=first|last|none` controls directory position.
Short flags: `-F` (first) and `-J` (last) — the home keys under
the index fingers.  The legacy `--group-directories-first` is a
hidden alias.


## Three layers of configuration

lx's configuration model has three layers, applied in order:

1. **Personality** — defines defaults: which columns, what format,
   which theme.  Comes from the config file or compiled-in definitions.
   Activated by name (`-p NAME`, argv[0] symlink, or the `lx` default).

2. **CLI flags** — override the personality for this invocation.
   `-g` adds the group column, `--no-g` removes it. `--theme=dark`
   overrides the personality's theme.  Last flag wins.

3. **Conditional overrides** (`[[when]]` blocks) — personality settings
   that vary by environment.  Evaluated between layers 1 and 2: the
   personality resolves, conditionals overlay, then CLI flags override.

This means a user can:
- Define `ll` with `header = true` and `total-size = true` (layer 1)
- Add `[[personality.ll.when]] env.SSH_CONNECTION = true` /
  `colour = "never"` (layer 3)
- Run `ll --no-h` to suppress the header for one listing (layer 2)

Each layer has a clear role: config defines *what*, conditionals
adapt to *where*, CLI flags handle *this time*.


## Short flag reference

Shipped 0.8 allocations.

**Display / layout**

| Flag | Long form       | Purpose                                  |
|------|-----------------|------------------------------------------|
| `-1` | `--oneline`     | One entry per line                       |
| `-l` | `--long`        | Long view (compounds: `-ll`, `-lll`)     |
| `-G` | `--grid`        | Grid view (default)                      |
| `-x` | `--across`      | Sort grid across                         |
| `-T` | `--tree`        | Tree view                                |
| `-R` | `--recurse`     | Recurse into directories                 |
| `-L` | `--level`       | Depth limit for `-T` / `-R`              |
| `-C` | `--count`       | Item count to stderr (`-CZ` adds total size) |
| `-w` | `--width`       | Terminal width override                  |
| `-A` | `--absolute`    | Show absolute paths                      |

**Filtering and sort**

| Flag | Long form       | Purpose                                  |
|------|-----------------|------------------------------------------|
| `-a` | `--all`         | Show hidden files (`-aa` for `.`/`..`)   |
| `-d` | `--list-dirs`   | Treat directories as files               |
| `-D` | `--only-dirs`   | Show only directories                    |
| `-f` | `--only-files`  | Show only files                          |
| `-F` | `--dirs-first`  | Directories first (`--group-dirs=first`) |
| `-J` | `--dirs-last`   | Directories last (`--group-dirs=last`)   |
| `-I` | `--ignore`      | Glob patterns to hide                    |
| `-P` | `--prune`       | Glob patterns to show but not recurse    |
| `-s` | `--sort`        | Sort field                               |
| `-r` | `--reverse`     | Reverse sort order                       |

**Long-view columns** (canonical order)

| Flag | Long form                   | Purpose                     |
|------|-----------------------------|-----------------------------|
| `-i` | `--inode`                   | Inode number                |
| `-o` | `--octal`                   | Octal permissions           |
| `-M` | `--permissions` / `--mode`  | Symbolic permission bits    |
| `-O` | `--flags`                   | Platform file flags         |
| `-H` | `--links`                   | Hard link count             |
| `-z` | `--filesize` / `--size`     | File size                   |
| `-Z` | `--total-size`              | Recursive directory totals  |
| `-S` | `--blocks`                  | Allocated block count       |
| `-u` | `--user`                    | Owner name                  |
| `-g` | `--group`                   | Group name                  |
| `-@` | `--extended`                | Extended attributes         |
| `-h` | `--header`                  | Header row                  |
| `-b` | `--bytes`                   | Raw byte counts             |
| `-K` | `--decimal`                 | Decimal size prefixes (k, M, G) |
| `-B` | `--binary`                  | Binary size prefixes (KiB)  |

**Timestamps**

| Flag    | Long form    | Purpose                                       |
|---------|--------------|-----------------------------------------------|
| `-m`    | `--modified` | Modification time                             |
| `-c`    | `--changed`  | Status-change time                            |
| `-t`    | (none)       | Compounding tier: `-t`/`-tt`/`-ttt`           |

`--accessed` and `--created` are long-only.  `--uid`, `--gid`,
`--vcs-status`, and `--vcs-repos` are also long-only — niche
enough that reserving a single-letter short flag would be
wasteful.

**Meta**

| Flag | Long form       | Purpose              |
|------|-----------------|----------------------|
| `-p` | `--personality` | Select a personality |
| `-v` | `--version`     | Show version         |
| `-?` | `--help`        | Show help            |

### Notable changes in 0.8

- `-n` / `--numeric` has been retired.  Use `--uid` and/or `--gid`
  as first-class columns instead, or define a `numeric`
  personality.
- `-t` used to select a single timestamp field (`-t FIELD` with
  `--time`).  It now compounds: `-t` / `-tt` / `-ttt`.  The long
  form `--time` has been dropped.
- `-u` used to be `--accessed`; it is now `--user`.  `-U` (previously
  `--created`) has been freed.
- `-M` (`--permissions` / `--mode`) and `-z` (`--filesize` /
  `--size`) are new in 0.8.
- The `perms` column name has been renamed to `permissions`.
  `perms` is still accepted as a backward-compat alias by
  `--columns=` (but not by `-s`).
