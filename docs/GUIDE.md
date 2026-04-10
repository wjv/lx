# The lx user guide

This is the tutorial and reference manual for `lx` ("alex"), the file
lister with personality.  It's organised as something you can read
top-to-bottom the first time you configure `lx`, and dip into later
when you want to change something specific.

`lx` works perfectly well out of the box — none of this is required
reading before you can use it.  If you've just installed `lx` and
want to see what it does, skip to [First run](#first-run) and come
back to the configuration chapters once you've decided you want to
tweak something.

## How this guide fits in

- **`lx --help`** is the authoritative flag reference.  Every flag
  is listed there with a one-line description.
- **`man lx`** (or [`man/lx.1.md`](../man/lx.1.md)) is the long-form
  command reference.  Look here for the exact semantics of a flag.
- **`man lxconfig.toml`** (or
  [`man/lxconfig.toml.5.md`](../man/lxconfig.toml.5.md)) is the
  reference for the configuration file format.  Every key, every
  section, every accepted value.
- **[`docs/DESIGN.md`](DESIGN.md)** explains the design philosophy
  behind `lx`'s CLI — *why* the flags are shaped the way they are.
- **This guide** is narrative and example-first.  It shows you how
  to get things done and teaches you the concepts as it goes.

When something in this guide calls for the exact list of flags or
config keys, it links to the reference rather than duplicating it.

## Contents

1. [First run](#first-run)
2. [Personalities](#personalities)
3. [The configuration file](#the-configuration-file)
4. [Themes, styles, and classes](#themes-styles-and-classes)
5. [VCS integration](#vcs-integration)
6. [Daily usage patterns](#daily-usage-patterns)
7. [Shell completions](#shell-completions)
8. [Debugging your configuration](#debugging-your-configuration)
9. [Further reading](#further-reading)


## First run

The best way to start using `lx` is to just start using it.  Most
flags differ from `ls`; when you're stuck, `lx --help` is a friend.

```sh
lx                    # grid view (like ls)

lx -l                 # long view: permissions, size, user, modified
lx -ll                # + group, VCS status
lx -lll               # + header, all timestamps, links, blocks

lx -T                 # tree view
lx -T -L2             # + limit to depth 2
lx -lTFZL2            # long tree, dirs first, total sizes, depth 2
```

`lx` is deliberately *not* a drop-in replacement for POSIX `ls`.  It
tries to be consistent with itself rather than with every `ls`
convention.  Once you get used to a handful of differences — the
compounding `-l`, the separation of columns from display modifiers,
the unified `--no-*` suppressors — the rest follows naturally.

If `lx -lTFZL2` looks like a lot to memorise, that's the moment to
define a [personality](#personalities).


## Personalities

Since Unix shells first supported command aliasing, people have
aliased `ls` to call it with their favourite options.  For decades
`ll` has been shorthand for `ls -l`:

```sh
alias ll="ls -l"
```

`lx` takes this idea and promotes it into a first-class feature.
A *personality* is a named bundle of settings that `lx` adopts
based on the name it was invoked under.  The idea is:

1. Define a personality in the config file:

   ```toml
   [personality.ll]
   format = "long"
   ```

2. Create a symlink to `lx` under that name:

   ```sh
   ln -s $(which lx) ~/.local/bin/ll
   ```

When you run `ll`, `lx` sees the symlink's name in `argv[0]`, looks
up the matching personality, and applies it.  No shell alias
needed.  If you invoke `lx` as itself, or as a name that doesn't
match any personality, it uses the compiled-in `lx` personality
(which inherits from `default`).

To preview a personality without creating a symlink, use
`-p` / `--personality`:

```sh
lx -p ll
lx --personality tree
```

You can also set a session-level default via the `LX_PERSONALITY`
environment variable:

```sh
export LX_PERSONALITY=ll    # every lx invocation in this shell uses ll
```

The full resolution order is: `-p` flag → argv[0] symlink name →
`$LX_PERSONALITY` → compiled-in default (`lx`).  When lx is invoked
as itself (i.e. argv[0] is `lx`), the argv[0] step is skipped so
that the environment variable can take effect — `$LX_PERSONALITY` is
conceptually "the personality for bare `lx`".  Symlinks like `ll` or
`tree` still win over the environment variable because they're
structural: when you type `ll`, you always mean "long view".

### Creating personalities from the command line

Once you've found a combination of flags you like, save it as a
personality in one step:

```sh
lx -l --total-size --sort=size --reverse --save-as=du
```

This writes `conf.d/du.toml` containing only the flags you typed,
inheriting everything else from the active personality.  If you were
invoked as `ll`, the saved personality inherits from `ll`; if bare
`lx`, it inherits from `lx`.  If the file already exists, the
previous version is backed up to `du.toml.bak`.

### Inheritance

Personalities form a tree via `inherits`.  Children replace the
parent's `format`/`columns` and merge everything else, with the
child winning on conflicts.  This is how you build up a family of
related views without repeating yourself:

```toml
[personality.la]          # long listing, plus hidden files
inherits = "ll"
all = true

[personality.lt]          # time-sorted long listing
inherits = "ll"
sort = "age"

[personality.recent]      # recently modified files
format = "long"
sort = "modified"
reverse = true

[personality.du]          # du replacement: directory sizes
columns = ["size"]
only-dirs = true
tree = true
level = 2
sort = "size"
reverse = true
total-size = true
```

Every CLI flag has a corresponding config key — `--sort=age`
becomes `sort = "age"`, `--total-size` becomes `total-size = true`,
and so on.  Boolean flags take `true`/`false`; list-valued flags
take TOML arrays.

### The compiled-in personalities

`lx` ships with a small set of built-in personalities so it works
out of the box without any config file.  They form an inheritance
tree rooted at `default`:

```text
default ──┬──→ lx ──┬──→ ll ──→ la
          │         └──→ lll
          └──→ tree

ls  (standalone — no inherits)
```

- **`default`** — the base personality, inherited by everything
  else.  There's nothing magical about the name: you can override
  it or define your own base under a different name.
- **`lx`** — the standard personality, used when `lx` is invoked
  as itself.  Inherits from `default`.
- **`ll`** — a long listing, equivalent to `lx -l` with VCS status
  and `--group-dirs=first`.
- **`la`** — like `ll`, plus hidden files (`ls -la`).
- **`lll`** — the expanded long listing with header, all
  timestamps, hard link count, and block count.
- **`tree`** — recursive tree view with directories first.
- **`ls`** — tries to look more like POSIX `ls`.

If you redefine one of these in your own config, your definition
takes precedence over the compiled-in version.

### Using personalities

Three ways to activate a personality:

1. **Symlink dispatch** — `ll`, `la`, etc. as symlinks on your
   `$PATH`.  The most natural; no shell configuration.
2. **`-p NAME`** — explicit CLI flag.  Useful for preview, or
   when you don't want a symlink.
3. **Default** — if you run `lx` with no flags, the `lx`
   personality applies.

Personalities coexist with shell aliases; there's no reason not
to use both.  Personalities are just a more structured alternative
for the cases where a string of flags has become hard to maintain.

See [Design goals](DESIGN.md#design-goals) for why personalities
are the shape they are.


## The configuration file

`lx` reads at most one main configuration file plus an optional
directory of drop-in fragments.

> **The configuration file is optional.**  `lx` is designed to
> work the same with or without one — the config file only exists
> so you can customise things you want to change.  `--init-config`
> generates a file that documents the defaults but doesn't change
> them; it's a no-op for behaviour.

### Getting started

Generate a starter configuration:

```sh
lx --init-config
```

This writes `~/.lxconfig.toml`.  The file is self-documenting:
prose comments (starting with `##`) explain each section, and
commented-out values (starting with `#`) show the compiled-in
defaults that you can uncomment and edit.

If you had an older config, migrate it with:

```sh
lx --upgrade-config
```

This converts from any previous format (0.1, 0.2, 0.3, 0.4, 0.5)
to the current 0.6 schema and saves a `.bak` of the original.
The 0.5 → 0.6 migration is mostly cosmetic (version string only)
but also injects auto-selection `[[when]]` blocks into your
`[personality.default]` section so capable terminals get the
new theme tiers automatically.

### Sections

The configuration has five kinds of named section, each governing
a different aspect of `lx`'s behaviour:

| Section              | Purpose                                             | Example                                               |
|----------------------|-----------------------------------------------------|-------------------------------------------------------|
| `[format]`           | Column layouts for long view (flat keys)            | `long = ["permissions", "size", "user", "modified"]`  |
| `[personality.NAME]` | Bundles of settings, activated by name              | `inherits = "lx"`, `format = "long2"`, `sort = "age"` |
| `[theme.NAME]`       | UI element colours (directories, dates, etc.)       | `directory = "bold blue"`, `date = "steelblue"`       |
| `[style.NAME]`       | File-type colours (by class, glob, or filename)     | `class.source = "yellow"`, `"*.rs" = "#ff8700"`       |
| `[class]`            | Named file-type categories (lists of glob patterns) | `media = ["*.jpg", "*.png", "*.mp4"]`                 |

These compose naturally:

- personalities pick a format and a theme;
- themes reference styles;
- styles reference classes.

```text
personality ──→ format  (column layout)
     │
     └──→ theme ──────→ style ────────→ class
          (UI colours)  (file colours)  (pattern lists)
```

Personalities and themes support inheritance.  Styles and classes
are flat.

### Formats

A format is a named list of columns.  The long view uses the
`long` format by default; `-l`, `-ll`, and `-lll` select `long`,
`long2`, and `long3` respectively.

```toml
[format]
long    = ["permissions", "size", "user", "modified"]
long2   = ["permissions", "size", "user", "group", "modified", "vcs"]
long3   = ["permissions", "links", "size", "blocks",
           "user", "group", "modified", "changed", "created", "accessed", "vcs"]

compact = ["permissions", "size", "modified"]
hpc     = ["permissions", "size", "user", "group", "modified", "vcs"]
```

The column vocabulary for `[format]` and `--columns=` is:

| Column        | Shows                                                            |
|---------------|------------------------------------------------------------------|
| `permissions` | Permission bits (`perms` is accepted as a backward-compat alias) |
| `size`        | File size (`filesize` is an alias)                               |
| `user`        | Owner name                                                       |
| `uid`         | Numeric user ID                                                  |
| `group`       | Group name                                                       |
| `gid`         | Numeric group ID                                                 |
| `links`       | Hard link count                                                  |
| `inode`       | Inode number                                                     |
| `blocks`      | Allocated block count                                            |
| `octal`       | Permission bits in octal                                         |
| `flags`       | Platform file flags                                              |
| `modified`    | Last modification time                                           |
| `changed`     | Last status-change time                                          |
| `accessed`    | Last access time                                                 |
| `created`     | Creation time                                                    |
| `vcs`         | Per-file VCS status                                              |
| `repos`       | Per-directory VCS repo indicator                                 |

Use a format explicitly with `--format=NAME`, or (more commonly)
reference it from a personality:

```toml
[personality.compact]
format = "compact"
```

### Conditional overrides

Personalities can include `[[personality.NAME.when]]` blocks that
activate based on environment variables.  This lets a single
personality adapt to different terminals, SSH sessions, and so on
without shell-level scripting:

```toml
[personality.ll]
inherits = "lx"
format = "long2"
header = true

[[personality.ll.when]]
env.TERM_PROGRAM = "ghostty"
icons = "always"

[[personality.ll.when]]
env.SSH_CONNECTION = true        # set → over SSH
colour = "never"
icons = "never"
```

Conditions take three forms:

- `env.VAR = "value"` — matches when `$VAR` equals the given
  string exactly;
- `env.VAR = true` — matches when `$VAR` is set (to anything);
- `env.VAR = false` — matches when `$VAR` is unset.

Multiple keys in a single `[[when]]` block must all match (AND).
Multiple `[[when]]` blocks stack, with later matches overriding
earlier ones.

Conditionals run between the personality and the CLI flags:
the personality resolves, conditionals overlay, then CLI flags
have the final word.  See
[Three layers of configuration](DESIGN.md#three-layers-of-configuration)
for the full precedence model.

### Config file locations

`lx` searches for its main config file in this order, first found
wins:

1. **`$LX_CONFIG`** — explicit path via environment variable.
   Useful for per-project configs or testing.
2. **`~/.lxconfig.toml`** — the simplest option.  This is where
   `lx --init-config` writes by default.
3. **`$XDG_CONFIG_HOME/lx/config.toml`** — XDG base-directory
   location.  Defaults to `~/.config/lx/config.toml` if
   `$XDG_CONFIG_HOME` is not set.  Preferred on Linux; also
   used on macOS.
4. **`~/Library/Application Support/lx/config.toml`** — the
   standard macOS application location, checked after the XDG
   path.

### Drop-in directory

After loading the main config, `lx` looks for a `conf.d/`
directory alongside it and loads every `*.toml` file found there
in alphabetical order.  Each file is a standalone TOML fragment
that can contain theme, style, class, personality, or format
definitions.

The drop-in directory is searched at:

- `~/.config/lx/conf.d/` (or `$XDG_CONFIG_HOME/lx/conf.d/`)
- `~/Library/Application Support/lx/conf.d/` (macOS)

This is how the curated themes in the [`themes/`](../themes)
directory are installed: just copy them in, no editing required.

```sh
mkdir -p ~/.config/lx/conf.d
cp themes/dracula.toml ~/.config/lx/conf.d/
```

See [`man/lxconfig.toml.5.md`](../man/lxconfig.toml.5.md) for the
full reference.


## Themes, styles, and classes

`lx`'s colour customisation uses three kinds of config section that
work together:

- **Themes** (`[theme.NAME]`) set colours for UI elements —
  directories, permissions, dates, VCS status, and so on.
- **Styles** (`[style.NAME]`) set colours for *files*, either by
  reference to a named class or by matching a glob pattern or
  exact filename.
- **Classes** (`[class]`) define named file-type categories as
  lists of glob patterns: `media`, `source`, `archive`, etc.
  Once a class is defined, you can style it as a unit.

`lx` ships with **three compiled-in themes**, plus a single
compiled-in style.  All four are baked into the binary, so
sensible colours work out of the box with no config file.

| Theme      | Description                                                                                 |
|------------|---------------------------------------------------------------------------------------------|
| `exa`      | Strict 8-colour ANSI; renders identically on any terminal from a vt220 onwards.             |
| `lx-256`   | 256-colour palette, refined exa-derived look, balanced for both light and dark backgrounds. |
| `lx-24bit` | 24-bit truecolour, the smoothest gradients, balanced for both backgrounds.                  |

The default `lx` personality auto-selects the best variant for
your terminal: `lx-24bit` if `$COLORTERM` is `truecolor` or
`24bit`, otherwise `lx-256` if `$TERM` matches `*-256color`,
otherwise `exa`.  You can always override with `--theme=NAME`.

### Writing your own theme

```toml
[theme.ocean]
inherits = "exa"                    # start from the compiled-in defaults
directory = "bold dodgerblue"
date = "steelblue"
vcs-new = "bold mediumspringgreen"
use-style = "dev"                   # reference a named style set

[style.dev]
class.source = "#ff8700"              # class reference (bare dotted key)
"*.toml" = "sandybrown"               # glob pattern (quoted key)
"Makefile" = "bold underline yellow"  # exact filename (quoted key)
```

Colour values accept several forms:

- Named ANSI colours: `"bold blue"`, `"red"`
- X11 / CSS colour names: `"tomato"`, `"cornflowerblue"`, `"dodgerblue"`
- Hex: `"#ff8700"`, `"#2b2b2b"`
- 256-colour ANSI: `"38;5;208"`
- Modifiers: `bold`, `dimmed`, `italic`, `underline` (combinable)

Themes can inherit from other themes via `inherits = "NAME"`.
Without `inherits`, a theme starts from a blank slate — useful
when you want full control.  The special name `"exa"` refers to
`lx`'s compiled-in default theme.

### Classes

Classes group file types by purpose so you can style them as a
unit:

```toml
[class]
source = ["*.rs", "*.py", "*.js", "*.go"]
data   = ["*.csv", "*.json", "*.xml"]
```

`lx` ships with built-in classes for `image`, `video`, `music`,
`lossless`, `crypto`, `document`, `compressed`, `compiled`, `temp`,
and `immediate` (build/project files).  These definitions are
omitted from `--init-config`'s output for brevity; see
[`man/lxconfig.toml.5.md`](../man/lxconfig.toml.5.md) for the
full list.  Redefining a class name in your config overrides the
compiled-in version.

### Activating a theme

Set a theme permanently through a personality:

```toml
[personality.default]
theme = "ocean"                 # all personalities inherit this
```

Or pick one for a single invocation:

```sh
lx --theme=ocean
lx -l --theme=dracula
```

### Curated themes

`lx` ships with ready-made themes in the [`themes/`](../themes)
directory.  These are **drop-in files** — copy the ones you want
into `~/.config/lx/conf.d/` to make them available, then
activate with `--theme=NAME` or set as a personality default.

**Light backgrounds:**

| Theme            | Filename                |
|------------------|-------------------------|
| Catppuccin Latte | `catppuccin-latte.toml` |
| Gruvbox Light    | `gruvbox-light.toml`    |
| Nord Light       | `nord-light.toml`       |
| Solarized Light  | `solarized-light.toml`  |

**Dark backgrounds:**

| Theme            | Filename                |
|------------------|-------------------------|
| Catppuccin Mocha | `catppuccin-mocha.toml` |
| Dracula          | `dracula.toml`          |
| Gruvbox Dark     | `gruvbox-dark.toml`     |
| Nord             | `nord.toml`             |
| Solarized Dark   | `solarized-dark.toml`   |

**Both backgrounds:**

| Theme          | Filename                                                          |
|----------------|-------------------------------------------------------------------|
| The Exa Future | `the-exa-future.toml` (a 24-bit tribute to the original exa look) |

**Builtin overrides** (drop-ins that override the compiled
`lx-256` and `lx-24bit` builtins with brighter, dark-tuned
gradients):

| Theme           | Filename               |
|-----------------|------------------------|
| `lx-256-dark`   | `lx-256-dark.toml`     |
| `lx-24bit-dark` | `lx-24bit-dark.toml`   |

Install:

```sh
mkdir -p ~/.config/lx/conf.d
cp themes/dracula.toml ~/.config/lx/conf.d/
lx -l --theme=dracula
```

See [`themes/README.md`](../themes/README.md) for the full
inventory and guidance on writing your own.


## VCS integration

`lx` shows per-file version-control status in the long view, with
built-in backends for both [Git](https://git-scm.com) and
[Jujutsu](https://jj-vcs.dev/).  VCS is exposed through three
independent flags:

- **`--vcs-status`** — per-file status column (included in tier 2
  and tier 3 long views).
- **`--vcs-ignore`** — hide files ignored by the repository's
  ignore rules, and hide the `.git` / `.jj` directories themselves.
- **`--vcs-repos`** — per-directory repo indicator showing whether
  each listed directory is a repo root and whether it's clean.

Pick a backend with `--vcs=auto|git|jj|none`.  The default is
`auto`, which probes for a jj workspace first, then falls back to
git.  Co-located jj/git repos are detected correctly.

```sh
lx -ll                # tier 2 includes VCS status by default
lx --vcs-status -l    # add VCS status to any long listing
lx --vcs=jj -ll       # force jj backend
lx --vcs=git -ll      # force git backend
lx --vcs=none -ll     # disable VCS entirely
lx --vcs-ignore       # hide VCS-ignored files
```

The column header (shown with `-h` / `--header`) reflects the
active backend: `Git` or `JJ`.

### jj support is opt-in at compile time

Jujutsu support depends on `jj-lib`, which adds ~5 MB to the binary
and several hundred extra crates to the build.  Enable it at build
time:

```sh
cargo install lx-ls --features jj
cargo build --features jj     # from a checkout
```

Homebrew and pre-built release binaries include jj support.
Without the `jj` feature, `--vcs=jj` returns a clear error.

### Status characters

| Char | Meaning        |
|------|----------------|
| `-`  | Not modified   |
| `M`  | Modified       |
| `A`  | Added (jj)     |
| `N`  | New (git)      |
| `D`  | Deleted        |
| `R`  | Renamed        |
| `C`  | Copied         |
| `I`  | Ignored        |
| `U`  | Untracked      |
| `!`  | Conflicted     |

### Git vs jj display

The VCS column is one or two characters wide depending on the
status.

**Git** uses two characters:

- column 1 is the staged status;
- column 2 is the unstaged status.

When both are the same, `lx` collapses them to one:

| Column | Meaning                                            |
|--------|----------------------------------------------------|
| `-M`   | Unstaged modification (staged: `-`, unstaged: `M`) |
| `M-`   | Staged modification (staged: `M`, unstaged: `-`)   |
| `-N`   | Untracked file                                     |
| `M`    | Same in both columns (collapsed)                   |

**jj** also uses two characters, but with different semantics — jj
has no staging area:

- column 1 is the *change status* (working-copy commit vs parent);
- column 2 is the *tracking status* — a space for tracked files,
  `U` for untracked, `I` for ignored.

| Column | Meaning                 |
|--------|-------------------------|
| `A `   | Added file, tracked     |
| `M `   | Modified, tracked       |
| `- `   | Not modified, tracked   |
| `-I`   | Not modified, ignored   |
| `-U`   | Not modified, untracked |
| `! `   | Merge conflict          |

`--vcs-ignore` works with both backends (under the hood, the jj
backend delegates to `git2` for ignore-file handling so global,
per-directory, and `info/exclude` layers all behave correctly).


## Daily usage patterns

### Column visibility

Every column has a positive flag and a negative flag.  The positive
adds it, the negative removes it:

```sh
lx -ll --no-group              # drop group from tier 2
lx -l --inode                  # add inode to tier 1
lx -l --permissions --no-user  # explicit control
```

Or take full control with `--columns`:

```sh
lx --columns=inode,permissions,size,user,group,modified,vcs
```

Columns added via individual flags are inserted at their canonical
position, not appended — `lx -l -S` places `blocks` between
`size` and `user` where it belongs, regardless of where you
wrote the flag.  See
[The orthogonal CLI](DESIGN.md#the-orthogonal-cli) in DESIGN.md
for the full precedence model, including the `--no-time` bulk
clear and the `-t` / `-tt` / `-ttt` timestamp shortcuts.

The long-name aliases `--mode` (for `--permissions`) and `--size`
(for `--filesize`) are accepted everywhere the canonical name is.
Use whichever reads more naturally for you.

### Filtering

```sh
lx -I '*.tmp|*.bak'             # hide files matching globs
lx -T -P 'target|node_modules'  # show these dirs but don't recurse
lx -TZ -P target                # pruned tree with total sizes (du replacement)
lx -f                           # only files (no directories)
lx -D                           # only directories
```

`-I` / `--ignore` hides files entirely.  `-P` / `--prune` shows
the directory (with its size and metadata) but doesn't recurse
into it — ideal for tree views of projects with large build or
dependency directories.

### Sorting

`lx`'s sort vocabulary is rich: anything you can display as a
column, you can also sort on.  Orthogonality cuts both ways.

```sh
lx -s name            # case-insensitive name (default)
lx -s Name            # case-sensitive (uppercase first)
lx -s size            # smallest first
lx -rs size           # largest first (-r reverses)
lx -s modified        # oldest first
lx -s age             # newest first (alias for reverse-modified)
lx -s ext             # by extension
lx -s none            # unsorted (readdir order)

lx -s permissions     # by permission bits (mode, octal)
lx -s blocks          # by allocated blocks
lx -s user            # by owner name
lx -s uid             # by numeric UID
lx -s version         # natural/version sort (v2.txt before v10.txt)
lx -s vcs             # cluster files by VCS status
```

With `-Z` / `--total-size`, `-s size` sorts directories by
their recursive size.  Combine with `-F` (directories first) or
`-J` (directories last) to partition the listing.

For the full sort vocabulary, including case-sensitive capital-letter
variants and the complete list of column-derived fields, see
[Sorting in DESIGN.md](DESIGN.md#sorting).

### Compounding shortcuts

Two flags compound by repetition:

- **`-l` / `-ll` / `-lll`** — detail tiers for the long view.
  Each tier maps to a named format (`long`, `long2`, `long3`)
  you can override in `[format]`.
- **`-t` / `-tt` / `-ttt`** — timestamp tiers.  `-t` adds
  `modified`; `-tt` adds `modified` and `changed`; `-ttt` adds
  all four (`modified`, `changed`, `created`, `accessed`).

The two compose: `lx -ll -tt` gives you the tier-2 long view with
two timestamp columns added.  `-t` composes with whatever format
you're already using; it doesn't replace it.

Use `--no-time` to clear all timestamps at once — handy when you
want to start from a format that includes timestamps and add back
only the ones you want: `lx -lll --no-time --accessed`.

### Size display

The file size column has three display modes, controlled by
`--size-style` or its short aliases:

```sh
lx -l                 # decimal prefixes: 85k, 1.2M (default)
lx -l -B              # binary prefixes:  83Ki, 1.1Mi
lx -l -b              # raw bytes:        85269
lx -l -K              # decimal (explicit — useful for overriding a personality)
```

Or equivalently:

```sh
lx -l --size-style=binary
lx -l --size-style=bytes
lx -l --size-style=decimal
```

A personality can set `size-style = "binary"` (or `"bytes"` or
`"decimal"`) and the CLI flag overrides it for one invocation.

### Gradients on size and date

Two kinds of column render gradients out of the box: **size**
(5 tiers from byte to huge) and the four **timestamp** columns
— modified, accessed, changed, and created — each with 6 age
tiers from "just now" to "old".  The compiled-in `lx-256` and
`lx-24bit` themes ship gradients tuned to their palette, and
every curated theme in [`themes/`](../themes) defines its tier
colours explicitly.

Switch gradients on or off with `--gradient`:

```sh
lx -lt                              # default: gradients on for everything
lx -lt --gradient=size              # only the size column
lx -lt --gradient=modified          # only the modified column
lx -lt --gradient=size,modified     # size and modified, others flat
lx -lt --gradient=accessed,created  # mix and match per-column
lx -lt --gradient=date              # bulk: every timestamp column
lx -lt --no-gradient                # everything flat
lx -lt --gradient=none              # equivalent to --no-gradient
```

The same vocabulary works as a personality config key:
`gradient = "all"` (default), `"size"`, `"date"` (bulk all
timestamps), `"none"`, or any comma-separated combination
(`"size,modified"`, `"modified,accessed,created"`, etc.).

When a column's gradient is off, it falls back to the theme's
*flat* slots — `size-major`/`size-minor` for size, and the
per-column `date-modified-flat` / `date-accessed-flat` /
`date-changed-flat` / `date-created-flat` for each timestamp
(or the bulk `date-flat` to set all four at once).  All shipped
themes set these explicitly, so a flat column still picks up
the theme's palette rather than a generic fallback.

### Timestamp colours

Timestamps are coloured by age — recent files appear brighter,
older files fade towards grey.  Six tiers:

| Theme key    | Age        | Builtin colour |
|--------------|------------|----------------|
| `date-now`   | < 1 hour   | bright cyan    |
| `date-today` | < 24 hours | cyan           |
| `date-week`  | < 7 days   | bold blue      |
| `date-month` | < 30 days  | blue           |
| `date-year`  | < 365 days | grey           |
| `date-old`   | > 1 year   | dark grey      |

Setting `date = "steelblue"` in a theme sets all six tiers (and
`date-flat`) to the same colour, on every timestamp column.  Set
individual tiers to create a custom gradient, or set `date-flat`
on its own to control the colour `--no-gradient` falls back to.

**Per-column overrides.**  Each timestamp column can be themed
independently.  For each of `modified`, `accessed`, `changed`,
and `created` there's a `date-<col>` bulk setter and seven
per-tier setters with a `date-<col>-` prefix:

```toml
[theme.example]
inherits          = "lx-256"
date              = "white"           # bulk: every column, every tier
date-modified     = "bright green"    # the modified column only
date-accessed-now = "bright magenta"  # only the freshest accessed files
```

**Order matters.**  Theme keys are applied in the order they
appear in the theme block, so write the bulk `date = ...`
setter (and any bulk per-tier setters) *before* per-column
overrides — otherwise the bulk setters will clobber them.  The
example above produces a modified column that's bright green
across the board, an accessed column that's white except for
"now"-tier files (bright magenta), and changed/created columns
that are white throughout.

Per-column overrides are config-file only; the two-letter
`LX_COLORS` codes (`da`, `dn`, ...) keep working as bulk
setters that fan out to all four columns.

### Summary footer

`-C` / `--count` prints an item count to stderr after the listing.
Combine with `-Z` / `--total-size` and the summary also includes
the total recursive size:

```sh
lx -C                 # item count
lx -CZ                # item count + total size
lx -lCZ               # … in a long listing
```

Works in grid view too, not just long view.

### Numeric formatting

By default, lx uses your system locale for decimal points and
thousands grouping.  Two personality config keys let you override
this:

```toml
[personality.default]
decimal-point = ","
thousands-separator = " "
```

Setting them in `[personality.default]` makes them global.  They
apply to **counts** — file sizes (in all `--size-style` modes),
`--total-size` totals, `-CZ` summaries, block counts, and link
counts — but not to IDs (inodes, UID, GID).  Set
`thousands-separator` to an empty string to disable grouping
entirely.

### Environment variables

| Variable          | Purpose                                                    |
|-------------------|------------------------------------------------------------|
| `LX_CONFIG`       | Explicit config file path                                  |
| `LX_COLORS`       | Colour theme (overrides `LS_COLORS`)                       |
| `LX_DEBUG`        | Enable debug logging (`1` or `trace`)                      |
| `LX_GRID_ROWS`    | Minimum rows for grid-details view (also a config key)     |
| `LX_ICON_SPACING` | Spaces between icon and filename (also a config key)       |
| `LX_PERSONALITY`  | Session-level personality selection (see §Personalities)   |
| `LS_COLORS`       | Standard file-type colour scheme                           |
| `COLUMNS`         | Override terminal width                                    |
| `TIME_STYLE`      | Default timestamp style                                    |
| `NO_COLOR`        | Disable colours (see [no-color.org](https://no-color.org)) |


## Shell completions

Completions are available for bash, zsh, and fish.

### Current session only

```sh
# bash
source <(lx --completions bash)

# zsh
source <(lx --completions zsh)

# fish
lx --completions fish | source
```

### Permanent installation

Save the completions to the standard location for your shell:

```sh
# bash
lx --completions bash > ~/.local/share/bash-completion/completions/lx

# zsh (ensure ~/.zfunc is in your $fpath)
lx --completions zsh > ~/.zfunc/_lx

# fish
lx --completions fish > ~/.config/fish/completions/lx.fish
```

Alternatively, add a `source <(lx --completions …)` line to your
shell's rc file to generate completions on the fly at startup.


## Debugging your configuration

Two families of flags help you inspect what `lx` is doing.

**`--show-config`** prints a human-friendly, coloured overview of
the active personality, theme, style, classes, and formats,
including which config file was loaded:

```sh
lx --show-config
```

**`--dump-*`** prints copy-pasteable TOML definitions for any
config object.  Each takes an optional `=NAME` to restrict output
to a single object:

```sh
lx --dump-class                 # all class definitions
lx --dump-class=temp            # just the temp class
lx --dump-format                # all formats
lx --dump-format=long2          # just long2
lx --dump-personality=ll        # the resolved ll personality
lx --dump-style=exa             # the exa style
lx --dump-theme=dracula         # the dracula theme (if loaded)
```

For deeper diagnostics — config-file discovery, theme resolution,
personality cascade — set `LX_DEBUG=1` in the environment and
`lx` will emit trace logging to stderr.


## Further reading

- **[`docs/DESIGN.md`](DESIGN.md)** — the design philosophy behind
  `lx`'s CLI.  Read this when you want to understand *why* things
  are shaped the way they are.
- **[`man/lx.1.md`](../man/lx.1.md)** — long-form command reference.
- **[`man/lxconfig.toml.5.md`](../man/lxconfig.toml.5.md)** —
  complete reference for the configuration file format, including
  every theme key, style syntax, and the full built-in class list.
- **[`CHANGELOG.md`](../CHANGELOG.md)** — release notes.
- **[`themes/README.md`](../themes/README.md)** — how to write
  your own theme.
