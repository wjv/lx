# The `lx` user guide
<!-- vim: set fo+=at fo-=w tw=72 cc=73 :-->

This is the tutorial for `lx`, the file lister with personality.  It is
organised as something you can read top-to-bottom the first time you
configure `lx`, and dip into later when you want to change something
specific.

`lx` works perfectly well out of the box with zero configuration —
none of this document is required
reading before you can use it!  If you've just installed `lx` and
want to see what it does, skip to [First run](#first-run) and come
back to the configuration chapters once you've decided you want to
tweak something.


## How this guide fits in

- **`lx --help`** is the go-to flag reference.  Every commonly used flag
  is listed there with a one-line description.
- **`man lx`** is the long-form, complete command reference.  Look here
  for the exact semantics of a flag. We'll often refer to this as the
  lx(1) man page.
- **`man lxconfig.toml`** is the reference for the configuration file
  format.  Every key, every section, every accepted value. We'll refer
  to this as the lxconfig.toml(5) man page.
- **This guide** is narrative and example-first.  It shows you how to
  get things done and teaches you the concepts as it goes.

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

The best way to start using `lx` is to just start using it.  It's close
enough to stock `ls` that you won't be lost, even though many flags
differ. When you're stuck, `lx --help` is your friend.

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


## The file lister with personality! 🌟

Since Unix shells first supported command aliasing, people have
aliased `ls` to call it with their favourite options.  For decades
`ll` has been shorthand for `ls -l`:

```sh
alias ll="ls -l"
```

`lx` takes this idea and promotes it into a first-class feature.
A *personality* is a named bundle of settings that `lx` adopts
*based on the name it was invoked under*.  To duplicate the effect
of the shell alias above, you would:

1. Define a personality in the config file:

   ```toml
   [personality.ll]
   format = "long"
   ```

2. Create a symlink to `lx` under that name, somewhere in your `$PATH`:

   ```sh
   ln -s $(which lx) ~/.local/bin/ll
   ```

When you run `ll`, `lx` sees the symlink's name, looks up the matching
personality, and applies it — no shell alias needed.  If you invoke `lx`
as itself, it uses the compiled-in `lx` personality.

`lx` ships pre-configured with a couple of personalities that recreate
shell aliases that are commonly used, like `ll` for a long listing with
multiple columns, and `la` for a long listing that includes hidden
files.

We'll have a lot more to say about personalities later!


## Using `lx`

`lx` is a file lister. It's a modern and extensive alternative to the
UNIX `ls` command.

`lx` *only lists* files. It never looks into your files. It never moves
or renames your files. It certainly never deletes them. `lx` is **not**
a file manager. It's a file lister.

The authoritative reference for everything in this section is the lx(1)
man page. A quick, curated reference is available using `lx`'s `--help`
flag:

```sh
lx --help           # Show online help
```

### Grid mode, one-line mode, and long mode

Just like `ls`, `lx` has three primary display modes.

**Grid mode** is the default — it packs entries into columns sized to
the terminal width, just like `ls`.  Use `-G` / `--grid` to request it
explicitly (though you rarely need to, since it's the default). `-x`
/ `--across` sorts entries across rows instead of down columns.

**One-line mode** (`-1` / `--oneline`) prints one entry per line.
Useful when piping to other commands or when filenames are long
enough that a grid wastes space.

**Long mode** (`-l` / `--long`) is the one you'll probably use the most.
It shows a detailed table: one row per file, with columns for
permissions, size, owner, timestamps, and more.  In `lx`, Long mode has
three detail tiers controlled by repeating `-l`:

```sh
lx -l                 # tier 1: permissions, size, user, modified
lx -ll                # tier 2: + group, VCS status
lx -lll               # tier 3: + header, all timestamps, links, blocks
```

`lx`'s three long mode displays are fully configurable. Each tier's
column set is defined by a named *format* that you can redefine in the
[configuration file](#the-configuration-file).

### Recursive and Tree listings

Two additional modes layer on top of the basic modes. (You may have seen
equivalents in other ls-likes.)

- **Recurse mode** (`-R` / `--recurse`) lists subdirectories
  recursively.
- **Tree mode** (`-T` / `--tree`) goes even further and shows directory
  structure as an indented tree.

You can limit the recursion depth of both with `-L`.

Both modes also combine with long mode: `lx -lT` gives a long tree,
`lx -lR` a long recursive listing.

A long tree is an especially useful view. Try it in your own
directories!

```sh
lx -llTL2             # tier 2 long mode, with tree, 2-level depth
```

### Column visibility

Unlike traditional `ls` and even most modern ls-likes, `lx` gives you
full control over which columns are shown in long view.

You've already seen that `lx` supports three tiers of detail — `-l`,
`-ll`, and `-lll` — which each displays a different set of columns. But
`lx` goes further than this: You can choose to display (or hide)
individual columns. You can even change the *order* in which columns are
displayed.

Every column has a flag that forces it to be shown — let's call it
a "positive" flag. For instance, if you want to show the column
containing inode numbers of the files being listed, you use `--inode`:

```sh
lx -l --inode              # add inode to tier 1 long listing
```

Every column also has a "negative" flag to suppress showing it. If you
explicitly do not want to show the inode numbers, use `--no-inode`.

You can use negative flags to suppress columns that are included in
whichever column format you're using. For instance, if you're using the
`-ll` long format but you don't want to see the group name, add
`--no-group`.

```sh
lx -ll --no-group           # drop group from tier 2 long listing
```

Many columns also have a short positive flag. To show the inode numbers,
you can also just use `-i` instead of `--inode`. Such short flags are
quite common for UNIX command-line utilities. But `lx` also supports
a less common practice — let's call it "short suppressors": Every short
flag also has a "negative" counterpart. If you want to suppress the
display of the inode column, you can use either `--no-inode`, or
`--no-i`.

```sh
lx -ll --no-g               # drop group from tier 2 long listing
lx -l --inode --no-user     # explicit control
```

Internally, `lx` maintains a *canonical* ordering of the long view
columns. When you use flags to add columns to the long view, they are
inserted at their canonical position; they are not just appended to the
end.

For instance, if you use `lx -l -S` to show the block size of files on
disk, `lx` places the `blocks` column between `size` and `user` where it
belongs, regardless of where on the command-line you used the flag.

For a full list of available columns, see the lx(1) man page, or the
"Long view" section in `--help`.

### Column headers

The `-h` / `--header` flag prints a neat header row with column names
above the columns.

### Explicit column list, and reordering columns

You can take full control both of *which* columns are shown, as well as
the *order* in which they're shown, by using the `--columns` flag. The
`--columns` flag takes a comma-separated list of column names: 

```sh
lx --columns=inode,permissions,size,user,group,modified,vcs
```

Some of the column names have aliases that may be shorter and easier to
remember. For instance, you can use `mode` instead of `permissions`. For
a full list, see the lx(1) man page.

### Compounding flags

Several flags compound by repetition. One of them you've already seen:

- `-l` / `-ll` / `-lll` — detail tiers for the long view
- `-t` / `-tt` / `-ttt` — timestamp tiers
- `-a` / `-aa` — show dotfiles, then also `.` and `..`
- `-@` / `-@@` — show the `@` indicator on files with extended
  attributes, then also list the attributes themselves

Files and directories on UNIX filesystems can have up to four
timestamps: "modified", "changed", "created" and "accessed". `lx` can
display each of these timestamps as a separate column (of course) and
provides flags to control their display (of course). You can look up
these flags in the `--help` or the lx(1) man page. (Their names are
unsurprising.)

But far easier is the compounding `-t` flag (which has no long variant).

- A single `-t` adds `modified`
- `-tt` adds `modified` and `changed`
- `-ttt` adds all four — `modified`, `changed`, `created`, `accessed`

The two compose: `lx -ll -tt` gives you the tier-2 long view with
two timestamp columns added. `-t` composes with whatever format
you're already using; it doesn't replace it.

Use `--no-time` to clear all timestamps at once — handy when you
want to start from a format that includes timestamps and add back
only the ones you want:

```sh
lx -lll --no-time --accessed    # show only accessed timestamp
```

A note on `-@`/`-@@`: probing for extended attributes is cheap on
Linux but disproportionately expensive on macOS (its `listxattr`
can dominate tree-traversal time on APFS).  As a result, `lx`
ships with the `@` indicator off by default *only* on macOS.
Linux and BSD users see no behavioural change.  macOS users who
want the indicator can opt in per invocation with `-l@`, or
permanently with `xattr-indicator = true` in their personality.
This is a showcase of the `platform` predicate for `[[when]]`
blocks; see "Conditional overrides" below for the gory details.

### Filtering the file listing

Like `ls`, `lx` uses the `-a` flag to show hidden (dot) files, which are
hidden by default. Unlike `ls`, `lx` requires you to use `-a` *twice* to
show the `.` and `..` entries (current and parent directory,
respectively). `lx` also provides a long flag `--all` that does the same
thing as `-a`.

You can use `-D` / `--only-dirs` and `-f` / `--only-files` to filter the
file listing to only show directories or only files, respectively.

Just like `ls`, `lx` uses `-d` to list directories as plain files. In
`lx`, you can also use the long flag `--list-dirs`.

```sh
lx -f                           # only files (no directories)
lx -D                           # only directories (no plain files)
lx -ld target                   # show 'target' dir, not its contents 
```

`-I` / `--ignore` excludes files from the listing based on a glob
pattern. You can ignore multiple glob patterns by
separating them with a pipe character (`|`).

> Note: You should quote the glob pattern to prevent the
  shell from expanding it.

```sh
lx -lI '*.tmp|*.bak'            # hide .tmp and .bak files from listing
```

`-P` / `--prune` takes pipe-separated glob patterns, just like `-I`. If
the patterns match a directory, that directory is shown in the long
listing, but is not recursed into. `-P` mainly makes sense when combined
with one of the recursive flags — `-R` or `-T`. It's ideal, for example,
for excluding large build or dependency directories from tree views of
projects.

```sh
lx -T -P 'target|node_modules'  # show these dirs but don't recurse
lx -TZ -P target                # pruned tree with total sizes (du)
```

### Sorting

`lx`'s sort vocabulary is rich: anything you can display as a
column, you can also sort on. Orthogonality cuts both ways.

It's easiest to show with some examples. (Note that `-r` / `--reverse`
reverses the sort order.)

```sh
lx -s name            # case-insensitive name (default)
lx -s Name            # case-sensitive (uppercase first)
lx -s size            # smallest first
lx -rs size           # largest first (-r reverses)
lx -s modified        # oldest first
lx -s age             # newest first (alias for reverse-modified)
lx -s ext             # sort by extension
lx -s none            # unsorted (readdir order)
```

Some more niche examples:

```sh
lx -s permissions     # by permission bits (mode, octal)
lx -s blocks          # by allocated blocks
lx -s user            # by owner name
lx -s uid             # by numeric UID
lx -s version         # natural/version sort (v2.txt before v10.txt)
lx -s vcs             # cluster files by VCS status
```

Independent of the sort order established by `--sort`, you can also
choose to display directories first or last by using the `--group-dirs`
flag.

```shell
lx -l --group-dirs=none   # directories sorted like files (default)
lx -l --group-dirs=first  # directories first
lx -l --dirs-first        # alias for --group-dirs=first
lx -l --group-dirs=last   # directories last
lx -l --dirs-last         # alias for --group-dirs=last
```

`lx` also provides you with a more succinct way to do the same thing:

```sh
lx -lF                    # directories first
lx -lJ                    # directories last
```

For the full sort vocabulary, including case-sensitive capital-letter
variants and the complete list of column-derived fields, see the lx(1)
man page. The `--help` gives an overview.

### Size display

The file size column has three display modes, controlled by
`--size-style` or its short aliases:

```sh
lx -l -K              # decimal prefixes: 85k, 1.2M (default)
lx -l -B              # binary prefixes:  83Ki, 1.1Mi
lx -l -b              # raw bytes:        85269
```

Or equivalently:

```sh
lx -l --size-style=binary
lx -l --size-style=bytes
lx -l --size-style=decimal
```

Out of the box, decimal is the default and will be displayed by a plain
`lx -l`. (This is one of the many things you may want to configure using
personalities.) 

### Summary footer, and the multi-purpose `-Z`

`-C` / `--count` prints an item count to stderr after the listing.

Combine with `-Z` / `--total` and the summary also includes
the total size of the listed items:

```sh
lx -C                 # item count
lx -CZ                # item count + total size
lx -lCZ               # … in a long listing
```

But `-Z` does more than that: In a recursive or tree listing (`-R` or
`-T`), it prints the total recursive size of every listed directory.

```sh
lx -lTZ -L2           # 2-level tree with directory sizes
```

> Note: The two seemingly independent effects of `-Z` are actually
  complementary and not in conflict: If you want to see the total size
  of the currently displayed items in the footer, it would not make
  sense to exclude the recursive sizes of any directories being
  displayed from that total.

When you're sorting by size (`-s size`) the recursive directory sizes
are used to sort directories.

```sh
lx -lTZL2 -rs size    # recursive 2-level tree in reverse size order
lx -lTCZL2 -rs size   # … with a total count/size footer
```

Now these flags are starting to compose into something truly useful! You
may think these are a lot of flags to remember, but bear in mind: We haven't
really started digging into the power of personalities yet!

Note that `-C` (and `-CZ`) only count the *displayed* items:

```sh
lx -lC *.txt          # How many .txt files in this dir?
lx -lCZ *.txt         # …and what is their total size? (Useful!)
```

No more piping `ls` to `wc -l`!

### Gradients on size and date

Two kinds of column can be rendered using colour gradients to give you
a quick visual impression of their range.

The file size column is coloured in up to five tiers:
- tiny: bytes
- kilobytes
- megabytes
- gigabytes
- huge: terabytes or bigger

The timestamp columns (all four of them) are each rendered in up to six
different colours, depending on the age they represent:
- "hot", < 1 hour
- "warm", < 24 hours
- < 7 days
- < 30 days
- < 365 days
- "cold", > 1 year

Switch gradients on or off with `--gradient` flag, which can take
a comma-separated list of column names as well as some special values:

```sh
lx -lt                                # default: gradients on for everything
lx -lt --gradient=size                # only the size column
lx -lt --gradient=modified            # only the modified column
lx -lt --gradient=size,modified       # size and modified, others flat
lx -lttt --gradient=accessed,created  # mix and match per-column
lx -lttt --gradient=date              # bulk: every timestamp column
lx -lt --no-gradient                  # everything flat
lx -lt --gradient=none                # equivalent to --no-gradient
```

### Smooth truecolour gradients

When using a 24-bit ("truecolour") colour scheme (default on any
terminal that supports it), you can tell `lx` to smoothly interpolate
between the fixed colours in the gradient by using the `--smooth` flag.

This is useful for creating a more visually appealing gradient,
especially when dealing with a large number of files or directories.

The interpolation formulas come from Björn Ottosson's 2020
paper on Oklab, <https://bottosson.github.io/posts/oklab/>.

> Note: On a terminal that doesn't support truecolour, `--smooth` is a no-op.


## Personalities and Formats

Personalities are a signature feature of `lx`. We've touched on the
concept briefly before, and now it's time to take a deeper dive!

A personality is a named bundle of settings. There are a couple of ways
to activate a personality, but the intended method is to put a symlink
with the same name as the personality in your `$PATH`, and point that
symlink at the `lx` binary.

Execute `lx` by calling `ll`, and the `ll` personality is automatically
active. Voilà!

Every single CLI flag is usable in a personality. There's nothing that
you can do with `lx` interactively that you cannot encode in
a personality!

Personalities are created in `lx`'s [configuration
file](#the-configuration-file). We'll discuss this file in more detail
later. For now, all you need to know is that it's
a [TOML](https://toml.io/) file.

### How to create a personality

A personality is defined by a block in the TOML configuration file that
starts with a `[personality.NAME]` header:

```toml
[personality.stree]
description = "Tree view, biggest files first"
tree = true
group-dirs = "first"
sort = "size"
reverse = true
```

The optional `description` is a one-line summary that `--show-config`
surfaces in its catalogue and that `--dump-personality` emits.  Skip
it if you don't care; it's purely informational.

If I were to run `lx` with this personality, it would be the same as if
I executed:

```sh
lx --tree --group-dirs=first --sort=size --reverse  # long flags
lx -TF -rs size                                     # same thing, short flags
```

Note that the configuration keys you use when defining a personality are
*exactly the same* as the long CLI flags!

*Every* CLI flag has a corresponding configuration key!

Boolean flags take `true`/`false` when used as a configuration key:

| flag         | configuration key |
|--------------|-------------------|
| `--tree`     | `tree = true`     |
| `--no-group` | `group = false`   |

Scalar flags used as configuration keys take a string or number value:

| flag          | configuration key |
|---------------|-------------------|
| `--sort=size` | `sort = "size"`   |
| `--level 2`   | `level = 2`       |

And list-valued flags take TOML arrays:

| flag                     | configuration key                |
|--------------------------|----------------------------------|
| `--columns=size,modified` | `columns = ["size", "modified"]` |

> You'll find a complete reference of all configuration keys in the
  lxconfig.toml(5) man page.

### Using personalities

Let's say we've created the `stree` personality in the previous section.
How do we use it? There are two ways:

Personalities can be explicitly applied using the `-p` / `--personality`
flag. This is intended to be used mostly for testing purposes:

```sh
lx -p stree
```

The intended way to use a personality (once you're happy with it) is to
create a symlink with the same name as the personality in your `$PATH`,
pointing that symlink to the `lx` binary.

> A discussion of symlinks is beyond the scope of this guide. On most
> UNIX variants, you can read the ln(1) and symlink(2) man pages.

One way to do so, given that `~/.local/bin` is in your `$PATH` *and*
that the `lx` binary is in your `$PATH`:

```sh
ln -s $(which lx) ~/.local/bin/stree
```

When this is done, you can run `stree` just like `lx`, or any other
binary. Of course, you can also use any of `lx`'s CLI flags with your
newly created personality:

```sh
stree                 # Run `lx` with the `stree` personality.
stree -l              # … add a long listing to `stree`
```

### Inheritance

Personalities can inherit from each other, forming a family tree. This
is done via the special configuration key `inherits`.

Children replace the
parent's `format`/`columns` and merge everything else, with the
child winning on conflicts.  This is how you build up a family of
related views without repeating yourself:

```toml
[personality.ll]          # long listing
format = "long"

[personality.la]          # long listing, plus hidden files
inherits = "ll"
all = true

[personality.lt]          # time-sorted long listing
inherits = "ll"
sort = "age"
```

Keys explicitly set in a child personality override the parent's
configuration. The value of any configuration key *not* explicitly set
in a child personality is inherited from the parent personality, which
may in turn inherit it from *its* parent, and so on.

When a key isn't set in a personality or any of its parents, the
compiled-in default is used.


XXX TODO: These two examples don't belong in this section:

```toml
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


### The built-in personalities

`lx` ships with a set of compiled-in personalities that are usable
out of the box without any config file.  They form an inheritance
tree rooted at `default`:

```text
default ──┬──→ lx ──┬──→ ll ──→ la
          │         └──→ lll
          └──→ tree

ls  (standalone — no inherits)
```

#### `default`

The base personality, intended to be inherited by (almost) everything
else.  There's nothing magical about the name: you can override it or
ignore it and define your own base under a different name.

The `default` built-in personality would look like this in `lx`'s
configuration format:

```toml
[personality.default]
colour = "auto"               # always / auto / never
gradient = "all"              # all / none / size / date / size,date
time-style = "default"        # default / iso / long-iso / full-iso
group-dirs = "none"           # first / last / none
icons = "never"               # always / auto / never
classify = "never"            # always / auto / never
theme = "exa"                 # named theme (see below)
```

Since the `default` personality is (intended to be) the base of all
other personalities, it's a good place to set global defaults.

> And yet, you're free to define personalities that *don't* inherit from
  `default`, and which therefore don't use the "global" default
  settings!

#### `lx`

The standard personality, used when `lx` is invoked as itself.  Inherits
from `default`, but is itself intended to be used as a base for other
personalities.

The built-in version of the `lx` personality inherits from `default`,
but applies no further customisations:

```toml
[personality.lx]
inherits = "default"
```

#### `ll`

A long listing, equivalent to `lx -l` with VCS status and
`--group-dirs=first`:

```toml
[personality.ll]
inherits = "lx"
format = "long2"
group-dirs = "first"
```

#### `la`

Like `ll`, plus hidden files (`ls -la`):

```toml
[personality.la]
inherits = "ll"
all = true
```

#### `lll`

The expanded long listing with header, all timestamps, hard link count,
and block count:

```toml
[personality.lll]
inherits = "lx"
format = "long3"
group-dirs = "first"
header = true
time-style = "long-iso"
```

#### `tree`

Recursive tree view with directories first:

```toml
[personality.tree]
inherits = "default"
tree = true
format = "long2"
group-dirs = "first"
```

#### `ls`

Tries to look more like POSIX `ls`:

```toml
[personality.ls]
grid = true
across = true
```

> Note: You can redefine the built-in personalities in your own config.
  Your definition takes precedence over the compiled-in version.


### Creating personalities from the command line

Sometimes you find a combination of flags you like — either for a particular project, or in general — and you realise it would be nice to have a personality that combines exactly those flags.

In a case like this, the `--save-as` flag can be used to save the set of flags to a named personality

```sh
lx -l --total --sort=size --reverse --save-as=du
```

This writes `du.toml` to `lx`'s `conf.d` drop-in configuration directory, containing the configuration keys corresponding to the flags you typed,
inheriting everything else from the active personality.

If you had invoked `lx` as (say) `ll` when using `--save-as`, the saved personality inherits from `ll`. If you had not invoked any specific personality while saving, the saved personality inherits from the `lx` personality.

To preview the same TOML snippet without writing a file, use `--show-as=NAME` instead. It's useful for sanity-checking what `--save-as` would produce, or for piping into a config file manually:

```sh
lx -l --total --sort=size --reverse --show-as=du
```

### `LX_PERSONALITY`

You can set a session-level default via the `LX_PERSONALITY`
environment variable:

```sh
export LX_PERSONALITY=ll    # every lx invocation in this shell uses ll
```

> The `LX_PERSONALITY` variable is very useful if you use a tool like
> `direnv` to set per-directory defaults!

The full personality resolution order is:

`-p` flag → `argv[0]` (symlink name) → `$LX_PERSONALITY` →
compiled-in default (`lx`).

When `lx` is invoked
as `lx`, the `argv[0]` step is skipped so
that the environment variable can take effect — `$LX_PERSONALITY` is
conceptually "the personality for bare `lx`".

Symlinks like `ll` or
`tree` still win over the environment variable because they're
structural: when you type `ll`, you always mean "long view".


### Formats

A format is a named, ordered list of columns. If you find yourself using the `--columns` flag often with a specific set of columns (in a specific order), you might want to define a format for it.

There is just a single `[format]` section in the config file; all the named formats are defined there:

```toml
[format]
compact = ["permissions", "size", "modified"]
hpc     = ["permissions", "size", "user", "group", "modified", "vcs"]
```

Once a format is defined, you can use it interactively with the `--format` flag:

```sh
lx --format=hpc
```

Like `--columns`, `--format` has no short flag, since it's anticipated that formats will mostly not be used interactively, but rather as part of personalities:

```toml
[personality.compact]
format = "compact"
```

If a named format is used in multiple personalities, all of those personalities can be updated by simply changing the definition of the format.

> You can find the full vocabulary of column names for `--format` (and `--columns`) in the lxconfig.toml(5) man page.

### The built-in formats

Three formats are built-in and thus available even without a configuration file: `long`, `long2` and `long3`. Their definitions are as follows:


```toml
[format]
long    = ["permissions", "size", "user", "modified"]
long2   = ["permissions", "size", "user", "group", "modified", "vcs"]
long3   = ["permissions", "links", "size", "blocks",
           "user", "group", "modified", "changed", "created", "accessed", "vcs"]
```

These three long formats are actually a bit special: They are used when you use `lx`'s compounding `-l` / `--long` flags:

```sh
lx -l               # equivalent to `lx --format=long`
lx -ll              # equivalent to `lx --format=long2`
lx -lll             # equivalent to `lx --format=long3`
```

You can redefine the three built-in formats in your own config. Changing the definition of `long`, `long2` or `long3` is perhaps the quickest way to get started with customising `lx` — a small change that makes a big difference!

### Conditional overrides

Personalities can include `[[personality.NAME.when]]` blocks that
activate based on environment variables or the host operating
system.  This allows you to adapt a personality to different
terminals, SSH sessions, platforms, and so on… all without
shell-level scripting:

A `[[when]]` block must contain at least one `env` or `platform`
condition.  Some examples:

```toml
[personality.ll]                # A basic long listing
inherits = "lx"
format = "long2"
header = true

[[personality.ll.when]]
env.TERM_PROGRAM = "ghostty"    # When using `ll` on Ghostty, enable icons
icons = "always"

[[personality.ll.when]]
env.SSH_CONNECTION = true       # Disable icons, colour when SSHing
colour = "never"
icons = "never"

[[personality.ll.when]]
platform = "macos"              # On macOS, prefer the system's `ls`-style
group-dirs = "none"             #   group-dirs behaviour (none) over `first`
```

Environment conditions take three forms:
- `env.VAR = "value"` — matches when `$VAR` equals the given
  string exactly;
- `env.VAR = true` — matches when `$VAR` is set (to anything);
- `env.VAR = false` — matches when `$VAR` is unset.

The `platform` condition matches against Rust's `std::env::consts::OS`
(`"macos"`, `"linux"`, `"freebsd"`, etc.):
- `platform = "macos"` — match on macOS only;
- `platform = ["linux", "freebsd"]` — match on any of the listed
  platforms.

You can mix `env` and `platform` keys, and set multiple `env`
keys in a single `[[when]]` block.  All conditions in a block
must match (implicit `AND`).

Multiple `[[when]]` blocks for the same personality stack, with later matches overriding earlier ones.

### Numeric formatting

There are some configuration keys that are not available as CLI flags. At present, just two, and they both pertain to the formatting of numeric values.

By default, `lx` uses your system locale for decimal points and
thousands grouping.  The two special config keys let you override
this:

```toml
[personality.default]
decimal-point = ","
thousands-separator = " "
```

Set `thousands-separator` to an empty string to disable digit grouping entirely.

These keys apply to **counts** — file sizes (in all `--size-style` modes),
`--total` size totals, `-CZ` summaries, block counts, and link
counts. They do **not** apply to identifiers that happen to be numeric (inodes, UID, GID) — these are always formatted as-is.

> These are good examples of keys you might want to set in   `[personality.default]` to make them global!


## The configuration file

We've already seen how to format configuration snippets in TOML — both personalities and formats. It's time to take a closer look at `lx`'s configuration file.

`lx` reads at most one main configuration file plus an optional
directory of drop-in fragments.

> **The configuration file is optional.**  `lx` is designed to
> work just fine without one — the config file only exists
> so you can customise things you want to change.

The configuration file is a [TOML](https://toml.io/) file. This gives it a simple, mostly self-documenting format.

### Getting started

You can generate a starter configuration with `--init-config`:

```sh
lx --init-config
```

This writes an example configuration file to `~/.lxconfig.toml`.  The file is self-documenting:
- prose comments (starting with `##`) explain each section, and
- commented-out values (starting with `#`) show the compiled-in
  defaults that you can uncomment and edit.

> The example config generated by `--init-config`
> documents the defaults but doesn't change
> them. `lx` will work exactly the same with this unedited configuration template as it does with no configuration file at all. You have to edit the file to change `lx`'s behaviour.

### The configuration version

During the course of `lx`'s development, the configuration file format has evolved. A special `version` key was therefore introduced to enable `lx` to judge whether the configuration file format is up-to-date or not.

The configuration file version is specified by the bare `version` key which is usually found near the top of the file. The current version is `0.6`:

```toml
version = "0.6" 
```

If you need to migrate from an older version configuration, use the `--upgrade-config` command:

```sh
lx --upgrade-config 
```

This converts from any previous format
to the current 0.6 schema and saves a `.bak` of the original.

The 0.5 → 0.6 migration is mostly cosmetic (version string only)
but also injects auto-selection `[[when]]` blocks into your
`[personality.default]` section so capable terminals get the
new 256-colour and 24-bit theme tiers automatically.

### Sections

The configuration file hosts any personalities and formats you may have defined, as you have already seen.

But that's not all. An `lx` configuration file usually consists of:

1. The `version` key
2. A single `[format]` section
3. Multiple `[personality.NAME]` sections
4. Multiple `[theme.NAME]` sections
5. Multiple `[style.NAME]` sections
6. A single `[class]` section

The order of the sections is not important, nor do you have to keep all the sections of a certain type (say, personalities) together.

Let's take a quick look at each of the sections:

#### Personalities: `[personality.NAME]`

This introduces a named personality. As you've already learned, personalities are bundles of settings, activated by name.

Each personality has its own section:

```toml
[personality.lt]
inherits = "lx"
format = "long2"
sort = "age"
```

#### Formats: `[format]`

A format is (as we've seen) a named column layout for long view. Formats are used in personalities.

There is just one `[format]` header; all the named formats form a flat list under it:

```toml
[format]
long = ["permissions", "size", "user", "modified"]
compact = ["permissions", "size", "modified"]
```

#### Themes: `[theme.NAME]`

A theme, as we'll see in the next section, is a named set of colours which can be applied to `lx`'s UI elements

Each named theme is in its own section of the config:

```toml
[theme.midnight]
directory = "bold blue"
date = "steelblue"
```

#### Styles: `[style.NAME]`

Styles are colours applied to the names of the files being listed, and are usually used in themes.

Each named style has its own section:

```toml
[style.midnight]
"*.rs" = "#ff8700"
class.source = "yellow"
```

#### File classes: `[class]`

File classes provide a way to give a name to a set of glob patterns, and are used in styles.

There is only one `[class]` header. All the named file classes form a flat list under it:

```toml
[class]
media = ["*.jpg", "*.png", "*.mp4"]
source = ["*.rs", "*.py", "*.c"]
```

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

See `man lxconfig.toml` for the full reference.


## Themes, Styles, and Classes


For anything beyond a basic colour scheme — theming individual
columns, per-tier size and date gradients, palette inheritance,
smooth colour interpolation — use a `[theme.NAME]` section in your
`~/.lxconfig.toml`.  The config file is `lx`'s full-power theming
surface.

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



### Themes

A theme is a named set of colours which can be applied to change how `lx`'s columns and other UI elements look. Themes are the top-level theming elements in the configuration file, but they work alongside styles and file classes.

Each theme is in its own section. A theme named `midnight` is introduced by 
the section header `[theme.midnight]`.

Themes can inherit from other themes via the `inherits` key.
Without `inherits`, a theme starts from a blank slate — useful
when you want full control.

```toml
[theme.midnight]
description  = "Cornflower blues on a midnight palette"
inherits     = "lx-256"
directory    = "bold cornflowerblue"
executable   = "bold khaki"
symlink      = "lightsteelblue"
date         = "steelblue"
date-now     = "bold lightsteelblue"
date-old     = "slategray"
size-major   = "cornflowerblue"
size-minor   = "slategray"
vcs-new      = "bold khaki"
vcs-modified = "salmon"
punctuation  = "midnightblue"
```

The optional `description` is a one-line summary that `--show-config`
surfaces in its catalogue and that `--dump-theme` emits.  Skip it
if you don't care.

### Using a theme

A theme can be interactively applied to the output of `lx` by using the `--theme` flag:

```sh
lx --theme=midnight
```

However, themes are primarily intended to be used in personalities, via the `theme` key. For instance, to apply our `midnight` theme to the default `lx` personality (and all personalities that inherit from it):

```toml
[personality.lx]
theme = "midnight" 
```

### The built-in themes

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

### Colour values

Colour values in themes (and indeed in styles, as we'll see next) can be specified in a variety of ways:

- Named ANSI colours: `"bold blue"`, `"red"`
- X11 / CSS colour names: `"tomato"`, `"cornflowerblue"`, `"dodgerblue"`
- Hex: `"#ff8700"`, `"#2b2b2b"`
- 256-colour ANSI: `"38;5;208"`
- Modifiers: `bold`, `dimmed`, `italic`, `underline` (combinable)

### Styles

Whereas themes are applied to `lx`'s UI elements, styles define the colours that are applied to the **files** being listed.

A style can assign a colour to one of three things:
* an exact filename, e.g. `Makefile`;
* a glob pattern, e.g. `*.txt`;
* a named filename *class*, which is something we'll detail in an upcoming section. 

Each style has its own section in the configuration file. A style named `midnight` is introduced by the section header `[style.midnight]`:

```toml
[style.midnight]
# Class references: style a whole category at once.
class.source     = "cornflowerblue"       # *.rs, *.py, *.c, …
class.document   = "lightsteelblue"
class.media      = "mediumpurple"
class.compressed = "#7b88a8"              # muted steel
class.temp       = "dimgray italic"
class.crypto     = "bold salmon"          # keys, certs — stand out

# Glob patterns: narrow overrides that win over class colours.
"*.rs"      = "bold cornflowerblue"       # Rust files stand out
"*.toml"    = "lightsteelblue"
"*.lock"    = "slategray italic"
"*.md"      = "lightsteelblue"
"README*"   = "bold lightsteelblue"

# Exact filenames: the most specific match of all.
"Makefile"      = "bold underline khaki"
"Dockerfile"    = "bold steelblue"
".env"          = "bold salmon"
".gitignore"    = "slategray"
```

There is no command-line flag to apply a style — styles are solely intended to be used in themes, by using the `use-style` key:

```toml
[theme.midnight]
use-style = "midnight"
```


### File classes

File classes provide a way to give a name to a set of explicit filenames and/or glob patterns. Classes are then used in styles.

There is only one `[class]` header. All the named formats form a flat list under it.

```toml
[class]
source = ["*.rs", "*.py", "*.js", "*.go", "*.c", "*.cpp"]
data   = ["*.csv", "*.json", "*.xml", "*.yaml", "*.h5"]
build  = ["Makefile", "Justfile", "*.mk", "CMakeLists.txt"]
media  = ["*.jpg", "*.png", "*.mp4", "*.mkv"]
```

Once a named class is defined in the `[class]` section, it can be used in styles by using a `class.NAME` key (as we've already seen):

```toml
[style.midnight]
class.source     = "cornflowerblue"       # *.rs, *.py, *.c, …
class.document   = "lightsteelblue"
class.media      = "mediumpurple"
class.compressed = "#7b88a8"              # muted steel
class.temp       = "dimgray italic"
class.crypto     = "bold salmon"          # keys, certs — stand out
```

### The built-in file classes

`lx` ships with built-in classes for `image`, `video`, `music`,
`lossless`, `crypto`, `document`, `compressed`, `compiled`, `temp`,
and "`immediate`" (build/project files).

Redefining a class name in your config overrides the
compiled-in version.

> For the full definitions of the built-in classes, see the lxconfig.toml(5) man page.

### Starting from a clean slate

If you want no compiled-in
class colouring at all (for example, because you've got an
exhaustive `$LS_COLORS` list and want to drive all file-type
colouring from there), point your personality at a style that
references only classes you define yourself — or define a style
with no class or extension entries and use it via `use-style`:

```toml
[style.empty]
# deliberately empty — no class.* or extension keys

[theme.mine]
use-style = "empty"

[personality.default]
theme = "mine"
```

The style layer is the compiled-in file-type colouring; an empty
style switches it off entirely.


### `$LS_COLORS`

`lx` honours `LS_COLORS` for interoperability — if your shell
profile already sets it, lx picks up your file-type and extension
colours automatically.  But `LS_COLORS` is a legacy format: its
vocabulary is limited to a handful of file-kind keys plus glob
patterns, with raw ANSI SGR values, no inheritance, no per-column
overrides, no gradients.

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
directory. For a list of the available themes, see
[`themes/README.md`](../themes/README.md).

These are **drop-in files** — copy the ones you want
into `~/.config/lx/conf.d/` to make them available, then
activate with `--theme=NAME` or set as a personality default:

```sh
mkdir -p ~/.config/lx/conf.d
cp themes/dracula.toml ~/.config/lx/conf.d/
lx -l --theme=dracula
```


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


## Shell completions

Completions are available for bash, zsh, and fish.

#### Current session only

```sh
# bash
source <(lx --completions bash)

# zsh
source <(lx --completions zsh)

# fish
lx --completions fish | source
```

#### Permanent installation

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

#### Personality symlinks

For bash, zsh, and fish, the generated completions also cover any
personality symlinks found in `$PATH` that point to the `lx` binary.
If you've symlinked `ll`, `la`, and `tree` to `lx`, tab completion
works for those names too — no extra setup needed.

Regenerate completions after creating or removing personality
symlinks.

Elvish and PowerShell completions cover the `lx` command


## Debugging your configuration

Two families of flags help you inspect what `lx` is doing.

### `--show-config`

Prints a human-friendly, coloured overview of the configuration.
Output divides into two halves:

- **Active** (default) — what's currently running: where the
  config came from, the resolved personality with inheritance
  chain, the active format, theme, and style.
- **Available** — a catalogue of every defined personality,
  format, theme, style, and class, each with its source and a
  one-line summary or `description`.

`--show-config[=MODE]` controls which halves to emit:

| Form                       | Shows               |
|----------------------------|---------------------|
| `--show-config`            | Active half (default) |
| `--show-config=full`       | Both, separated by a horizontal rule |
| `--show-config=available`  | Catalogue only      |

The personality section in the active half is the most detailed:

```
Personality: la
  activated by: -p
  inheritance:
    la (builtin)
    ll (builtin)
    lx (builtin)                    1 [[when]] block, 0 active
    default (builtin)               2 [[when]] blocks, 2 active
  settings:
    all = true
    classify = "never"
    ...

Format: long2
  source: personality
  columns: perms, size, user, group, modified, vcs
```

The **inheritance** list shows the full chain from the requested
personality down to its root ancestor.  Each entry shows:

- **source**: `builtin`, `config`, or `config, overrides builtin`
  if a config-defined personality shadows a compiled-in one.
- **`[[when]]` block status**: how many conditional override blocks
  are defined at that level and how many are currently active.
  This is the first thing to check if theme auto-selection isn't
  working as expected — if `0 active` appears where you expect a
  match, the environment variable condition isn't being met.

The **Format** section shows the active format name, its source
(`personality` or `implicit, selected by -lll`), and the columns
it resolves to.

### `--dump-*`

Prints copy-pasteable TOML definitions for any config object.
Each takes an optional `=NAME` to restrict output to a single
object:

```sh
lx --dump-class                 # all class definitions
lx --dump-class=temp            # just the temp class
lx --dump-format                # all formats
lx --dump-format=long2          # just long2
lx --dump-personality=ll        # the ll personality (with [[when]] blocks)
lx --dump-style=exa             # the exa style
lx --dump-theme=dracula         # the dracula theme (if loaded)
lx --dump-theme=lx-24bit        # the compiled-in 24-bit theme
```

Personality dumps include `[[when]]` blocks and list personalities
in inheritance order (parents before children), so the output is
valid, self-contained TOML that can be pasted directly into a
config file.

Theme dumps work for both user-defined themes and the three
compiled-in themes (`exa`, `lx-256`, `lx-24bit`).  Output is
grouped by key family — file kinds, permissions, size, users,
links, VCS, then the four per-timestamp date blocks, then
columns and symlink overlays — with blank lines between families.
This is the same shape `--init-config` writes, so a copy-paste
into `~/.config/lx/lxconfig.toml` produces an immediately usable
theme block.

### Debug logging

For deeper diagnostics — config-file discovery, theme resolution,
personality cascade — set `LX_DEBUG=1` in the environment and
`lx` will emit trace logging to stderr.


## Environment variables

| Variable          | Purpose                                                    |
|-------------------|------------------------------------------------------------|
| `LX_CONFIG`       | Explicit config file path                                  |
| `LX_DEBUG`        | Enable debug logging (`1` or `trace`)                      |
| `LX_GRID_ROWS`    | Minimum rows for grid-details view (also a config key)     |
| `LX_ICON_SPACING` | Spaces between icon and filename (also a config key)       |
| `LX_PERSONALITY`  | Session-level personality selection (see §Personalities)   |
| `LS_COLORS`       | Standard file-type colour scheme                           |
| `COLUMNS`         | Override terminal width                                    |
| `TIME_STYLE`      | Default timestamp style                                    |
| `NO_COLOR`        | Disable colours (see [no-color.org](https://no-color.org)) |


## Appendix: short flag reference

Quick-reference table of all short flag allocations.

**Display / layout**

| Flag | Long form    | Purpose                                      |
|------|--------------|----------------------------------------------|
| `-1` | `--oneline`  | One entry per line                           |
| `-l` | `--long`     | Long view (compounds: `-ll`, `-lll`)         |
| `-G` | `--grid`     | Grid view (default)                          |
| `-x` | `--across`   | Sort grid across                             |
| `-T` | `--tree`     | Tree view                                    |
| `-R` | `--recurse`  | Recurse into directories                     |
| `-L` | `--level`    | Depth limit for `-T` / `-R`                  |
| `-C` | `--count`    | Item count to stderr (`-CZ` adds total size) |
| `-w` | `--width`    | Terminal width override                      |
| `-A` | `--absolute` | Show absolute paths                          |

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

| Flag | Long form                  | Purpose                         |
|------|----------------------------|---------------------------------|
| `-i` | `--inode`                  | Inode number                    |
| `-o` | `--octal`                  | Octal permissions               |
| `-M` | `--permissions` / `--mode` | Symbolic permission bits        |
| `-O` | `--flags`                  | Platform file flags             |
| `-H` | `--links`                  | Hard link count                 |
| `-z` | `--size`                   | File size                       |
| `-Z` | `--total`                  | Recursive directory totals      |
| `-S` | `--blocks`                 | Allocated block count           |
| `-u` | `--user`                   | Owner name                      |
| `-g` | `--group`                  | Group name                      |
| `-@` | `--extended` (alias `--xattr`) | Extended attributes (`-@` indicator only, `-@@` full listing) |
| `-h` | `--header`                 | Header row                      |
| `-b` | `--bytes`                  | Raw byte counts                 |
| `-K` | `--decimal`                | Decimal size prefixes (k, M, G) |
| `-B` | `--binary`                 | Binary size prefixes (KiB)      |

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


## Further reading

- **`man lx`** — long-form command reference.  (mdoc source at
  [`man/lx.1`](../man/lx.1).)
- **`man lxconfig.toml`** — complete reference for the configuration
  file format, including every theme key, style syntax, and the full
  built-in class list.  (mdoc source at
  [`man/lxconfig.toml.5`](../man/lxconfig.toml.5).)
- **[`CHANGELOG.md`](../CHANGELOG.md)** — release notes.
- **[`themes/README.md`](../themes/README.md)** — how to write
  your own theme.
