# Changelog

All notable changes to lx are documented here. lx is forked from
[exa](https://github.com/ogham/exa) v0.10.1.

## [Unreleased] — 0.9.0

### Added

- `$LX_PERSONALITY` environment variable for session-level
  personality selection.  Resolution pipeline:
  `-p` → argv[0] → `$LX_PERSONALITY` → default.
- `--save-as=NAME` writes the current CLI flag delta as a
  personality to `conf.d/NAME.toml`.
- `--show-config` now shows an "activated by" line indicating how
  the personality was chosen.
- `grid-rows` and `icon-spacing` personality config keys (previously
  `LX_GRID_ROWS` / `LX_ICON_SPACING` env-var-only).
- `decimal-point` and `thousands-separator` personality config keys
  for overriding locale numeric formatting.
- Age-based timestamp colouring: six tiers from "just now"
  (bright cyan) to "old" (grey).  Theme keys `date-now` through
  `date-old`; `date` remains a bulk setter for backwards
  compatibility.
- Three new light-background curated themes: Catppuccin Latte,
  Gruvbox Light, Nord Light.
- Error on unknown `--theme=NAME` (exit 3, same as unknown `-p`).
- Dependabot for Cargo + GitHub Actions dependency updates.
- `cargo-deny` in CI (licence compliance + security advisories).
- Weekly scheduled security audit workflow.

### Changed

- `--total-size` in tree mode uses parallel pre-computation and
  `(dev, ino)` caching instead of a redundant second filesystem
  walk.  On NFS: **10x faster wall time** vs 0.8 (4s vs 41s
  median), 7x less kernel time (13s vs 85s).  On local SSD:
  wall time unchanged; kernel time marginally improved.
- UID/GID theme cascade removed.  The `uid-you`, `uid-other`,
  `gid-yours`, `gid-other` theme slots are now independent —
  no longer cascade from `user-*` / `group-*` when unset.
  Custom themes that relied on the cascade should add explicit
  `uid-*` / `gid-*` entries.
- Linux release builds pinned to `ubuntu-22.04` (glibc 2.35),
  lowering the floor from glibc 2.39.


## [0.8.0] — 2026-04-07

The 0.8 release is the CLI-refactor release.  A string of batches
(`0.8.0-pre.2` through `0.8.0-pre.10`) reshaped the flag surface
around the orthogonal design described in
[`docs/DESIGN.md`](docs/DESIGN.md) — every column has a positive
flag and a `--no-*` counterpart, every column is sortable, and
timestamps are ordinary columns rather than a render-mode toggle.

Internally, both the column list and the sort-field vocabulary
are now driven by data-driven registries, so adding a new column
or sort field touches two files instead of six.

Breaking changes are gathered under **Removed** and **Changed**
below.  Old configuration files (through schema version 0.4) can
be migrated with `lx --upgrade-config`.

### Added
- **Compounding timestamp tiers** — `-t` / `-tt` / `-ttt` add one,
  two, or four timestamp columns respectively.  `-t` is
  `--modified`; `-tt` adds `--changed`; `-ttt` adds `--created`
  and `--accessed` on top.  Composes with any format or `-l` tier.
- **`--uid` and `--gid`** as first-class long-only columns.
  Headers `UID` and `GID`, right-aligned.  Hidden `--no-uid` /
  `--no-gid` suppressors.
- **`-M` / `--permissions`** (with `--mode` as a long alias) for
  the symbolic permission-bits column.
- **`-z` / `--filesize`** (with `--size` as a long alias) for the
  file-size column.
- **`-u` / `--user`** — short flag for the owner column (`-u` was
  previously `--accessed`).
- **Expanded sort vocabulary** — every metadata column is now a
  sort key.  New `--sort` values: `permissions` (aliases `mode`,
  `octal`), `blocks`, `links`, `flags`, `user` / `User`, `group`
  / `Group`, `uid`, `gid`, `version` (natural / version sort on
  names), `vcs` (group by VCS status).
- **Data-driven column registry** (`ColumnDef` in
  `src/output/column_registry.rs`) feeding both the CLI parser
  and the table renderer.  Adding a column touches the registry
  and one render module.
- **Data-driven sort-field registry** mirroring the column one.
  `SortField::compare_files` and `SortField::deduce` both collapse
  to registry lookups; clap's sort-value list is auto-generated.
- **256-colour theme invariant** — the compiled default theme now
  uses only 8/256-palette colours and does not rely on `is_dimmed`
  for visual hierarchy, so it works predictably on 256-colour
  terminals.
- **UID/GID theme cascade** — `UiStyles::Users` gains four
  `Option<Style>` slots (`uid_you`, `uid_someone_else`,
  `gid_yours`, `gid_not_yours`) with a two-stage placeholder
  cascade: stale placeholders are invalidated when a parent is
  overridden, then remaining slots fall back to a dim copy of
  their parent.  New `LX_COLORS` codes `Uy`/`Un`/`Gy`/`Gn`.
- **`--size-style=decimal|binary|bytes`** — valued flag for size
  display mode, parallel to `--time-style`.  Config key:
  `size-style`.  Closes the asymmetry where a personality setting
  `binary = true` could not be overridden back to decimal.
- **`-K` / `--decimal`** — new short flag selecting decimal size
  prefixes (alias for `--size-style=decimal`).
- **`--help` respects `NO_COLOR` and stderr TTY state** — help
  output is now plain text when `NO_COLOR` is set or stderr is
  not a terminal.
- **`--completions` flag** documented in `lx(1)` man page (was
  missing from the command reference).
- **README badges** — CI status, crates.io version, MSRV, licence.

### Changed
- **`perms` column renamed to `permissions`.**  The canonical
  column name matches the `--permissions` flag.  `perms` is still
  accepted as a backward-compat alias in `--columns=`, but not as
  a sort field.
- **Long view `--help` reordered** into canonical column order
  (inode, octal, permissions, flags, links, size, blocks, user,
  uid, group, gid, timestamps, …) so the help text matches the
  insertion order used when individual column flags are added.
- **Timestamps are ordinary columns.**  They can be added and
  suppressed like any other metadata column (`--modified` /
  `--no-modified`, etc.).  `-t` no longer takes a field name.
- **Six curated themes** (`catppuccin-mocha`, `dracula`,
  `gruvbox-dark`, `nord`, `solarized-dark`, `solarized-light`)
  had a `group = "..."` line that was silently ignored because
  `group` is not a valid theme key.  Fixed.
- **Config schema bumped to 0.5.**  Migrations from 0.1, 0.2,
  0.3, and 0.4 are all supported by `--upgrade-config`.  The
  `time = "…"` and `numeric = …` personality settings were
  removed as part of the timestamp redesign and UID/GID column
  work; the migration emits a deprecation warning and drops
  them.
- **`-b` / `-B` short flags swapped.**  `-b` is now `--bytes`
  (was `--binary`); `-B` is now `--binary` (was `--bytes`).
  Mnemonic: lowercase for the simple raw byte count, uppercase
  for the Binary prefix formatting system.  Long forms unchanged.
- **`--show-config` and `--dump-*` output** now says `builtin`
  instead of `compiled-in` (shorter, friendlier).
- **Curated themes** — `group-yours` now uses an on-palette accent
  colour distinct from `group-other` in all six shipped themes.
- **`lx(1)` man page restructured** — sorting split into its own
  section; column overrides moved adjacent to column selection;
  personalities section expanded with compiled-in list and TOML
  example.

### Fixed
- **`-p` / `--personality` with an unknown name** now exits with
  an error (exit code 3) instead of silently falling through to
  defaults.

### Removed
- **`-n` / `--numeric`** has been retired entirely.  Use `--uid`
  / `--gid` as first-class columns, or define a `numeric`
  personality with `inherits = "ll"`, `uid = true`, `gid = true`,
  `no-user = true`, `no-group = true`.
- **`-t FIELD` / `--time`.**  Replaced by the compounding `-t` /
  `-tt` / `-ttt` tiers and the individual `--modified` /
  `--changed` / `--accessed` / `--created` flags.
- **`-u` as `--accessed`** and **`-U` as `--created`.**  `-u` is
  now `--user`; `-U` has been freed.  Both timestamp flags are
  still available in long form.
- **`UserFormat` enum** and its plumbing.  `f::User::render` and
  `f::Group::render` no longer take a format argument; they fall
  back to numeric IDs automatically when name resolution fails.

### Docs
- **`README.md` slimmed** from 857 to ~220 lines.  The manual
  content moved to a new `docs/GUIDE.md`; the README is again a
  landing page with four top-tier Highlights.
- **New `docs/GUIDE.md`** — the user guide.  Personalities,
  configuration, themes, VCS, daily usage patterns, size display,
  shell completions, and configuration debugging.
- **New `docs/UPGRADING.md`** — per-release breaking-change list
  with migration notes and justifications.  The 0.8 section leads
  with additions and treats removals as consequences.
- **`docs/DESIGN.md` refreshed** with a new "The orthogonal CLI"
  section distilling the 0.8 flag principles, and an updated
  short-flag reference table.

## [0.7.0] — 2026-04-04

### Added
- **Conditional config** — `[[personality.NAME.when]]` blocks that
  activate based on environment variables. Conditions use
  `env.VAR = "value"` (exact match), `env.VAR = true` (must be set),
  or `env.VAR = false` (must be unset). Enables per-terminal settings
  (e.g. icons in Ghostty, disable colour over SSH). Config schema
  version bumped to 0.4 (0.3 configs still accepted;
  `--upgrade-config` handles 0.3→0.4).
- `-C`/`--count` — print item count to stderr. Combined with `-Z`,
  also shows total size of displayed items (no double-counting in
  tree views). Respects `-b`/`-B` size formatting.
- `-O`/`--flags` — show platform file flags. macOS/FreeBSD: `chflags`
  attributes (hidden, uchg, uappnd, nodump, uarch, etc.). Linux:
  `chattr` attributes via ioctl (immutable, append, nodump, noatime,
  etc.). Available as a column (`--columns=flags`).
- `--no-count`, `--no-total-size`, `--no-header`, `--no-octal` —
  override flags for suppressing personality defaults. Hidden
  `--no-X` short aliases (e.g. `--no-C`, `--no-Z`, `--no-h`) also
  accepted.
- CI: automatic publishing to crates.io and Homebrew tap on release.
- CI: man pages built with pandoc and included in release assets.
- Homebrew installation: `brew tap wjv/tap && brew install lx`.
- `just release-check` recipe for pre-publish verification.

### Fixed
- `--icons=auto`, `--classify=auto`, and `--hyperlink=auto` now
  check whether stdout is a terminal. Previously `auto` behaved
  identically to `always`.
- Config personality settings: `ignore`, `prune`, `symlinks`,
  `classify`, `flags`, and `vcs-repos` were accepted on the CLI but
  rejected in personality definitions.
- Cargo.toml: use `dep:git2` to prevent implicit feature exposure;
  pin `serde` to `1.0`, `toml` to `1.1`.

### Changed
- `--help` reorganised: Long view before Filtering, positive enablers
  (`--permissions`, `--filesize`, `--user`) moved to Long view,
  negation flags in "Column overrides" section. Personality-only
  `--no-*` flags hidden from `--help` (documented in man page).
- Bump jj-lib dependency to 0.40.

## [0.6.3] — 2026-04-03

### Fixed
- Config personality settings: `ignore`, `prune`, `symlinks`, `classify`,
  `flags`, and `vcs-repos` were accepted on the CLI but rejected in
  personality definitions. All CLI flags are now available as config keys.

## [0.6.2] — 2026-04-02

### Added
- CI: automatic publishing to crates.io and Homebrew tap on release.
- CI: man pages built with pandoc and included in release assets.
- Homebrew installation: `brew tap wjv/tap && brew install lx`.
- `just release-check` recipe for pre-publish verification.

## [0.6.1] — 2026-04-02

### Fixed
- `--icons=auto`, `--classify=auto`, and `--hyperlink=auto` now
  check whether stdout is a terminal.  Previously `auto` behaved
  identically to `always`, emitting icons, file indicators, and
  OSC 8 hyperlinks even when piped.

## [0.6.0] — 2026-04-01

### Added
- **Drop-in config directory** (`conf.d/`): load additional TOML
  fragments from `~/.config/lx/conf.d/` (or XDG/macOS equivalent).
  Files loaded in alphabetical order; later files override earlier
  ones by name.  Useful for installing shared themes without editing
  the main config.
- **Curated theme library** in `themes/`: Catppuccin Mocha, Dracula,
  Gruvbox Dark, Nord, Solarized Dark, Solarized Light.  Copy to
  `conf.d/` to activate.
- `--show-config` now lists loaded drop-in files.
- Accept US spelling `color` and `color-scale` in config file.
- Published on crates.io as `lx-ls` (`cargo install lx-ls`).

### Changed
- Icon assignment migrated to the class system: media-type icons
  (audio, image, video) now use `[class]` config instead of hard-coded
  extension checks.  Custom class definitions affect icons too.
- `--total-size` parallelised with rayon — significantly faster on
  large trees, especially on network filesystems.
- `--help` tidied: possible values shown inline, noisy aliases hidden.
- Crate renamed from `lx` to `lx-ls` for crates.io (binary is still `lx`).

### Removed
- `src/info/` module (dead code: `filetype.rs` extension checks
  superseded by class system, `sources.rs` never called)

## [0.5.0] — 2026-03-27

### Added
- `-P`/`--prune` — show directories but don't recurse into them
  (tree pruning); same glob syntax as `-I`/`--ignore`
- `--time-style=relative` — human-friendly durations ("2 hours ago")
- `--time-style='+FORMAT'` — custom strftime format strings
- `--dump-class`, `--dump-format`, `--dump-personality`, `--dump-theme`,
  `--dump-style` flags for copy-pasteable TOML output (bare = all,
  `=NAME` = single definition)
- `--init-config`, `--upgrade-config`, `--completions` now visible in
  `--help` output
- `--symlinks=show|hide|follow` — control symlink display and
  dereferencing (combines eza's `--no-symlinks`, `--follow-symlinks`,
  and `-X`/`--dereference` into one flag)
- `--vcs-repos` — show per-directory VCS repo indicator (`G`/`J`/`-`)
  with branch name for git repos
- Hero screenshot in README

### Changed
- `--help` reorganised with section headings (Display, Filtering,
  Long view, Timestamps, Column visibility, VCS, Appearance,
  Configuration)
- `--ignore-glob` renamed to `--ignore` (old name kept as alias)
- `--octal-permissions` renamed to `--octal` (old name kept as alias)
- `--group-directories-first`/`last` shortened to `--dirs-first`/`last`
  (old names kept as aliases)
- `--vcs-ignore` now also hides VCS metadata directories (`.git`, `.jj`)
- `--total-size` performance: cached recursive sizes avoid redundant
  directory walks (~3x faster on large trees)

## [0.4.0] — 2026-03-26

### Added
- First-class Jujutsu (jj) support via `jj-lib` crate (opt-in `jj`
  feature flag):
  - Two-column VCS display: change status + tracking status
  - `A` (added) matches jj's own `jj diff --summary` output
  - `--vcs-ignore` with full gitignore support via `git2` (all layers:
    global `core.excludesFile`, `.git/info/exclude`, per-directory
    `.gitignore`)
  - Untracked (`U`) and conflicted (`!`) file detection
  - Dynamic column header: **Git** or **JJ** depending on backend
  - Works with colocated, non-colocated, and external git repos
  - Conflict detection via `MergedTreeValue.is_resolved()`
- `-F`/`-J` short flags for `--group-dirs=first`/`last`
- `-o` short flag for `--octal-permissions`
- `-A` short flag for `--absolute`
- Canonical column insertion order for individual flags
- Coloured `--show-config` output
- CI tests both with and without `jj` feature
- Release binaries include jj support
- Justfile recipes for jj builds (`build-jj`, `install-jj`, `test-jj`,
  etc.)

### Changed
- jj feature implies `git` (jj repos are backed by git)
- jj is opt-in (`--features jj`) due to ~5 MB binary size impact;
  `--vcs=jj` without the feature gives a clear error message
- `--show-config` now uses colour (yellow headers, cyan names, green
  values, dimmed source annotations)

### Removed
- CLI-based jj backend (replaced by jj-lib integration)

## [0.3.0] — 2026-03-25

### Added
- File-type classes: `[class]` section with named pattern lists and
  compiled-in defaults (`image`, `video`, `music`, `lossless`, `crypto`,
  `document`, `compressed`, `compiled`, `temp`, `immediate`)
- Styles reference classes via bare dotted TOML keys (`class.NAME = "colour"`)
  and file patterns via quoted keys (`"*.ext" = "colour"`)
- Compiled-in `"exa"` style maps classes to default colours
- Explicit exa chain: default personality → exa theme → exa style
  (no magic fallback)
- `--show-config` flag to display the active personality, theme, style,
  classes, and formats with their source (compiled-in vs config)
- `la` compiled-in personality (inherits `ll`, shows hidden files)
- Config schema version bumped to `"0.3"`
- Upgrade tool handles 0.1→0.3 and 0.2→0.3 migrations
- Compiled-in `default` and `lx` personalities matching the
  default config template
- Clap `wrap_help` for readable `--help` on wide terminals

### Changed
- Formats are now flat `[format]` sections (was `[format.NAME]` with
  `columns` sub-key)
- `--group-directories-first` now uses `overrides_with` for proper
  precedence against personality-injected `--group-dirs` values

### Removed
- `--git` and `--git-ignore` legacy flags (use `--vcs-status`
  and `--vcs-ignore`)
- `reset-extensions` option (replaced by explicit style references)
- Dead `FileColours` impl and unused `is_*` methods from `filetype.rs`

## [0.2.1] — 2026-03-23

### Fixed
- Add compiled-in `default` and `lx` personalities so the tool
  behaves the same with or without a config file

## [0.2.0] — 2026-03-23

### Added
- Configuration redesign: personality inheritance (`inherits`), named settings for all CLI flags, config versioning (`version = "0.2"`), `--upgrade-config` migration tool
- Theme system: `[theme.NAME]` sections with human-readable colour values (named ANSI, X11/CSS names, hex `#RRGGBB`), theme inheritance, `--theme=NAME` flag
- `[style.NAME]` file colour sets with glob patterns, referenced from themes
- `-w`/`--width` for explicit terminal width
- `--absolute` for absolute file paths
- `--hyperlink` for OSC 8 clickable file names
- `--quotes` for quoting file names containing spaces
- `phf` crate for X11 colour name lookup (148 names)
- `thiserror` crate for typed configuration errors

### Changed
- `[defaults]` config section replaced by `[personality.lx]` (or user-defined base personality)
- Config-file personalities use named settings instead of `flags` arrays
- `la` removed from compiled-in personality defaults (available as config example)
- Default config template uses `##` for prose comments, uncommented structural sections
- Test helpers now isolate from user config (`LX_CONFIG`/`HOME`)

### Removed
- `[defaults]` config section (use `[personality.default]` + inheritance)
- `flags` field in personality definitions (use named settings)

## [0.1.1] — 2026-03-20

### Fixed
- Release workflow now produces distinct binaries per platform
  (was overwriting with a single `lx` file)

## [0.1.0] — 2026-03-20

First release of lx. Major changes from the exa base:

### Added
- Compounding `-l`/`-ll`/`-lll` for tiered detail levels
- `--columns` and `--format` for dynamic column selection
- Personalities (`-p`/`--personality`) with argv[0] dispatch
- Configuration file (`~/.lxconfig.toml`) with `--init-config`
- Unified VCS support: `--vcs=auto|git|jj|none`
- jj (Jujutsu) VCS backend
- `-Z`/`--total-size` for recursive directory sizing
- `-f`/`--only-files`
- `-c`/`--changed`
- `--group-dirs=first|last|none`
- Symmetric column visibility flags (`--no-inode`, `--permissions`, etc.)
- `--colour-scale=16|256|none`
- Shell completions via `--completions`

### Changed
- Binary renamed to `lx`
- `--colour` is the primary flag; `--color` is an alias
- `--classify` and `--icons` accept `=always|auto|never`
- Environment variables renamed: `EXA_*` → `LX_*`
- `--git`/`--git-ignore` are hidden aliases for `--vcs-status`/`--vcs-ignore`
- `-F` short flag removed from `--classify`

### Removed
- `EXA_STRICT` mode
- Hand-written shell completions (replaced by `clap_complete`)
- `build.rs` (version from `CARGO_PKG_VERSION`)
