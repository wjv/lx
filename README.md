# lx — file Lister eXtended

**`lx`** is a modern file lister for Unix — a replacement for the standard
`ls` command.

But… `lx` is a file lister with *personality!* 🌟

<img src="docs/images/hero.svg" alt="lx output showing long view with VCS status, file type colours, and size gradient">

> **⚠️  Upgrading from 0.7?  0.8 has breaking CLI changes.**
> `-n` / `--numeric`, `-t FIELD` / `--time`, and the `-u` /
> `-U` short flags have all been reshaped as part of the CLI
> refactor.  Your config file migrates automatically with
> `lx --upgrade-config`; your muscle memory is on its own.
> See [`docs/UPGRADING.md`](docs/UPGRADING.md) for the full list
> and the reasoning — it's short, and each change earns its
> keep.


## Highlights

### 🌟 Personalities

Shell aliases have been wrapping `ls` in user-preferred flags for
forty years.  `lx` takes the idea and promotes it into a feature:
a *personality* is a named bundle of settings, activated by the
name you call `lx` under.  Symlink `lx` to `ll` and `ll` behaves
like `lx -l`.  Create symlinks named `la`, `tree` or `du` — it Just
Works, no shell aliases required.

```sh
ln -s $(which lx) ~/.local/bin/ll     # ll is now "lx long view"
ln -s $(which lx) ~/.local/bin/tree   # tree is now "lx tree view"
```

Personalities can inherit from each other, pick formats and themes,
bundle filter rules, and activate conditionally based on environment
variables (different behaviour inside SSH, in a specific terminal,
on a particular host, …).  `lx` ships with compiled-in personalities
so it works the way you'd expect out of the box — and you can override or 
extend any of them from your config file.

### A CLI you can predict

`lx`'s flag surface is designed to be **orthogonal**.  Every long view column
has the same three things — an add flag, a `--no-*` counterpart, and a sort 
key. Learn a flag once, and you can guess the rest:

```sh
lx -l --inode         # add the inode column to long view
lx -l --no-inode      # remove it (even if the current format includes it)
lx -l -s inode        # sort by it (without having to show the column)
```

Short flags aim to be **guessable mnemonics** — `-u` for `--user`, `-g` for 
`--group`, `-m` for `--modified`.  Related actions share a letter in different 
cases: `-b` and `-B` modify the file size column to show raw *byte* counts
or *Binary* size prefixes, respectively.  `-d` (list directories as files)
pairs with `-D` (list *only* directories).

View detail **compounds**: `-l` fulfils its historical role of showing more 
detail in a "long" view, but using it multiple times (`-ll` / `-lll`) increases 
the amount of detail shown. Similarly `-t`, `-tt` and `-ttt` show progressively 
more timestamp fields.

CLI flags are divided into four **disjoint classes** stay out of each other's 
way.  See [`docs/DESIGN.md`](docs/DESIGN.md) for the full design story.

### Zero config, or every detail

**Out of the box:** no config file needed.  The compiled-in defaults use only 
the 8/256-colour ANSI palette, so a fresh `lx` looks the same on 
a twenty-year-old serial console as it does on a modern 
[Ghostty](https://ghostty.org) — identical columns, identical layout, sensible 
colours… whether your background is light or dark.  The invariant is strict: `lx 
--init-config` generates a config file that *documents* the defaults without 
altering them, so you can go from zero-config to fully-tuned without ever 
crossing a behavioural discontinuity.

**When you want more:** `lx` may be the most flexibly configurable
`ls`-like around.  You've already seen personalities as symlinks;
they come alive inside `lx`'s config file, where they're
just one of **five** kinds of composable section.  The others —
*formats*, *themes*, *styles*, and file-type *classes* — have their own
inheritance or pattern-matching rules, and everything combines freely:

```toml
[theme.work]                    # a theme…
inherits = "catppuccin-mocha"   # …based on a curated preset
use-style = "dev"               # …with its own file-name styling

[style.dev]
class.source = "#ff8700"        # every source file in orange
"Makefile"   = "bold yellow"    # this exact filename in bold yellow

[personality.work]              # a personality using the theme
inherits = "ll"                 # …based on a builtin personality
theme = "work"

[[personality.work.when]]       # conditional: only inside SSH
env.SSH_CONNECTION = true
colour = "never"                # …disable all colour
```

`lx` ships with curated example [themes](themes) which you can drop in 
a `conf.d` directory.

A suite of flags (`--show-config` and `--dump-*`) lets you inspect and
troubleshoot your configuration.

`lx --upgrade-config` migrates schemas between releases automatically.

### Version control integration

`lx` has built-in backends for both [Git](https://git-scm.com)
and [Jujutsu](https://jj-vcs.dev/), with VCS auto-detection.  Per-file status 
optionally appears in long views, and you can choose to exclude files matched 
on repository ignore rules.

The jj backend is opt-in at compile time to keep the default binary small 
— Homebrew and pre-built release binaries include it.

> For a walkthrough of *everything* above, see the [user guide](docs/GUIDE.md)!


## Installation

### Homebrew (macOS and Linux)

```sh
brew tap wjv/tap
brew install lx
```

Installs the `lx` binary (with jj support) and man pages.

### From crates.io

```sh
cargo install lx-ls                # git support only
cargo install lx-ls --features jj  # + jj support
```

The crate is published as [`lx-ls`](https://crates.io/crates/lx-ls)
on crates.io (the name `lx` is taken by an unrelated library).  The
installed binary is still called `lx`.

### Pre-built binaries

Download from the [GitHub releases
page](https://github.com/wjv/lx/releases) for macOS (Intel and Apple
Silicon) and Linux (x86_64 and aarch64).  All release binaries include
jj support.

### Build from source

`lx` requires Rust 1.94 or later.

```sh
git clone https://github.com/wjv/lx
cd lx
cargo build --release --features jj   # binary in target/release/lx
```

If you have [`just`](https://just.systems/), the included `Justfile`
automates installation, man pages, personality symlinks, and
completions — run `just -l` to see the recipes.


## Documentation

- **[`docs/GUIDE.md`](docs/GUIDE.md)** — the user guide: personalities,
  configuration, themes, VCS, daily usage, shell completions.
- **[`docs/DESIGN.md`](docs/DESIGN.md)** — the design philosophy behind
  `lx`'s CLI; why things are shaped the way they are.
- **[`docs/UPGRADING.md`](docs/UPGRADING.md)** — breaking changes per
  release, with migration notes and justifications.
- **`man lx`** ([source](man/lx.1.md)) — command reference.
- **`man lxconfig.toml`** ([source](man/lxconfig.toml.5.md)) —
  configuration file reference.
- **[`CHANGELOG.md`](CHANGELOG.md)** — release notes.
- **`lx --help`** — online flag reference.


## Known limitations

- **jj support is opt-in** at compile time.  Release binaries
  (Homebrew, GitHub releases) include it; `cargo install` defaults to
  git-only.  Build with `--features jj` to enable.
- **Old config files** from before 0.5 need migrating.  Run
  `lx --upgrade-config` to convert from any earlier format (a `.bak`
  of the original is saved automatically).
- **The crate name on crates.io is `lx-ls`**; the binary is still `lx`.
- `lx` is an experiment under active development.  The CLI surface
  is not yet stable.


## Acknowledgements

`lx` stands on the shoulders of giants.

It is built on the foundations of [`exa`](https://github.com/ogham/exa)
by Benjamin Sago (ogham).  The core file system, output rendering, and
column system are all based on his work, and that of his contributors.
Thank you! 🌟

Several features were inspired by
[`eza`](https://github.com/eza-community/eza), the active community
fork of `exa` maintained by Christina Sørensen and collaborators.
These were reimplemented from scratch for `lx` and sometimes differ
from their eza counterparts.


## Licence

MIT — same as the original `exa`.  See [LICENCE](LICENCE).

## Pronunciation

`lx` is pronounced "alex". Or "ell-ex". Really, you choose.
