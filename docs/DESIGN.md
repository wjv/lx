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


## Flag vocabulary

### Logical flag pairs

Short flags are assigned in logical pairs or groups wherever possible,
making them easier to remember:

| Pair               | Flags                                                               | Relationship              |
|--------------------|---------------------------------------------------------------------|---------------------------|
| View modes         | `-l` (long) / `-G` (grid) / `-1` (oneline)                          | Mutually exclusive        |
| Recursion          | `-T` (tree) / `-R` (recurse) / `-L` (level limit)                   | Related group             |
| Dir grouping       | `-F` (first) / `-J` (last)                                          | Opposites on the home row |
| Dir/file filtering | `-D` (only dirs) / `-f` (only files)                                | Opposites                 |
| Filtering          | `-I` (ignore glob) / `-P` (prune glob)                              | Related: hide vs show-but-skip |
| Timestamps         | `-m` (modified) / `-c` (changed) / `-u` (accessed) / `-U` (created) | Full set                  |
| Size display       | `-b` (binary prefixes) / `-B` (bytes) / `-Z` (total size)           | Related group             |
| Users              | `-g` (group) / `-n` (numeric IDs)                                   | Related                   |
| Visibility         | `-a` (show hidden) / `-I` (ignore glob)                             | Opposite intent           |
| Sort               | `-s` (sort field) / `-r` (reverse)                                  | Compose together          |
| Summary            | `-C` (count) / `-Z` (total size)                                    | Compose: `-CZ` = count + size |
| Metadata           | `-o` (octal) / `-O` (flags)                                         | Uppercase/lowercase pair  |

The uppercase/lowercase pairing is deliberate where feasible:
`-D`/`-d` (dirs only / dirs as files), `-U`/`-u` (created / accessed),
`-B`/`-b` (bytes / binary), `-S`/`-s` (blocks / sort).


### `=WHEN` flags

Flags that control conditional behaviour use a standard `=WHEN` vocabulary:
`always`, `auto`, `never`.  This applies to `--colour`, `--icons`,
`--classify`, `--hyperlink`, and `--quotes`.

`auto` checks whether stdout is a terminal: enabled on a TTY, disabled
when piped.  (`--quotes=auto` is the exception — quoting is useful in
both contexts, so `auto` behaves like `always`.)

### Compounding flags

`-l` compounds: `-l` (basic), `-ll` (more detail), `-lll` (everything).
Each tier maps to a named format (`long`, `long2`, `long3`) that can be
overridden in the config file. This mirrors the behaviour of `-a` and
`-aa` in `exa`. Further compounding flags were and are still under
consideration.

### Positive / negative flag symmetry

Every column and display option has both a positive and negative form.
The positive form adds or enables; the negative form suppresses or
overrides.  This is how CLI flags and personalities work together:
a personality sets your defaults, and `--no-*` flags override them
per invocation.

| Show                | Hide                 | Feature          |
|---------------------|----------------------|------------------|
| `--permissions`     | `--no-permissions`   | Permission bits  |
| `--filesize`        | `--no-filesize`      | File size        |
| `--user`            | `--no-user`          | Owner            |
| `--group` / `-g`    | `--no-group`         | Group            |
| `--inode` / `-i`    | `--no-inode`         | Inode            |
| `--links` / `-H`    | `--no-links`         | Hard links       |
| `--blocks` / `-S`   | `--no-blocks`        | Blocks           |
| `--octal` / `-o`    | `--no-octal`         | Octal perms      |
| `--header` / `-h`   | `--no-header`        | Header row       |
| `--count` / `-C`    | `--no-count`         | Item count       |
| `--total-size`/`-Z` | `--no-total-size`    | Recursive sizes  |
| `--icons`           | `--no-icons`         | File icons       |

### `--no-X` short aliases

For any flag with a short form, the negation accepts a `--no-X` alias
where `X` is the short flag letter.  This is deliberately non-standard
but internally consistent: if you've memorised `-Z` for total sizes,
`--no-Z` is the obvious way to suppress it.

| Alias     | Expands to         |
|-----------|--------------------|
| `--no-h`  | `--no-header`      |
| `--no-g`  | `--no-group`       |
| `--no-i`  | `--no-inode`       |
| `--no-H`  | `--no-links`       |
| `--no-S`  | `--no-blocks`      |
| `--no-o`  | `--no-octal`       |
| `--no-C`  | `--no-count`       |
| `--no-Z`  | `--no-total-size`  |

These aliases are hidden from `--help` (power users discover them
naturally) but documented in the man page.

### Directory grouping

`--group-dirs=first|last|none` controls directory position in listings.
Short flags: `-F` (first), `-J` (last) — the home keys under the index
fingers.  The legacy `--group-directories-first` is a hidden alias.

### Sorting

`-s FIELD` with `--reverse` / `-r`.  Sort fields include `name`, `Name`
(case-sensitive), `size`, `modified`, `changed`, `accessed`, `created`,
`extension`, `type`, `none`.  The alias `age` means reverse-modified
(newest first). This is taken with essentially no change from `exa`.

### Canonical column ordering

When a column is added via an individual flag (e.g. `-S` for blocks),
it is inserted at its canonical position relative to the columns already
present — not appended at the end.  The canonical order is:

```text
inode, octal, perms, flags, links, size, blocks, user, group,
modified, changed, created, accessed, vcs, repos
```

This means `-lS` places blocks after size (where it belongs), not after
the date column.  If you want full control over column position, use
`--columns=...` or define a format.


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

| Flag | Long form       | Purpose                                  |
|------|-----------------|------------------------------------------|
| `-1` | `--oneline`     | One entry per line                       |
| `-l` | `--long`        | Long view (compounds: `-ll`, `-lll`)     |
| `-G` | `--grid`        | Grid view (default)                      |
| `-x` | `--across`      | Sort grid across                         |
| `-T` | `--tree`        | Tree view                                |
| `-R` | `--recurse`     | Recurse into directories                 |
| `-L` | `--level`       | Depth limit for `-T`/`-R`                |
| `-C` | `--count`       | Item count to stderr (`-CZ` + size)      |
| `-a` | `--all`         | Show hidden files (`-aa` for `.`/`..`)   |
| `-d` | `--list-dirs`   | Treat directories as files               |
| `-D` | `--only-dirs`   | Show only directories                    |
| `-f` | `--only-files`  | Show only files                          |
| `-F` |                 | Directories first (`--group-dirs=first`) |
| `-J` |                 | Directories last (`--group-dirs=last`)   |
| `-r` | `--reverse`     | Reverse sort order                       |
| `-s` | `--sort`        | Sort field                               |
| `-I` | `--ignore`      | Glob patterns to hide                    |
| `-P` | `--prune`       | Glob patterns to show but not recurse    |
| `-A` | `--absolute`    | Show absolute paths                      |
| `-b` | `--binary`      | Binary size prefixes (KiB)               |
| `-B` | `--bytes`       | Size in bytes                            |
| `-g` | `--group`       | Show group column                        |
| `-h` | `--header`      | Show header row                          |
| `-H` | `--links`       | Show hard link count                     |
| `-i` | `--inode`       | Show inode number                        |
| `-S` | `--blocks`      | Show block count                         |
| `-o` | `--octal`       | Show octal permissions                   |
| `-O` | `--flags`       | Show platform file flags                 |
| `-Z` | `--total-size`  | Recursive directory size                 |
| `-n` | `--numeric`     | Numeric user/group IDs                   |
| `-m` | `--modified`    | Show modified time                       |
| `-c` | `--changed`     | Show changed time                        |
| `-u` | `--accessed`    | Show accessed time                       |
| `-U` | `--created`     | Show created time                        |
| `-t` | `--time`        | Select timestamp field                   |
| `-@` | `--extended`    | Show extended attributes                 |
| `-p` | `--personality` | Select a personality                     |
| `-w` | `--width`       | Set terminal width                       |
| `-v` | `--version`     | Show version                             |
| `-?` | `--help`        | Show help                                |
