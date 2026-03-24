# lx — file Lister eXtended

**`lx`** ("alex") is a modern file lister for Unix. A replacement for the standard `ls`.

> `lx` is explicitly not a *drop-in* replacement for POSIX `ls`.

`lx` is forked from [`exa`](https://github.com/ogham/exa) by Benjamin Sago. `exa` appears to be unmaintained (and has been for some years).

An active community fork of `exa` named
[`eza`](https://github.com/eza-community/eza) exists, but `lx` is an experiment
with a somewhat different approach to the command-line user interface.


## Highlights

- **Personalities** — named profiles that bundle columns, flags, and
  settings

  Create symlinks (`ll`, `la`, `lll`, `tree`) and `lx` adapts its behaviour to
  the name it's invoked as!

- **Fully configurable column layout**

  `--columns` gives you complete control over which columns are displayed *and*
  their order: `--columns=perms,size,user,modified`.  
  `--format` allows you to apply named sets of columns: `--format=long2`.

- **"Compounding" flags** — flags that compound their effect when repeated

  Use `-l` for a long listing, `-ll` for more detail, and `-lll` for even more.  
  No more remembering which combination of `-g`, `-H`, `-h`, and `--git` you 
  need!

- **Configuration file**

  One `lxconfig.toml` replaces all your shell aliases and environment
  variables. Define formats, personalities, colour themes, styles,
  and file-type classes — all with inheritance. Run
  `lx --init-config` to get started.

- **Named colour themes, styles, and file-type classes**

  Define themes (UI elements), styles (file colours), and classes
  (file-type categories) in your config using human-readable colour
  names (`"bold dodgerblue"`, `"tomato"`, `"#ff8700"`).  Everything
  inherits, composes, and can be overridden. Select via personality
  settings or `--theme=NAME`.

- **Unified VCS support, including Jujutsu!**

  `--vcs=auto|git|jj|none` with built-in backends for both 
  [Git](https://git-scm.com) and [Jujutsu](https://jj-vcs.dev/latest/). 
  The VCS in use can be auto-detected.


## Installation

`lx` is built from source using [Cargo](https://doc.rust-lang.org/cargo/),
Rust's package manager. `lx` requires Rust 1.94 or later.

Install Rust if you don't have it already:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install `lx` from GitHub
```sh
cargo install --git https://github.com/wjv/lx
```

The binary is installed to `~/.cargo/bin/`.  Make sure this directory is on 
your `$PATH`.

Alternatively, build `lx` from a local clone:
```sh
git clone https://github.com/wjv/lx.git
cd lx
cargo build --release
```

The binary is at `target/release/lx`; you can optionally copy it somewhere on 
your `$PATH`.

> **Note:** The `lx` crate name on `crates.io` is taken by an unrelated
> library. `cargo install lx` will *not* install this tool.


## Quick start

Just start using it. Do keep in mind that the flags differ from those of `ls`. When you get stuck, the `--help` flag is your friend.

```sh
lx                    # grid view (like ls)
lx -l                 # long view: perms, size, user, modified
lx -ll                # + group, VCS status
lx -lll               # + header, all timestamps, links, blocks
lx --help             # display online help
lx -T                 # tree view
lx -T -L2             # tree, depth 2
```

## A file lister with personality

Since time immemorial (or at least since Unix shells supported command aliasing), Unix users have used shell aliases to call `ls` with certain options. For example, it's very common to alias `ll` to list files in the long format:

```sh
alias ll="ls -l"
```

Or `la` for a long listing that also shows hidden files:

```sh
alias la="ls -la"
```

`lx` *embraces* the idea of being called by different names. It calls this "personalities". The idea is pretty simple:

1. Define a *personality* in `lx`'s configuration — `ll`, for instance:

    ```toml
    [personality.ll]
    format = "long"
    ```

2. Invoke `lx` as `ll` by creating a [symlink](https://en.wikipedia.org/wiki/Symbolic_link#Unix-like) named `ll` that points to `lx`, for example:

    ```sh
    ln -s $(which lx) ~/.local/bin/ll
    ```

Whenever you invoke `lx` under a different name *and* a personality with that name is defined, `lx` behaves like that personality. (If no personality with the given name exists, `lx` adopts its default personality named `lx`.)

Personalities can inherit from each other. For instance, you can create an `la` personality that behaves just like `ll`, but shows hidden files as well:

```toml
[personality.la]
inherits = "ll"
all = true
```

`lx` ships with some default, built-in personalities. You can redefine these in the configuration file.

* `default` is base personality used to define default settings, inherited by other personalities. Note that there's nothing magical about the name `default`; you can define your own base personality with a different name.
* `lx` is `lx`'s standard personality, and is used when no other personality has been invoked.
* `ll` is a long file listing, like `lx -l`; it shows VCS information and groups directories first
* `la` is like `ll` but also shows hidden files (like `ls -la`)
* `lll` is an even more expansive file listing with headers, like `lx -lll`
* `tree` is a recursive file listing showing a graphical file tree, with directories first
* `ls` makes `lx`'s output look more like that of standard POSIX `ls`

You can test personalities with the `-p`/`--personality` flag. To see what `ll` or `tree` will look like:

```sh
lx -p ll
lx --personality tree
```

Of course, you can still use shell aliases with `lx`. Personalities are an alternative that offer you a more structured way of doing the same thing.

For more on how personalities are configured — including the inheritance tree 
and all available settings — see [Personalities](#personalities) in the 
Configuration section below.

## Configuration

Personalities — and other aspects of `lx`'s behaviour — can be
defined in a configuration file.

> **Of course** the configuration file is **optional**. `lx` is
> just a file lister, after all, and it's designed to work just
> fine with its compiled-in defaults. The configuration file is a
> tool for the user who wants more flexibility!

Generate a starter config with:

```sh
lx --init-config
```

This creates `~/.lxconfig.toml` with commented examples.  The
file is self-documenting — prose comments (starting with `##`)
explain each section, while commented-out values (starting with
`#`) show the compiled-in defaults you can customise.

The configuration has five kinds of named section, each controlling
a different aspect of `lx`'s behaviour:

| Section              | Purpose                                             | Example                                               |
|----------------------|-----------------------------------------------------|-------------------------------------------------------|
| `[format]`           | Column layouts for long view (flat keys)            | `long = ["perms", "size", "user", "modified"]`        |
| `[personality.NAME]` | Bundles of settings, activated by name              | `inherits = "lx"`, `format = "long2"`, `sort = "age"` |
| `[theme.NAME]`       | UI element colours (directories, dates, etc.)       | `directory = "bold blue"`, `date = "steelblue"`       |
| `[style.NAME]`       | File-type colours (by class, glob, or filename)     | `class.source = "yellow"`, `"*.rs" = "#ff8700"`       |
| `[class]`            | Named file-type categories (lists of glob patterns) | `media = ["*.jpg", "*.png", "*.mp4"]`                 |

These sections compose naturally.  Personalities and themes support
inheritance; styles and classes are simple flat definitions.

```
personality ──→ format  (column layout)
     │
     └──→ theme ──────→ style ────────→ class
          (UI colours)  (file colours)  (pattern lists)
```

For the full reference, see the
[`lxconfig.toml(5)`](man/lxconfig.toml.5.md) man page.


### Formats

Formats are defined as keys in a flat `[format]` section. Each key
is a format name; its value is a list of column names:

```toml
[format]
compact = ["perms", "size", "modified"]
hpc     = ["perms", "size", "user", "group", "modified", "vcs"]
```

Available column names:

| Column     | Shows                   |
|------------|-------------------------|
| `perms`    | Permission bits         |
| `size`     | File size               |
| `user`     | Owner                   |
| `group`    | Group                   |
| `links`    | Hard link count         |
| `inode`    | Inode number            |
| `blocks`   | Block count             |
| `octal`    | Octal permissions       |
| `modified` | Last modified time      |
| `changed`  | Last status change time |
| `accessed` | Last access time        |
| `created`  | Creation time           |
| `vcs`      | VCS status              |

The built-in formats `long`, `long2`, and `long3` are used by the
flags `-l`, `-ll`, and `-lll`.

```toml
[format]
long  = ["perms", "size", "user", "modified"]
long2 = ["perms", "size", "user", "group", "modified", "vcs"]
long3 = ["perms", "links", "size", "blocks", "user", "group",
         "modified", "changed", "created", "accessed", "vcs"]
```

You can override the built-in defaults by simply redefining them.

You can explicitly use a format with `--format=NAME`, but more often
you will want to use formats in personalities:

### Personalities

As described [above](#a-file-lister-with-personality), a personality
bundles format, columns, and settings under a name. Every CLI flag
has a corresponding config key (e.g. `--sort=age` becomes
`sort = "age"`).

The built-in personalities form an inheritance tree:

```
default ──┬──→ lx ──┬──→ ll ──→ la
          │         └──→ lll
          └──→ tree

ls  (standalone — no inherits)
```

The child's `format`/`columns` replace the parent's; settings merge
with the child winning. Define your own and wire them into the tree
however you like:

```toml
[personality.la]              # all files, including hidden
inherits = "ll"
all = true

[personality.lt]              # time-sorted long listing
inherits = "ll"
sort = "age"

[personality.recent]          # recently modified files
format = "long"
sort = "modified"
reverse = true

[personality.du]              # du replacement: dir sizes
columns = ["size"]
only-dirs = true
tree = true
level = 2
sort = "size"
reverse = true
total-size = true
```

> **Upgrading from 0.1 or 0.2:** run `lx --upgrade-config` to migrate
> your config file to the current 0.3 format. Both 0.1→0.3 and 0.2→0.3
> migrations are supported. A `.bak` backup of the original is saved.


### Config file locations

`lx` searches for its config file in this order (first found wins):

1. **`$LX_CONFIG`** — set this environment variable to point to a
   config file at any path. Useful for testing or per-project configs.
2. **`~/.lxconfig.toml`** — the simplest option; just drop a file in
   your home directory. This is where `lx --init-config` writes by
   default.
3. **`$XDG_CONFIG_HOME/lx/config.toml`** — follows the
   [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/latest/)
   specification. Defaults to `~/.config/lx/config.toml` if
   `$XDG_CONFIG_HOME` is not set. Preferred on Linux; also used on macOS.
4. **`~/Library/Application Support/lx/config.toml`** — the standard
   macOS application configuration location. Only checked on macOS,
   after the XDG location.

Most users will be fine with option 2 (`~/.lxconfig.toml`). If you
prefer to keep your dotfiles tidy under `~/.config/`, use option 3.


## VCS support

`lx` shows per-file version control status in long view:

```sh
lx -ll                # tier 2 includes VCS status by default
lx --vcs-status -l    # or explicitly
lx --vcs=jj -ll       # force jj backend
lx --vcs=git -ll      # force git backend
lx --vcs=none -ll     # disable VCS
```

With `--vcs=auto` (the default), lx probes for a jj workspace first,
then falls back to git. (This is so that co-located jj/git repositories
are detected correctly.)

Status characters: `-` not modified, `M` modified, `N` new, `D` deleted,
`R` renamed, `C` copied, `I` ignored, `U` conflicted.

Git shows two columns (staged + unstaged). jj shows one (since there is no staging area).


## Column visibility

Every column has both a positive and negative flag:

```sh
lx -ll --no-group              # remove group from tier 2
lx -l --inode                  # add inode to tier 1
lx -l --permissions --no-user  # explicit control
```

Or take full control with `--columns`:

```sh
lx --columns=inode,perms,size,user,group,modified,vcs
```

Or define your own named format in the config file.


## Themes, styles, and classes

Colour customisation uses three kinds of config section that work
together:

- **Themes** (`[theme.NAME]`) set colours for UI elements:
  directories, permissions, dates, VCS status, etc.
- **Styles** (`[style.NAME]`) set colours for files — either by
  referencing a named class or by matching a glob pattern / filename.
- **Classes** (`[class]`) define named file-type categories as lists
  of glob patterns: `media`, `source`, `archive`, etc.

The built-in `"exa"` theme and `"exa"` style provide sensible
defaults out of the box.  To customise, define your own theme
and/or style:

```toml
[theme.ocean]
inherits = "exa"                  # start from the compiled-in defaults
directory = "bold dodgerblue"
date = "steelblue"
vcs-new = "bold mediumspringgreen"
use-style = "dev"                 # reference a named style set

[style.dev]
class.source = "#ff8700"          # class reference (bare dotted key)
"*.toml" = "sandybrown"           # glob pattern (quoted key)
"Makefile" = "bold underline yellow"  # exact filename (quoted key)
```

Classes let you group file types by category:

```toml
[class]
source = ["*.rs", "*.py", "*.js", "*.go"]
data   = ["*.csv", "*.json", "*.xml"]
```

`lx` ships with built-in classes for `image`, `video`, `music`,
`lossless`, `crypto`, `document`, `compressed`, `compiled`, `temp`,
and `immediate` (build/project files).  Override any of them by
redefining the name in your `[class]` section.

Select a theme through a personality or from the command line:

```toml
[personality.default]
theme = "ocean"                   # all personalities inherit this
```

```sh
lx --theme=warm                   # override from the command line
```

Colour values accept named ANSI colours (`"bold blue"`), X11/CSS
names (`"tomato"`, `"cornflowerblue"`), hex (`"#ff8700"`),
256-colour (`"38;5;208"`), and modifiers (`bold`, `dimmed`,
`italic`, `underline`).

Themes can inherit from other themes. The special name `"exa"`
refers to the compiled-in default theme and style.  Without
`inherits`, a theme starts from a blank slate.

See [`lxconfig.toml(5)`](man/lxconfig.toml.5.md) for the full
list of theme keys, style syntax, and built-in class definitions.


## Sorting

```sh
lx -s name            # sort by name (default, case-insensitive)
lx -s Name            # case-sensitive (uppercase first)
lx -s size            # smallest first
lx -rs size           # largest first (-r reverses)
lx -s modified        # oldest first
lx -s age             # newest first (alias for reverse-modified)
lx -s ext             # by extension
lx -s none            # unsorted (readdir order)
```

With `--total-size`, `-s size` sorts by recursive directory size.


## Environment variables

| Variable          | Purpose                                                    |
|-------------------|------------------------------------------------------------|
| `LX_CONFIG`       | Explicit config file path                                  |
| `LX_COLORS`       | Colour theme (overrides `LS_COLORS`)                       |
| `LX_DEBUG`        | Enable debug logging (`1` or `trace`)                      |
| `LX_GRID_ROWS`    | Minimum rows for grid-details view                         |
| `LX_ICON_SPACING` | Spaces between icon and filename                           |
| `LS_COLORS`       | Standard file-type colour scheme                           |
| `COLUMNS`         | Override terminal width                                    |
| `TIME_STYLE`      | Default timestamp style                                    |
| `NO_COLOR`        | Disable colours (see [no-color.org](https://no-color.org)) |


## Shell completions

Shell completions are available for bash, zsh, and fish.

### Quick activation (current session only)

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

# zsh (make sure ~/.zfunc is in your $fpath)
lx --completions zsh > ~/.zfunc/_lx

# fish
lx --completions fish > ~/.config/fish/completions/lx.fish
```

Alternatively, add the `source <(lx --completions ...)` line to your
shell's rc file (`.bashrc`, `.zshrc`) to generate completions on the
fly at shell startup.


## Known limitations

- **`--vcs-ignore` does not work with the jj backend**

  The `jj` CLI currently has no way to report which files are gitignored. 
  Workarounds:
  - Use `--vcs=git --vcs-ignore` in a colocated repository.
  - Use `-I` glob patterns to exclude specific files (e.g. `-I target`).

- **0.1 and 0.2 config files need migrating** — the 0.3 config format
  is not backwards-compatible. Run `lx --upgrade-config` to convert
  automatically (a `.bak` backup is saved).

- **The `lx` crate name on crates.io is taken** by an unrelated
  library. Install from GitHub instead (see [Installation](#installation)).


## What's new in 0.3

- **File-type classes** (`[class]`) — named lists of glob patterns
  (`image`, `video`, `music`, `lossless`, `crypto`, `document`,
  `compressed`, `compiled`, `temp`, `immediate`), with compiled-in
  defaults that can be overridden in the config.
- **Styles reference classes** via bare dotted TOML keys
  (`class.NAME = "colour"`) and file patterns via quoted keys.
- **Compiled-in "exa" style** maps classes to default colours.
- **Flat formats** — the `[format]` section is now flat (keys are
  format names, values are column lists), replacing the previous
  `[format.NAME]` sub-tables with `columns` keys.
- **Explicit exa chain** — default personality → exa theme → exa
  style, with no magic fallback.
- **Config version 0.3** — the upgrade tool handles 0.1→0.3 and
  0.2→0.3 migrations.
- `--git` and `--git-ignore` legacy flags removed (use
  `--vcs-status` and `--vcs-ignore`).
- `--group-directories-first` precedence fixed.

## Roadmap: post-0.3

- `--show-config` to display the active personality, format, theme,
  and their resolved definitions
- `--list-themes`, `--list-personalities`, `--list-formats` for
  discoverability
- `--time-style=relative` ("2 hours ago")
- Symlink display flags (`--symlinks=show|hide|follow`)
- `--vcs-repos` (per-directory repo status)
- Polish and bug fixes from daily driving


## User interface stability

`lx` is under active development and literally anything may still change.


## Licence

MIT — same as the original `exa`. See [LICENCE](LICENCE).
