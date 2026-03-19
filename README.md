# lx --- an eXtended file Lister

**`lx`** ("alex") is a modern file lister for Unix. A replacement (albeit 
explicitly *not* a drop-in replacement) for the standard `ls`.

`lx` is forked from [`exa`](https://github.com/ogham/exa) by Benjamin Sago. `exa` 
appears to be unmaintained.

An active community fork of `exa` named
[`eza`](https://github.com/eza-community/eza) exists, but `lx` is an experiment
with a somewhat different approach to the command-line user interface.


## Highlights

- **Personalities** --- named profiles that bundle columns, flags, and
  settings

  Create symlinks (`ll`, `ls`, `la`, `tree`) and `lx` adapts its behaviour to 
  the name it's invoked as!

- **Fully configurable column layout**

  `--columns` gives you complete control over which columns are displayed *and*
  their order: `--columns=perms,size,user,modified`.  
  `--format` allows you to apply named sets of columns: `--format=long2`.

- **"Compounding" flags** --- flags that compound their effect when repeated

  Use `-l` for a long listing, `-ll` for more detail, and `-lll` for even more.  
  No more remembering which combination of `-g`, `-H`, `-h`, and `--git` you 
  need!

- **Configuration file**

  One `lxconfig.toml` replaces all your shell aliases and environment 
  variables. Define defaults, custom formats, and personalities. Run
  `lx --init-config` to get started.

- **Unified VCS support**

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

```sh
lx                    # grid view (like ls)
lx -l                 # long view: perms, size, user, modified
lx -ll                # + group, VCS status
lx -lll               # + header, all timestamps, links, blocks
lx -T                 # tree view
lx -T -L2             # tree, depth 2
```

### Personalities

Instead of shell aliases, use personalities:

```sh
# Create symlinks once:
ln -s $(which lx) ~/bin/ll
ln -s $(which lx) ~/bin/lll
ln -s $(which lx) ~/bin/tree
ln -s $(which lx) ~/bin/la

# Then just use them:
ll                    # long view, group, VCS, dirs first
lll                   # all columns, header, long-iso timestamps
tree                  # long tree view, dirs first
la                    # long view + hidden files
```

Or use the `-p`/`--profile` flag directly:

```sh
lx -pll               # "longer" long view
lx --profile tree     # tree view
```

The personalities mentioned above (`ll`, `lll`, `la`, `tree`) are 
compiled-in defaults, as is `ls` (mimics "plain" ls).  You can edit these or 
define your own in the config file.


## Configuration

Generate a starter config with:

```sh
lx --init-config
```

This creates `~/.lxconfig.toml` with commented examples. The config
file has three sections: Defaults, Formats, and Personalities.

### Defaults

```toml
[defaults]
colour = "always"
time-style = "long-iso"
group-dirs = "first"
```

These are prepended as flags before your CLI arguments, so explicit
flags always win.


### Formats

A format is a named column layout:

```toml
[formats.compact]
columns = ["perms", "size", "modified"]

[formats.hpc]
columns = ["perms", "size", "user", "group", "modified", "vcs"]
```

Use with `--format=compact` or reference from a personality.

Available column names: `perms`, `size`, `user`, `group`, `links`,
`inode`, `blocks`, `octal`, `modified`, `changed`, `accessed`,
`created`, `vcs`, `totalsize`.


### Personalities

A personality bundles columns, flags, and settings:

```toml
[personalities.lt]
format = "long2"
flags = ["--group-dirs=first", "--sort=age"]

[personalities.stree]
columns = ["totalsize"]
flags = ["--tree", "--only-dirs", "--reverse", "--sort=size",
         "--total-size", "--level=1"]
```

Invoke a personality with `-p NAME` or `--personality=NAME`, or by creating a
symlink with the personality name pointing to the lx binary.


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

  The `jj` CLI currently has no way to report which files are gitignored. Workarounds:
  - Use `--vcs=git --vcs-ignore`
  - Use `-I` glob patterns to exclude specific files (e.g. `-I target`).

- **The `lx` crate name on crates.io is taken** by an unrelated
  library. Install from GitHub instead (see [Installation](#installation)).


## User interface stability

`lx` is under active development and literally anything may still change.


## Licence

MIT — same as the original `exa`. See [LICENCE](LICENCE).
