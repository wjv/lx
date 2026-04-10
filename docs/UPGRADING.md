# Upgrading between lx releases

lx is still in its 0.x series, so the CLI surface is explicitly
allowed to change between minor versions.  This document lists the
breaking changes per release and explains the reasoning — so you
can decide whether to follow along or stay on an older version for
a while.

Each section is written assuming you're coming from the immediately
preceding release.  If you're jumping multiple versions, read them
in order.

- [Upgrading to 0.9](#upgrading-to-09)
- [Upgrading to 0.8](#upgrading-to-08)


## Upgrading to 0.9

0.9 is the **performance and theming release**.  The big changes:

- The `--total-size` traversal is rebuilt for NFS performance
  (10x faster wall time on big NFS trees).
- Three compiled-in theme tiers (`exa` / `lx-256` / `lx-24bit`)
  with automatic selection based on terminal capability.
- Several smaller theme refinements (UID/GID cascade removed,
  age-based date gradient, three-tier group colouring,
  themed `-CZ` footer).

**If you have a 0.5 config file**, run `lx --upgrade-config` to
migrate to the 0.6 schema.  The migration is mostly cosmetic
(version string only) but will inject auto-selection blocks
into your `[personality.default]` section so you get the new
theme tiers automatically.

### Auto-selection of theme tiers

The new compiled-in `default` personality has two `[[when]]`
blocks that override `theme = "exa"` based on the terminal:

| Condition                                  | Theme       |
|--------------------------------------------|-------------|
| (default)                                  | `exa`       |
| `$TERM` matches `*-256color`               | `lx-256`    |
| `$COLORTERM` is `truecolor` or `24bit`     | `lx-24bit`  |

The truecolour block runs second, so on a terminal with both
`xterm-256color` and `COLORTERM=truecolor` (the typical modern
setup), `lx-24bit` wins.

**No action needed** for users with no config file or for users
who run `--upgrade-config`.

**To opt out** (always use the strict 8-colour `exa` theme):
delete the two `[[when]]` blocks from the `[personality.default]`
section of your config file.

**To force a specific theme**, override at the personality or
CLI level:

```toml
[personality.default]
theme = "lx-24bit"  # always, regardless of terminal
```

```sh
lx --theme=exa  # one-off override
```

### `exa` theme is now strict 8-colour ANSI

In 0.9, the compiled-in `exa` theme contains *only* base ANSI
colours — no 256-colour or truecolour values.  The 256-colour
embellishments that crept in during 0.9's gradient work
(`Fixed(244)` for punctuation, the date age gradient) have moved
to `lx-256` and `lx-24bit`.

**If you've been seeing the date gradient on a strict ANSI
terminal**, the gradient will collapse to a single colour
(`Blue.normal()`, the historical exa default).  Use `lx-256` or
`lx-24bit` if your terminal can handle them.  In practice the
auto-selection will pick the right thing for you.

### UID/GID theme cascade removed

In 0.8, the `uid-you` / `uid-other` / `gid-yours` / `gid-other`
theme slots cascaded from `user-you` / `group-yours` when unset.
In 0.9, they are independent slots with no cascade.

**If you have a custom theme** that sets `user-you` but not
`uid-you`: add explicit `uid-you` and `uid-other` entries (and
likewise for `gid-yours` / `gid-other`).  All curated themes and
the builtin default now set all identity slots explicitly.

**If you never customised these slots:** no action needed.

### `--colour-scale` retired; new `--gradient` flag

In 0.8 and earlier, `--colour-scale=none|16|256` controlled the
size column's gradient — but only for the compiled-in `exa`
theme, and only the size column.  The flag's name suggested
something broader than what it actually did, the `16`/`256`
distinction was about gradient depth (no longer relevant now
that themes ship their own values), and the timestamp gradient
had no equivalent knob at all.

In 0.9 it's replaced by **`--gradient`**, a per-column on/off
switch covering the size column and each of the four timestamp
columns individually:

```sh
lx -lt                                # default: gradient on for everything
lx -lt --gradient=size                # size only, flat timestamps
lx -lt --gradient=date                # bulk: every timestamp column on
lx -lt --gradient=modified            # only the modified column
lx -lt --gradient=size,modified       # size and modified, others flat
lx -lt --gradient=accessed,created    # mix and match per-column
lx -lt --gradient=all                 # everything on
lx -lt --gradient=none                # everything flat
lx -lt --no-gradient                  # alias for --gradient=none
```

The same vocabulary works in a personality:

```toml
[personality.minimal]
inherits = "ll"
gradient = "none"          # always render flat columns
```

**Default flips from off to on.**  Today's `--colour-scale`
defaults to `none` (the size column is a flat green under the
`exa` theme).  In 0.9 the default is `all`: any theme that ships
gradients (`lx-256`, `lx-24bit`, the curated `the-exa-future`,
all the dropped-in themes) shows them out of the box.

The strict `exa` theme is unaffected — its size tier values are
all "green-bold" anyway, so the on/off switch is visually
indistinguishable.

**Off-state colours are themeable.**  With `--no-gradient`, the
size column collapses to the theme's `size-major` (numbers) and
`size-minor` (units) slots, and each timestamp column collapses
to its own `date-<col>-flat` slot (or the bulk `date-flat`,
which fans out to all four).  All these slots existed (or were
added) explicitly across the curated themes, so `--no-gradient`
produces a sensibly themed flat column rather than a fall-through
mess.

**Per-timestamp-column theming is new in 0.9.**  The four
timestamp columns (modified, accessed, changed, created) used to
share one set of `date-*` colours; in 0.9 each column can be
themed independently via `date-<col>` and `date-<col>-<tier>`
keys (32 new keys total).  Existing themes are unaffected — the
unprefixed `date-*` keys still fan out to all four columns.  See
the GUIDE for the worked example.

**`--upgrade-config` migrates old configs automatically.**  Run
`lx --upgrade-config` and any `colour-scale = "..."` line inside
a `[personality.X]` (or `[[personality.X.when]]`) block is
rewritten:

| Old value                 | New value          |
|---------------------------|--------------------|
| `colour-scale = "none"`   | `gradient = "none"` |
| `colour-scale = "16"`     | `gradient = "all"`  |
| `colour-scale = "256"`    | `gradient = "all"`  |

The bit-depth distinction is gone: `16` and `256` both meant
"draw the gradient", which is now `all`.

**Using `--colour-scale` on the CLI is now an error**, with the
error message pointing at `--gradient`.  Same for the
`colour-scale = "..."` personality key if you skip
`--upgrade-config`.

### Curated themes have deliberate gradients

Most of the curated themes shipped in `themes/*.toml` previously
used a single bulk `size-number = "..."` setter and (in some
cases) `date = "..."`, so under the new default
(`--gradient=all`) they would have rendered visually flat
anyway.  In 0.9 every curated theme defines its size and date
tier colours explicitly with a deliberate, palette-appropriate
gradient.  If you've been using a curated theme as-is, expect
sizes and timestamps to suddenly show distinct tier colours.

If you preferred the flat look, run `lx --no-gradient` (or set
`gradient = "none"` in your personality).

### Three-tier group colouring

Group columns now distinguish three tiers:

- **`group-yours`** / **`gid-yours`** — your primary group
- **`group-member`** / **`gid-member`** — a supplementary group
  you belong to
- **`group-other`** / **`gid-other`** — not your group at all

Custom themes can set the new `group-member` / `gid-member` keys.
If unset, they default to the same value as `group-yours` /
`gid-yours`.

### `permissions-*` theme keys

The theme parser now accepts `permissions-user-read`,
`permissions-user-write`, etc. — matching the column name.  The
legacy `perm-*` short form is still accepted as an alias.  This
fixes a long-standing bug where curated themes that used the
longer form were silently ignored.

### `LX_COLORS` has been removed

The `LX_COLORS` environment variable is gone in 0.9.  It was an
lx-specific extension to `LS_COLORS` with ~60 two-letter codes
(`ur`, `uw`, `nk`, `da`, etc.) for lx's own columns.  Every one
of those codes has a long-standing config-file equivalent in
`[theme.NAME]` sections — and the config file is strictly more
powerful (inheritance, per-column overrides, per-tier gradients,
X11/hex colours, smooth interpolation).

**Why:** the two-letter namespace had run out of room.  Recent
additions were already reaching for mixed case (`Uy`, `Gb`,
`bO`) and awkward mnemonics (`in`, `do`, `xx`), and new theming
features (per-timestamp-column keys, per-tier gradients) couldn't
be expressed in two letters at all.  Rather than keep papering
over the cracks, lx now has a single canonical theming path:
`LS_COLORS` for the standards-compatible floor, the config file
for everything else.

**Nothing to do** if you never used `LX_COLORS`.

**If you *did* use `LX_COLORS`**, move the settings into a
`[theme.NAME]` section and activate the theme through a
personality (or `--theme=NAME`):

Before (in `.bashrc` or `.zshrc`):

```sh
export LX_COLORS='ur=38;5;100:uw=38;5;101:nk=32:da=90'
```

After (in `~/.lxconfig.toml`):

```toml
[theme.mine]
inherits                 = "lx-24bit"
permissions-user-read    = "colour-100"
permissions-user-write   = "colour-101"
size-number-kilo         = "green"
date                     = "bright-black"

[personality.default]
theme = "mine"
```

Every `LX_COLORS` code has a config-key equivalent; the full
list is in `lxconfig.toml(5)` or visible via `lx
--dump-theme=exa`.

**Glob precedence change.**  `LX_COLORS` used to let you
override `LS_COLORS` globs (e.g. setting `*.log=32` in
`LX_COLORS` would beat `*.log=31` in `LS_COLORS`).  With
`LX_COLORS` gone, lx sees only whatever `LS_COLORS` says.  If
you relied on this, either edit `LS_COLORS` directly or define
a class in `[style.NAME]` (class-based colours win over
`LS_COLORS` globs).

**`LS_COLORS` is unchanged** and continues to work exactly as
before, both for the standard file-type keys (`di`, `ex`, `ln`,
...) and for filename glob entries.

### Broken config files are now fatal

In 0.8 and earlier, lx would emit a warning and continue with
compiled defaults if your config file failed to parse, used an
unsupported schema version, or contained a `[theme.X]`
inheritance cycle.  In 0.9 these are *errors*: lx prints the
problem and exits non-zero (exit code 1 for I/O / parse errors,
3 for theme cycles).

The motivation is that a silent fallback hides real bugs — if
you've taken the trouble to write a config file, an invalid line
should tell you about itself, not vanish.

**No action needed** for users with a working config file or no
config file at all.

**If you see a new error on launch**, the first line will tell
you what's wrong.  Most likely:

- **TOML parse error**: edit the file or restore an earlier
  version.  `lx --upgrade-config` keeps a `.bak` of your last
  successful upgrade.
- **`needs upgrade`**: run `lx --upgrade-config` to migrate the
  schema version.
- **`theme inheritance cycle`**: a chain of `inherits = "..."`
  loops back on itself.  Break the loop and try again.

The `--upgrade-config` flow is unaffected — it still reads the
config file directly and runs even when the schema is too old
for normal operation.


## Upgrading to 0.8

0.8 is the **CLI refactor release**.  It reshapes the flag surface
around a consistent, orthogonal design — the kind where you can
predict what a flag will do without looking it up.  Most of the
work was *additive*: new columns, new short flags, new sort
fields, a compounding timestamp shortcut.  A handful of older
flags fell out as consequences, which is where the breakage lives.

If you're happy on 0.7, nothing forces you to upgrade.  But 0.8's
flag surface is the one lx is going to grow on from here, so the
sooner you make the jump the less churn you'll see later.

The good news: `lx --upgrade-config` will migrate your config file
automatically (and save a `.bak` of the original), so the path from
a 0.7 setup to a working 0.8 setup is a single command plus a
handful of muscle-memory adjustments.

The rest of this section is arranged around *what 0.8 adds*, with
the removals from 0.7 listed under each addition as the price of
doing business.

### 1. UID and GID are first-class columns

```sh
lx -l --uid                     # add a numeric UID column
lx -l --uid --gid               # numeric UID and GID
lx -l --user --uid --gid        # names *and* numeric IDs, side by side
lx -l --columns=user,uid,gid,name -s uid    # audit view sorted by numeric UID
```

`--uid` and `--gid` are long-only column add flags, with
`--no-uid` / `--no-gid` to match.  They sit at their own canonical
positions (immediately after `user` and `group`), so individual
adds land where you'd expect them.  They're also sortable — you
can sort by numeric UID without displaying the column.

**Why:** the old story was that lx had a *render-mode* toggle
(`-n` / `--numeric`) that swapped the contents of the existing
user and group columns from names to numbers.  That meant you
could see names *or* numeric IDs, never both; you couldn't sort
by UID without showing it; and you couldn't display just one of
the two.  Promoting UID and GID to real columns closes all three
gaps at once.

**⚠️  Consequence: `-n` / `--numeric` has been removed.**  The
old render toggle had nothing to do once the columns existed in
their own right.  If you were typing `lx -ln` and want the same
name-less view back:

```sh
lx -l --uid --gid --no-user --no-group    # explicit, one-off
```

or (better) define it once as a personality:

```toml
[personality.numeric]
inherits = "ll"
uid = true
gid = true
no-user = true
no-group = true
```

and invoke with `lx -p numeric` or symlink `numeric → lx`.

Users with `numeric = true` in an older config will see a
deprecation warning with the same migration guidance on next load.

### 2. Compounding `-t` / `-tt` / `-ttt` timestamp tiers

```sh
lx -l -t          # long + modified        (same as -l --modified)
lx -l -tt         # long + modified, changed
lx -l -ttt        # long + modified, changed, created, accessed
lx -ll -tt        # tier-2 long view plus modified+changed on top
```

`-t` compounds exactly like `-l`: repeat it to add more timestamp
columns.  Unlike `-l`, it composes with whatever format or tier
you're already using — it desugars into individual add flags.

You can still reach for any single timestamp directly:
`--modified` / `--changed` / `--accessed` / `--created` (and
their `--no-*` counterparts).  Each is a first-class add/suppress
flag like every other column.

**Why:** before 0.8, `-t FIELD` was somewhat magical: it picked a single 
timestamp to show in *the* timestamp column — one at a time, modified or 
accessed but never both.  In practice most people's "which timestamp do 
I want?" question really means "let me see more than one so I can tell them 
apart", and the old flag couldn't answer that.  The compounding tiers put every 
timestamp on the table.

**⚠️  Consequence: `-t FIELD` and `--time` have been removed.**
If you were typing `lx -l -t accessed`, the closest one-flag
replacement is:

```sh
lx -l --accessed --no-modified    # exactly one timestamp: accessed
```

or, if "clear the defaults, then show just these" is a common
pattern for you:

```sh
lx -l --no-time --accessed        # --no-time is a bulk clear that runs first
```

Config files with `time = "..."` in a personality section are
migrated automatically with a deprecation warning — the setting
becomes a no-op and the equivalent per-timestamp boolean is set
instead.

### 3. New short flags for the most common long-view columns

Three new single-letter flags cover the columns people reach for
most often:

| Flag | Long form                   | Column                         |
|------|-----------------------------|--------------------------------|
| `-u` | `--user`                    | Owner name                     |
| `-M` | `--permissions` (`--mode`)  | Symbolic permission bits       |
| `-z` | `--filesize` (`--size`)     | File size                      |

These are additive on the long view (they add the column if the
current format doesn't already include it, and are no-ops if it
does).  Every one has a hidden `--no-X` negation for symmetry.

**Why:** the long view is where lx spends most of its time, and
three of its most visible columns had no short flag.  `-u`
reads as "user"; `-M` matches traditional Unix vocabulary
(`chmod`, `stat`, the "file mode bits") and the Windows-side
column header; `-z` was the shortest unused letter near `--size`
(`-s` is `--sort`).

**⚠️  Consequences: `-u` is no longer `--accessed`; `-U` is no
longer `--created`.**  Both letters used to be pinned to
individual timestamps and were reassigned as part of the
reshuffle: `-u` went straight to `--user`, and `-U` was freed
outright, reserved for future use.

At first glance, losing the short flags for two out of four
timestamp columns looks like a regression.  It isn't.  Between
§2's compounding `-t` and the per-timestamp long flags, it has
become *easier* to pull in whatever timestamps you want — all
you have to remember is that to see more of them, you add
another `-t`.

| 0.7         | 0.8        |
|-------------|------------|
| `lx -lmc`   | `lx -ltt`  |
| `lx -lmcuU` | `lx -lttt` |

See?  Easier.

### 4. Size display: `-b`/`-B` swapped, `-K` and `--size-style` added

The three size-display modes — decimal prefixes (k, M, G), binary
prefixes (KiB, MiB), and raw bytes — now have a canonical valued
flag and three short aliases:

| Flag | Long form    | Mode                            |
|------|--------------|---------------------------------|
| `-K` | `--decimal`  | Decimal prefixes (default)      |
| `-B` | `--binary`   | Binary prefixes (KiB, MiB, GiB) |
| `-b` | `--bytes`    | Raw byte count                  |

All three are also expressible as `--size-style=decimal`,
`--size-style=binary`, and `--size-style=bytes`.  The valued flag
parallels `--time-style` and is the canonical form; the short
flags are aliases.  Personality config key: `size-style = "decimal"`.

**⚠️  Breaking: `-b` and `-B` have swapped.**  In 0.7, `-b` was
`--binary` and `-B` was `--bytes`.  The new assignment puts the
simple, original display (raw bytes, lowercase) on the lowercase
letter and the formatting system (Binary prefixes, uppercase) on
the uppercase letter.

If you had `-b` or `-B` in scripts or muscle memory, swap them.
The long forms `--binary` and `--bytes` are unchanged.

**`-K` / `--decimal` is new.**  It explicitly selects the default
decimal-prefix mode — useful for overriding a personality that
sets `binary` or `bytes` back to the default.  `-K` for Kilo, the
most recognisable decimal prefix.

**`--size-style` closes an asymmetry.**  In 0.7, a personality
that set `binary = true` could not be overridden back to decimal
from the command line — there was no flag for it.  Now there is:
`--size-style=decimal`, or `-K`, or just `--decimal`.


### 5. Sort on every column you can display

```sh
lx -ls blocks              # sort by allocated blocks
lx -ls permissions         # sort by permission bits (numeric octal)
lx -ls user                # sort by owner name, case-insensitive
lx -ls uid                 # sort by numeric UID
lx -ls version             # natural / version sort on names (v2.txt < v10.txt)
lx -ls vcs                 # cluster files by VCS status
```

Every metadata column — `permissions`, `blocks`, `links`, `flags`,
`user` / `User`, `group` / `Group`, `uid`, `gid`, plus the new
`version` and `vcs` fields — is a valid `--sort` value in 0.8.

**Why:** before 0.8, `--sort` understood `name`, `size`,
`modified`, `extension`, and a couple of related aliases.  That
was an arbitrary subset: there was no reason to privilege "size"
and "modified" over "blocks" and "user" other than inherited
habit.  The orthogonal rule now is "if it's a column, it's
sortable".

No removals fall out of this one — it's pure addition.

### 6. `perms` → `permissions` (column canonical rename)

```sh
# Both work in 0.8:
lx --columns=permissions,size,user,modified
lx --columns=perms,size,user,modified
```

The CLI flag is `--permissions`, the theme keys are
`permissions-user-read` etc., and the config setting is
`permissions`.  The column vocabulary now matches: the canonical
column name is `permissions`, with `perms` retained as an alias
in `--columns=` and `[format]` definitions for backward
compatibility.  The only place `perms` is *not* accepted is as
a value for `-s` / `--sort`, where only the canonical
`permissions` (and its aliases `mode` / `octal`) will work.

**Why:** having the column name as `perms` was the one place a
different word was used.  Closing that gap keeps the whole
vocabulary internally consistent.

### 7. Config schema 0.4 → 0.5

Old config files (version 0.1 through 0.4) are migrated
automatically by `lx --upgrade-config`.  The only personality
settings that have been removed are the two tied to the changes
above — `time = "..."` (replaced by the per-timestamp booleans)
and `numeric = true` (replaced by `uid` / `gid` / `no-user` /
`no-group` or a `numeric` personality).  The migration drops
them, emits a deprecation warning, and saves a `.bak` of the
original.

### Staying on 0.7 for now

Every release tag is available on GitHub and crates.io, so
pinning to 0.7 is a fine choice if the 0.8 changes land at an
awkward moment:

```sh
cargo install lx-ls --version '^0.7'
```

0.7 will not receive further updates (no back-ports), so the
usual advice applies: staying on an older version is a temporary
measure, not a destination.

### Summary table

| Change                            | How to adapt                                                   |
|-----------------------------------|----------------------------------------------------------------|
| `--uid` / `--gid` as columns      | Nothing required — new long-only flags                         |
| Compounding `-t` / `-tt` / `-ttt` | Nothing required — new short flag                              |
| New: `-u` / `--user`              | Nothing required — new short flag                              |
| New: `-M` / `--permissions`       | Nothing required — new short flag                              |
| New: `-z` / `--filesize`          | Nothing required — new short flag                              |
| New: `--size-style`, `-K`         | Nothing required — new valued flag + short alias               |
| **`-b` / `-B` swapped**           | `-b` is now `--bytes`; `-B` is now `--binary`. Swap them.      |
| Expanded `--sort` vocabulary      | Nothing required — new sort fields                             |
| `-n` / `--numeric` removed        | `--uid --gid --no-user --no-group`, or a `numeric` personality |
| `-t FIELD` / `--time` removed     | `--FIELD` / `--no-time --FIELD` / `-t`/`-tt`/`-ttt`            |
| `-u` was `--accessed`             | Use `--accessed` (long) or `-ttt`                              |
| `-U` was `--created`              | Use `--created` (long) or `-ttt`                               |
| `perms` → `permissions`           | Optional; `perms` still works in `--columns=` and `[format]`   |
| Config schema bump                | `lx --upgrade-config` (keeps a `.bak`)                         |

For the design rationale behind the orthogonal flag surface as a
whole — why it's shaped the way it is, not just what changed —
see [`DESIGN.md`](DESIGN.md#the-orthogonal-cli).
