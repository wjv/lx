# lx — CLI design principles

> *"I know it when I see it"*
>
> — US Supreme Court Justice Potter Stewart, who was not talking
>   about good CLI design at the time, but might as well have been.

A real problem with trying to enumerate the aspects of a "good"
command-line interface is that it is mostly characterised by
*absences*: the absence of frustration, the absence of friction or painful
sticking points, the absence of time spent hunting for the flag
you need.  When discussing design goals we're forced to enumerate
positive choices, but it's worth bearing in mind that we were
designign for *absences*.

`lx` was born from such frustrations *not* being absent from
the CLIs of existing file listing tools — from POSIX `ls`
through to modern "ls-likes" such as `exa`, the dormant project
from which `lx` was forked.

Convoluted, sub-optimal CLI design leads to usage anti-patterns:

The average user would probably only ever learn the one or two
flags they use most often, and resort to the man page when they
needed more.  Often they would be unaware that their file lister
*could* do more.

The power user would respond by building a
stack of shell aliases so they wouldn't have to remember a whole
lot of flags.  This has been so common that some aliases have
almost become traditional: `ll` for `ls -l`, `la` for `ls -la`.

`lx` attempts to address this with two complementary approaches:

1. **Make the base CLI consistent and approachable** — so you
   don't *need* aliases in the first place.
2. **Replace aliases with something better** —
   *personalities*: named, inheritable, structured bundles of
   settings.

Identifying a problem is not the same thing as solving it, but
try to solve it we did. The process is iterative and
usage-centric: "I think this change will smooth usage, so let's
make it and see if I'm right."  Extracting a list of positive
design goals is something that happens only after the fact.

Try `lx` and decide for yourself whether it succeeds.  Feedback
would be hugely appreciated.

Note: `lx` is not a drop-in replacement for `ls`, `exa`, or any
other ls-like.  Where a consistent design conflicts with legacy
conventions, consistency wins — though well-established
conventions are not broken needlessly.


## Design goals

In no particular order:

1. **Orthogonality.** Flags don't reach across class boundaries to
   do work that belongs elsewhere.  `-Z` (`--total`) only modifies
   the size column — it doesn't pull a size column in, doesn't
   switch to long view, doesn't change the sort field.  If you want
   a size column, ask for it; if you want it shown as a recursive
   total, pair `-Z` with the column flag.  Each flag does one thing
   in one class; the classes (view modes, column selection,
   modifiers, appearance) compose freely without surprises.

2. **Discoverable negations.** Every flag with a meaningful inverse
   has a `--no-*` counterpart.  `--size` adds the file size column;
   `--no-size` removes it.  Short flags negate the same way:
   `--no-z` negates `-z` without your having to remember whether
   the long form is `--size` or `--filesize`.

3. **Compounding shortcuts.** Where a flag has a natural intensity
   dimension, repeating it cranks the dial.  `-l` is a long listing,
   `-ll` shows more columns, `-lll` shows more still.  `-t` does the
   same for timestamp columns.  No new vocabulary to memorise — the
   flag you already know goes further.

4. **Logical and paired short flags.** Widely-used functionality
   gets a short flag, preferably a mnemonic for its purpose (`-u`
   for `--user`, `-g` for `--group`, `-m` for `--modified`).
   Related functionality gets visibly related letters: `-z` enables
   the file si**z**e column; `-Z` enables the recursive total
   modifier on it.  `-d` lists directories as files; `-D` lists
   *only* directories.

5. **Long flags for less-common functionality.** When a feature is
   used rarely, an easy-to-remember verbose flag saves you from
   grepping `--help`.  Individual column-add flags exist for power
   users, but `--columns=size,permissions,user` is the answer when
   you can't remember the short forms.

6. **Guessable flag names.** Beyond canonical names, `lx` accepts
   hidden aliases for spellings users might guess from other
   ls-likes — `--filesize` for `--size`, `--total-size` for
   `--total`, `--ignore-glob` for `--ignore`.  These are silent
   compat shims, not promoted alternatives, but they work when you
   reach for them.

7. **`lx --help` is a designed UI**, not a text dump.  It shows a
   curated overview of the surface without overwhelming you with
   details that can be deduced from the patterns.  If you need the
   full reference, the man page is for that.

8. **Shell completion** is a first-class part of the discoverability
   story.  `lx` ships completions for all common shells (and a few
   uncommon ones).  Tab through your way to the flag you want.

9. **Sane defaults out of the box.** `lx` is usable immediately
    after installation, with no config file required.  When you do
    write one, `--init-config` documents the compiled-in defaults
    without changing them.

## Personalities

Shell aliases have been the standard way to customise `ls` behaviour
for decades.  Personalities are `lx`'s answer to the same need, but
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

| Role            | Shape                                         |
|-----------------|-----------------------------------------------|
| Add             | `--COLUMN` / short                            |
| Suppress        | `--no-COLUMN` / `--no-X` (hidden short alias) |
| Sort ascending  | `-s COLUMN`                                   |
| Sort descending | `-rs COLUMN`                                  |

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


