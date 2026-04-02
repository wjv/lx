# lx — file Lister eXtended

**`lx`** ("alex") is a modern file lister for Unix; that is, a replacement for 
the standard `ls` command.

But… `lx` is a file lister with *personality!* 🌟

<img src="docs/images/hero.svg" alt="lx output showing long view with VCS status, file type colours, and size gradient">


## Highlights

What makes `lx` stand out from the crowd?

- **Personalities** — named profiles that bundle columns, flags, and
  settings 🌟

  Create symlinks (e.g. `ll`, `la`, `du`, `tree`) and `lx` adapts its behaviour 
  to the name it's invoked as! And all this without a shell alias in sight!

- **Fully configurable column layout**

  Complete control over displayed columns *and* their order
  (`--columns=perms,size,user,modified`), as well as the ability to define
  and use named column sets known as "formats".

- **"Compounding" flags** — flags that compound their effect when repeated

  For example: Use `-l` for a long listing, `-ll` for more detail, and `-lll` 
  for *even more*.  No more remembering which combination of `-g`, `-H`, `-h`, 
  and `--git` you need!

- **Configuration file** — optional, obviously

  One `lxconfig.toml` replaces all your shell aliases and environment
  variables. Define formats, personalities, and more! Run `lx --init-config` to 
  get started.

- **Named colour themes, styles, and file-type classes**

  The design-conscious power user can define themes, styles, and file-type 
  classes in the config file using human-readable colour names. Everything 
  inherits, composes, and can be overridden. Themes can be applied explicitly 
  with the `--theme` flag, but they're designed to be assigned to 
  personalities!

- **Unified VCS support** — including Jujutsu!

  Built-in backends for both [Git](https://git-scm.com) and
  [Jujutsu](https://jj-vcs.dev/latest/).  VCS auto-detection supported.

For the design principles behind the CLI, see [docs/DESIGN.md](DESIGN.md).


## Quick start

> Don't have `lx` yet? Jump to [Installation](#installation) to get it,
> then come back here.

The best way to start using `lx` is to… just start using it. Flags differ from
those of `ls`, but if you get stuck, `--help` is your friend!

```sh
lx                    # grid view (like ls)

lx -l                 # [l]ong view: perms, size, user, modified
lx -ll                # + group, VCS status
lx -lll               # + header, all timestamps, links, blocks

lx --help             # display online help

lx -T                 # [T]ree view
lx -T -L2             # + [L]imit to depth 2
lx -lTFZL2            # + [l]ong view, dirs [F]irst, show total si[Z]e of dirs
```

If you feel `lx -lTFZL2` is perhaps a bit much to remember, the time has come 
to define a new [personality](#a-file-lister-with-personality)!

> Note that `lx` is explicitly not a *drop-in* replacement for POSIX `ls`.


## A file lister with personality 🌟

Since time immemorial (or at least since Unix shells supported command 
aliasing), Unix users have used shell aliases to call `ls` with certain 
options. For example, it's very common to alias `ll` to list files in the long 
format:

```sh
alias ll="ls -l"
```

Or `la` for a long listing that also shows hidden files:

```sh
alias la="ls -la"
```

`lx` *embraces* the idea of being called by different names. It calls this 
"personalities". The idea is pretty simple:

1. Define a *personality* in `lx`'s configuration — `ll`, for instance:

    ```toml
    [personality.ll]
    format = "long"
    ```

2. Invoke `lx` as `ll` by creating a
   [symlink](https://en.wikipedia.org/wiki/Symbolic_link#Unix-like) named 
   `ll` that points to `lx`, for example:

    ```sh
    ln -s $(which lx) ~/.local/bin/ll
    ```

Whenever you invoke `lx` under a different name *and* a personality with that 
name is defined, `lx` behaves like that personality. (If no personality with 
the given name exists, `lx` adopts its default personality named `lx`.)

Personalities can inherit from each other. For instance, you can create an `la` 
personality that behaves just like `ll`, but shows hidden files as well:

```toml
[personality.la]
inherits = "ll"
all = true
```

`lx` ships with some default, built-in personalities. You can redefine these in 
the configuration file.

* `default` is the base personality used to define default settings, inherited 
  by other personalities. Note that there's nothing magical about the name 
  `default`; you can define your own base personality with a different name.
* `lx` is `lx`'s standard personality, and is used when no other personality 
  has been invoked.
* `ll` is a long file listing, like `lx -l`; it shows VCS information and 
  groups directories first
* `la` is like `ll` but also shows hidden files (like `ls -la`)
* `lll` is an even more expansive file listing with headers, like `lx -lll`
* `tree` is a recursive file listing showing a graphical file tree, with 
  directories first
* `ls` makes `lx`'s output look more like that of standard POSIX `ls`

You can test personalities with the `-p`/`--personality` flag. To see what `ll` 
or `tree` will look like:

```sh
lx -p ll
lx --personality tree
```

Of course, you can still use shell aliases with `lx`. Personalities are an 
alternative that offer you a more structured way of doing the same thing.

For more on how personalities are configured — including the inheritance tree 
and all available settings — see [Personalities](#personalities) in the 
Configuration section below.


## Configuration

Personalities — and other aspects of `lx`'s behaviour — can be
defined in a configuration file.

> **Of course** the configuration file is **optional**. `lx` is
> just a file lister after all, and it's designed to work just
> fine with its compiled-in defaults. The configuration file is a
> tool for the user who wants more flexibility!

Generate a starter config with:

```sh
lx --init-config
```



This creates `~/.lxconfig.toml`. The file is self-documenting — prose comments 
(starting with `##`) explain each section, while commented-out values (starting 
with `#`) show the compiled-in defaults you can customise.

The configuration has five kinds of named section, each controlling
a different aspect of `lx`'s behaviour:

| Section              | Purpose                                             | Example                                               |
|----------------------|-----------------------------------------------------|-------------------------------------------------------|
| `[format]`           | Column layouts for long view (flat keys)            | `long = ["perms", "size", "user", "modified"]`        |
| `[personality.NAME]` | Bundles of settings, activated by name              | `inherits = "lx"`, `format = "long2"`, `sort = "age"` |
| `[theme.NAME]`       | UI element colours (directories, dates, etc.)       | `directory = "bold blue"`, `date = "steelblue"`       |
| `[style.NAME]`       | File-type colours (by class, glob, or filename)     | `class.source = "yellow"`, `"*.rs" = "#ff8700"`       |
| `[class]`            | Named file-type categories (lists of glob patterns) | `media = ["*.jpg", "*.png", "*.mp4"]`                 |

These sections compose naturally:
* personalities use formats and themes
* themes use styles
* styles use classes

```text
personality ──→ format  (column layout)
     │
     └──→ theme ──────→ style ────────→ class
          (UI colours)  (file colours)  (pattern lists)
```

Personalities and themes support *inheritance*. In other words, a personality 
can be based on another personality, and a theme can be based on another theme. 
Styles and classes, by contrast, are simple flat definitions.

For the full reference, see the
[`lxconfig.toml(5)`](man/lxconfig.toml.5.md) man page.


### Formats

Formats are defined as keys in a `[format]` section. Each key
is a format name and its value is a list of column names:

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

The built-in formats `long`, `long2`, and `long3` are used by the flags
`-l`, `-ll`, and `-lll`. You can override them by simply redefining them.

```toml
[format]
long  = ["perms", "size", "user", "modified"]
long2 = ["perms", "size", "user", "group", "modified", "vcs"]
long3 = ["perms", "links", "size", "blocks", "user", "group",
         "modified", "changed", "created", "accessed", "vcs"]
```

You can explicitly use a format with `--format=NAME`, but more often
you will want to use formats in personalities by using the `format` keyword.

### Personalities

As described [above](#a-file-lister-with-personality), a personality
bundles format, columns, and settings under a name. Every CLI flag
has a corresponding config key (e.g. `--sort=age` becomes
`sort = "age"`).

The built-in personalities form an inheritance tree:

```text
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


### Themes, styles, and classes

Colour customisation uses three kinds of config section that work
together:

- **Themes** (`[theme.NAME]`) set colours for UI elements:
  directories, permissions, dates, VCS status, etc.
- **Styles** (`[style.NAME]`) set colours for files — either by
  referencing a named class or by matching a glob pattern / filename.
- **Classes** (`[class]`) define named file-type categories as lists
  of glob patterns: `media`, `source`, `archive`, etc. Once defined,
  a class can be styled as a unit.

`lx` provides a built-in theme named `"exa"`, and a built-in style, also named 
`"exa"`. These provide sensible defaults out of the box, mirroring the 
[`exa`](https://github.com/ogham/exa) app. You can redefine these, or define 
your own.

The `"exa"` style is provided (commented-out) in the default `~/.lxconfig.toml` 
created by `lx --init-config`. The `"exa"` theme was too large to be included, 
but it is documented in the [`lxconfig.toml(5)`](man/lxconfig.toml.5.md) man 
page.


To define your own theme and/or style:

```toml
[theme.ocean]
inherits = "exa"                      # start from the compiled-in defaults
directory = "bold dodgerblue"
date = "steelblue"
vcs-new = "bold mediumspringgreen"
use-style = "dev"                     # reference a named style set

[style.dev]
class.source = "#ff8700"              # class reference (bare dotted key)
"*.toml" = "sandybrown"               # glob pattern (quoted key)
"Makefile" = "bold underline yellow"  # exact filename (quoted key)
```

Classes let you group file types by category in order to style them:

```toml
[class]
source = ["*.rs", "*.py", "*.js", "*.go"]
data   = ["*.csv", "*.json", "*.xml"]
```

`lx` ships with built-in classes for `image`, `video`, `music`,
`lossless`, `crypto`, `document`, `compressed`, `compiled`, `temp`,
and `immediate` (build/project files). The definitions of these were left out 
of the default `~/.lxconfig.toml` for the sake of brevity, but you can find 
them in the [`lxconfig.toml(5)`](man/lxconfig.toml.5.md) man page.

Override any of the defaults by redefining the name in your `[class]` section. 
Or define your own file classes, as in the example above.

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

#### Curated themes

lx ships with ready-made themes in the `themes/` directory:

| theme           | filename               |
|-----------------|------------------------|
| Catpuccin Mocha | `catpuccin-mocha.toml` |
| Dracula         | `dracula.toml`         |
| Gruvbox Dark    | `gruvbox-dark.toml`    |
| Nord            | `nord.toml`            |
| Solarized Dark  | `solarized-dark.toml`  |
| Solarized Light | `solarized-light.toml` |

To install a theme, simply copy it to the drop-in configuration directory (See 
[Config file locations](#config-file-locations) below).

```sh
mkdir -p ~/.config/lx/conf.d
cp themes/dracula.toml ~/.config/lx/conf.d/
```

Activate a theme on the command line with the `--theme` flag:

```sh
lx -l `--theme=dracula`
````

Or associate the theme permanently with a specific personality in your 
configuration. For instance, to set a theme as default for all personalities 
that inherit from the default:

```toml
[personality.default]
theme = "dracula"
```

See [`themes/README.md`](themes/README.md) for instructions for creating your 
own.

See [`lxconfig.toml(5)`](man/lxconfig.toml.5.md) for the full
list of theme keys, style syntax, and built-in class definitions.


### Debugging your configuration

To see the active configuration, use `lx --show-config`. To extract 
copy-pasteable TOML definitions for any config object, use the `--dump-*` 
flags:

```sh
lx --show-config              # overview of active config

lx --dump-class               # dump all class definitions as TOML
lx --dump-class=temp          # dump a single class
lx --dump-format=long2        # dump a single format
lx --dump-personality=ll      # dump a single personality
lx --dump-style=exa           # dump the exa style
```


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

#### Drop-in directory

After loading the main config, lx looks for a `conf.d/` directory
and loads any `*.toml` files it finds there, in alphabetical order.
Each file is a standalone TOML fragment that can contain theme,
style, class, personality, or format definitions.  The drop-in
directory is searched at:

- `~/.config/lx/conf.d/` (or `$XDG_CONFIG_HOME/lx/conf.d/`)
- `~/Library/Application Support/lx/conf.d/` (macOS)

This is how you install themes from the `themes/` library — just
copy files in, no editing required.


## Installation

### Download a pre-compiled binary

`lx` is just a single binary. You can download pre-built binaries of the latest
release from GitHub, for both Linux and macOS. These binaries all include
jj support.

| OS    | CPU architecture | filename                       |
|-------|------------------|--------------------------------|
| Linux | ARM 64-bit       | `lx-aarch64-unknown-linux-gnu` |
| Linux | Intel 64-bit     | `lx-x86_64-unknown-linux-gnu`  |
| macOS | Apple Silicon    | `lx-aarch64-apple-darwin`      |
| macOS | Intel 64-bit     | `lx-x86_64-apple-darwin`       |

Just download the file, rename it to `lx`, and put it somewhere in your
`$PATH`!

### Build from source

`lx` is built from source using [Cargo](https://doc.rust-lang.org/cargo/),
Rust's package manager. `lx` requires Rust 1.94 or later.

Install Rust if you don't have it already:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install `lx` from [crates.io](https://crates.io/crates/lx-ls):
```sh
cargo install lx-ls                # git support only
cargo install lx-ls --features jj  # + jj support
```

Or install directly from GitHub:
```sh
cargo install --git https://github.com/wjv/lx                # git support only
cargo install --git https://github.com/wjv/lx --features jj  # + jj support
```

The binary is installed to `~/.cargo/bin/` as `lx`.  Make sure this
directory is on your `$PATH`.

Alternatively, build `lx` from a local clone:
```sh
git clone https://github.com/wjv/lx.git
cd lx
cargo build --release
```

The binary is at `target/release/lx`; copy it somewhere on your `$PATH`
(e.g. `~/.local/bin/`).

Man pages are built from Markdown source using
[pandoc](https://pandoc.org/):

```sh
pandoc man/lx.1.md -s -t man -o man/lx.1
pandoc man/lxconfig.toml.5.md -s -t man -o man/lxconfig.toml.5
```

Install them to the standard XDG location:

```sh
mkdir -p ~/.local/share/man/man1 ~/.local/share/man/man5
cp man/lx.1 ~/.local/share/man/man1/
cp man/lxconfig.toml.5 ~/.local/share/man/man5/
```

> **Note:** The crate is published as `lx-ls` on crates.io (the name
> `lx` is taken by an unrelated library).  The installed binary is
> still called `lx`.


### Installing with `just`

If you have [`just`](https://just.systems/) installed, the included
Justfile automates building, installing, and setting up personalities:

```sh
git clone https://github.com/wjv/lx
cd lx                       # create local clone and `cd` to it

just install                # build release + install binary and man pages
                            # to ~/.local/bin and ~/.local/share/man

just install-personalities  # create symlinks for ll, la, lll, tree
                            # in ~/.local/bin

just init-config            # generate ~/.lxconfig.toml
```

To install with jj support, use the `install-jj` recipe instead:

```sh
just install-jj             # build release with jj support + install
                            # to ~/.local/bin and ~/.local/share/man
```

Other useful recipes: `just test`, `just test-all`, `just lint`,
`just completions`. List them all with `just -l`.


## VCS support

`lx` shows per-file version control status in long view, with built-in
backends for both [Git](https://git-scm.com) and
[Jujutsu](https://jj-vcs.dev/) (jj).

> Jujutsu support is optional and needs to be enabled at compile time —
see [Installation](#installation).

```sh
lx -ll                # tier 2 includes VCS status by default
lx --vcs-status -l    # or explicitly
lx --vcs=jj -ll       # force jj backend
lx --vcs=git -ll      # force git backend
lx --vcs=none -ll     # disable VCS
lx --vcs-ignore       # hide VCS-ignored files
```

With `--vcs=auto` (the default), lx probes for a jj workspace first,
then falls back to git — so co-located jj/git repositories are detected
correctly.

The column header (`-h`/`--header`) shows which backend is active: **Git** or 
**JJ**.

### Status characters

| Char | Meaning              |
|------|----------------------|
| `-`  | Not modified         |
| `M`  | Modified             |
| `A`  | Added (jj)           |
| `N`  | New (git)            |
| `D`  | Deleted              |
| `R`  | Renamed              |
| `C`  | Copied               |
| `I`  | Ignored              |
| `U`  | Untracked            |
| `!`  | Conflicted           |

### Git vs jj display

The VCS column is one or two characters wide, depending on the status.

**Git** uses two characters:

- character 1 is the staged status
- character 2 is the unstaged status

When both are the same, `lx` collapses them to one.  For example:

| Column | Meaning                                            |
|--------|----------------------------------------------------|
| `-M`   | Unstaged modification (staged: `-`, unstaged: `M`) |
| `M-`   | Staged modification (staged: `M`, unstaged: `-`)   |
| `-N`   | Untracked file (staged: `-`, unstaged: `N`)        |
| `M `   | Same in both columns (collapsed to one)            |

**jj** also uses two characters, but with different semantics (jj has
no staging area):

- character 1 is the *change status* (working copy commit vs its parent)
- character 2 is the *tracking status* — a space for tracked files, `U` for 
  untracked, or `I` for ignored

| Column | Meaning                 |
|--------|-------------------------|
| `A `   | Added file, tracked     |
| `M `   | Modified, tracked       |
| `- `   | Not modified, tracked   |
| `-I`   | Not modified, ignored   |
| `-U`   | Not modified, untracked |
| `! `   | Merge conflict          |

`--vcs-ignore` works with both backends — it hides files showing `I` 
completely.


## More on daily `lx` usage

### Column visibility

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

### Filtering

```sh
lx -I '*.tmp|*.bak'          # hide files matching globs (-I / --ignore)
lx -T -P 'target|node_modules'  # show dirs but don't recurse (-P / --prune)
lx -TZ -P target              # pruned tree with total sizes — a du replacement!
```

`-I` hides files entirely.  `-P` shows the directory (with its size and
metadata) but doesn't recurse into it — perfect for tree views of projects
with large build or dependency directories.

### Sorting

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

### Environment variables

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

- **jj support is an opt-in feature** — the `jj` feature flag pulls in
  `jj-lib`, which adds ~5 MB to the binary and ~550 extra crates to the
  build.  Build with `cargo build --features jj` (or
  `cargo install lx-ls --features jj`) to enable it.  Without the feature,
  `--vcs=jj` returns a clear error message.

- **0.1 and 0.2 config files need migrating** — the 0.3 config format
  is not backwards-compatible. Run `lx --upgrade-config` to convert
  automatically (a `.bak` backup is saved).

- **The `lx` crate name on crates.io is taken** by an unrelated
  library. Install from GitHub instead (see [Installation](#installation)).

- `lx` is an experiment under active development. Literally anything may still
  change, including the details of the user interface!

## Recent highlights

- **`-C`/`--count`** — print item count to stderr (0.7)
- **Drop-in config directory** (`conf.d/`) and curated theme library (0.6)
- **`--total-size` parallelised** with rayon (0.6)
- **First-class Jujutsu support** via `jj-lib` (0.4)
- Published on [crates.io](https://crates.io/crates/lx-ls) as `lx-ls`

See [CHANGELOG.md](CHANGELOG.md) for the full release history.


## On the horizon

Ideas under consideration for future releases; not promises, but
directions being explored:

- **Conditional config** — per-personality `[[when]]` blocks that
  activate based on environment variables (e.g. enable icons only in
  Ghostty, disable colour over SSH)
- An option to display **platform file flags** (macOS `chflags`,
  Linux `chattr`)
- **Homebrew tap** -- easy installation on macOS


## Acknowledgements

`lx` is my own experiment to test some ideas I have about the user experience 
of a Unix file listing utility. As such, being an experiment, it does not try 
particularly hard to be compatible with anything else. That said, it stands on 
the shoulders of giants!

`lx` is built on the foundations of
[`exa`](https://github.com/ogham/exa) by Benjamin Sago (ogham). The core file 
system, output rendering, and column system are all his work. Thank you! 🌟

Several features were inspired by
[`eza`](https://github.com/eza-community/eza), the active community fork
of `exa` maintained by Christina Sørensen and collaborators. These were
reimplemented from scratch for `lx` and sometimes differ from their `eza`
counterparts:

| Feature                    | eza flag                                     | lx flag                         |
|----------------------------|----------------------------------------------|---------------------------------|
| Recursive directory sizing | `--total-size`                               | `--total-size` / `-Z`           |
| List only files            | `--only-files`                               | `--only-files` / `-f`           |
| Filename quoting           | `--no-quotes` (default on)                   | `--quotes` (default off)        |
| Terminal hyperlinks        | `--hyperlink`                                | `--hyperlink`                   |
| Explicit terminal width    | `--width`                                    | `--width` / `-w`                |
| Absolute paths             | `--absolute`                                 | `--absolute` / `-A`             |
| Symlink control            | `--no-symlinks` + `--follow-symlinks` + `-X` | `--symlinks=show\|hide\|follow` |
| Per-directory repo status  | `--git-repos`                                | `--vcs-repos`                   |


## Licence

MIT — same as the original `exa`. See [LICENCE](LICENCE).
