# Upgrading between lx releases

lx is still in its 0.x series, so the CLI surface is explicitly
allowed to change between minor versions.  This document lists the
breaking changes per release and explains the reasoning — so you
can decide whether to follow along or stay on an older version for
a while.

Each section is written assuming you're coming from the immediately
preceding release.  If you're jumping multiple versions, read them
in order.

- [Upgrading to 0.8](#upgrading-to-08)


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

### 4. Sort on any column you can display

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

### 5. `perms` → `permissions` (column canonical rename)

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

### 6. Config schema 0.4 → 0.5

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
