# Changelog

All notable changes to lx are documented here. lx is forked from
[exa](https://github.com/ogham/exa) v0.10.1.

## [Unreleased] — 0.9.0

### Added

- **Shell completions cover personality symlinks.**  For bash, zsh,
  and fish, `lx --completions SHELL` now also registers completions
  for every symlink in `$PATH` that points to the `lx` binary, so
  `ll <TAB>`, `tree <TAB>`, etc. work out of the box.  Discovery
  is automatic; only genuine symlinks to the running binary are
  registered (no risk of shadowing the real `tree(1)` or `ls(1)`).
  Regenerate completions after creating or removing symlinks.
  Elvish and PowerShell completions are unaffected.
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
- Three-tier group colouring: primary group, supplementary group
  (you have group access), and other.  Theme keys `group-member`
  and `gid-member`.  Uses `getgrouplist()` so it works with
  macOS Directory Services and LDAP.
- **Two new compiled-in themes**: `lx-256` (256-colour, refined
  exa-derived palette) and `lx-24bit` (24-bit truecolour).
  The compiled `default` personality auto-selects the best
  variant based on `$TERM` and `$COLORTERM`: bare terminals get
  `exa`, `*-256color` terminals get `lx-256`, and terminals with
  `COLORTERM=truecolor`/`24bit` get `lx-24bit`.  No flags needed.
- **Theme tier system** in `[[when]]` env conditions: TOML arrays
  match if any element matches; strings containing `*`/`?`/`[`
  are treated as glob patterns.  Both extensions are backwards
  compatible.
- New light-background curated themes: Catppuccin Latte,
  Gruvbox Light, Nord Light.
- **`--gradient`** — per-column gradient on/off for the size and
  timestamp columns.  Vocabulary: `none` / `size` / `date` /
  `modified` / `accessed` / `changed` / `created` / `all`
  (default `all`).  `date` is a bulk setter that flips all four
  timestamp columns at once; `modified`/`accessed`/`changed`/
  `created` flip just one.  Comma-combinable:
  `--gradient=size,modified` or `--gradient=accessed,created`.
  `--no-gradient` is an alias for `--gradient=none`.  Personality
  config key: `gradient = "all"`.  Replaces `--colour-scale`.
- **Per-timestamp-column theme keys** — each of the four timestamp
  columns (`modified`, `accessed`, `changed`, `created`) can be
  themed independently.  For each column there's a `date-<col>`
  bulk setter and seven per-tier setters
  (`date-<col>-now` ... `date-<col>-flat`), 32 new keys total.
  `lx` applies the bulk and per-column keys in specificity
  order automatically, so you can write them in any order in
  the theme block and the most specific one always wins.
- **`--smooth`** — interpolate gradients between the theme's
  per-tier anchor colours in perceptually uniform Oklab colour
  space, instead of snapping to discrete tier boundaries.  Each
  file gets a colour proportional to its position on a log scale
  between adjacent anchors (256 stops, precomputed once per
  theme).  Gated on 24-bit themes whose anchors are all
  `Color::Rgb` — silently a no-op on `lx-256`, `exa`, and any
  palette-based theme.  Default off; opt in via `--smooth`,
  `--no-smooth` suppresses a personality that turns it on.
  Personality config key: `smooth = true`.  The conversion
  formulas come from Björn Ottosson, *"A perceptual color space
  for image processing"* (2020),
  <https://bottosson.github.io/posts/oklab/>.
- New `date-flat` theme key for the colour the date column uses
  when its gradient is off.  Curated themes set it explicitly;
  child themes inherit it through the normal
  `inherits = "..."` chain.
- New dark-background curated themes: `lx-256-dark` and
  `lx-24bit-dark` (drop-in overrides of the builtins with
  brighter gradients), and `the-exa-future` (a 24-bit tribute
  to the original exa look).
- Error on unknown `--theme=NAME` (exit 3, same as unknown `-p`).
- Hidden `--no-dirs-first` / `--no-dirs-last` suppressors, for
  cancelling a personality's `group-dirs` default from the command
  line.  Either flag is equivalent to `--group-dirs=none`.

### Changed

- **`octal-permissions` config key is now `octal`** to match the
  canonical `--octal` flag.  `octal-permissions` continues to work
  as a backward-compatible setting key; the CLI side was already
  using `--octal` as canonical.  Closes the same internal-vs-flag
  mismatch the `--filesize` and `--total-size` renames addressed.
- **`--total-size` is now `--total`** (and `--no-total-size` is
  now `--no-total`).  `--total-size` stays as a *visible* alias
  in `--help` because it's the well-known long form, and the
  config keys `total-size` and `no-total-size` still work as
  backward-compatible setting keys.  Same cross-surface
  consistency rationale as the `--filesize` → `--size` rename
  below; see [`docs/UPGRADING.md`](docs/UPGRADING.md) for the
  detail.
- **`--filesize` is now `--size`** (and `--no-filesize` is now
  `--no-size`).  The file-size column's canonical flag name now
  matches its internal column name (`size` in `--columns=`, `-s`,
  and the registry), closing a long-standing cross-surface
  mismatch.  `--filesize` and `--no-filesize` are still accepted
  as hidden CLI aliases; the config keys `filesize` and
  `no-filesize` still work as backward-compatible setting keys.
  See [`docs/UPGRADING.md`](docs/UPGRADING.md) for the rationale.
- **Flag-alias hygiene.**  `--ignore-glob` and `--prune-glob` are
  now hidden aliases of `--ignore` and `--prune` (previously
  visible).  `--octal-permissions` no longer appears as an
  annotation in `--help` next to `--octal` (still accepted on the
  CLI).  All three were eza-compatibility long forms surfaced in
  `--help` for discoverability; the short canonical names are
  shorter, unambiguous, and less cluttered, so the long forms now
  live as silent compat shims rather than promoted aliases.
- **`mode` accepted everywhere `permissions` is.**  The `--mode`
  flag alias has existed since 0.8 but the spelling was missing
  from `--columns=` (column registry only knew `permissions` and
  `perms`) and from theme keys (only `permissions-*` and `perm-*`
  prefixes existed).  Both gaps closed: `--columns=mode` works,
  and theme keys like `mode-user-read = "red"` work alongside the
  existing `permissions-user-read` and `perm-user-read` spellings.
- **Man pages rewritten in `mdoc(7)`**, hand-authored and committed
  directly as `man/lx.1` and `man/lxconfig.toml.5`.  Drops the
  Markdown-plus-pandoc build pipeline entirely: the `.1`/`.5` files
  are now the source, semantically tagged with `.Fl`/`.Ar`/`.Ev`/
  `.Cm`/`.Pa` instead of presentational bold/italic conventions.
  Fixes a long-standing rendering issue where pandoc mapped
  backticks to `\f[CR]` (constant-width Roman, which has no terminal
  glyph), leaving every flag name, token value, and environment
  variable as undifferentiated plain text.  `just man-lint` validates
  both pages with `mandoc -Tlint -Wwarning`; `just install` and the
  release CI no longer invoke pandoc.
- Non-clap fatal errors now use the same `error:` prefix as clap
  (bold red on a TTY, plain otherwise, `NO_COLOR`-aware) instead
  of the old `lx: ` prefix.  Clap-generated and our own errors
  are now visually indistinguishable.
- Friendlier `--tree --all --all` error message: explains that
  listing `.` and `..` in tree mode would recurse forever,
  phrased in the same style as clap's conflict errors.
- `--dump-theme` output groups `date-*` keys into a structured
  block: bulk keys first in canonical tier order (`now`, `today`,
  `week`, `month`, `year`, `old`, `flat`), then each per-column
  family (`modified`, `accessed`, `changed`, `created`) in the
  same tier order, with blank lines between groups.  Previously,
  per-column keys were interleaved alphabetically with bulk
  keys, obscuring the "baseline + per-column overrides" structure
  that theme authors almost always write.
- Dependabot for Cargo + GitHub Actions dependency updates.
- `cargo-deny` in CI (licence compliance + security advisories).
- Weekly scheduled security audit workflow.
- `--total-size` in tree mode uses parallel pre-computation and
  `(dev, ino)` caching instead of a redundant second filesystem
  walk.  On NFS: **10x faster wall time** vs 0.8 (4s vs 41s
  median), 7x less kernel time (13s vs 85s).  On local SSD:
  wall time unchanged; kernel time marginally improved.
- The compiled-in `exa` theme is now strictly 8-colour ANSI.
  The 256-colour values that crept in during the gradient work
  (`Fixed(244)` for punctuation, the date age gradient) have
  been moved to the new `lx-256` and `lx-24bit` themes.  Users
  on capable terminals get the gradients automatically via the
  `default` personality's auto-selection; users who pin
  `theme = "exa"` get the strict 8-colour rendering.
- UID/GID theme cascade removed.  The `uid-you`, `uid-other`,
  `gid-yours`, `gid-other` theme slots are now independent —
  no longer cascade from `user-*` / `group-*` when unset.
  Custom themes that relied on the cascade should add explicit
  `uid-*` / `gid-*` entries.
- The summary footer (`-C`/`-CZ`) is now coloured: counts and
  totals in `size.major`, surrounding text in `punctuation`.
- Config schema bumps to 0.6 (still backwards compatible —
  0.5 configs load unchanged).  `--upgrade-config` from 0.5
  injects the auto-selection `[[when]]` blocks into existing
  `[personality.default]` sections.
- The theme parser now accepts `permissions-*` keys in addition
  to the legacy `perm-*` short form.  This fixes a long-standing
  bug where the curated themes shipped with `permissions-*`
  keys (matching the column name) but the parser silently
  ignored them.  Backportable to 0.8.
- Linux release builds pinned to `ubuntu-22.04` (glibc 2.35),
  lowering the floor from glibc 2.39.
- **Error handling refactor.**  Every fallible code path now
  bubbles a `thiserror`-based error type up to a single handler
  in `main()`.  New module-level error types (`ThemeError`),
  thiserror conversions for `OptionsError`, new `ConfigError`
  variants for unknown lookups (`NotFound`) and missing upgrade
  targets (`NothingToUpgrade`), and a top-level `LxError` enum
  in `src/main.rs` that wraps them all via `#[from]` for
  ergonomic `?` propagation.  No user-visible CLI changes from
  this; error messages and exit codes are preserved.
- **`--colour-scale` retired.**  The flag's old `none`/`16`/`256`
  vocabulary was about gradient depth and only ever affected the
  size column under the compiled-in `exa` theme.  Replaced by
  `--gradient` (per-column on/off, default on for the size and
  date columns regardless of theme).  `--upgrade-config` rewrites
  `colour-scale = "..."` lines in personality blocks (`16`/`256`
  → `"all"`, `none` → `"none"`).  CLI usage of `--colour-scale`
  now hard-fails at parse time with a migration pointer.  See
  [`docs/UPGRADING.md`](docs/UPGRADING.md) for the full migration
  note.
- **Curated themes get deliberate per-tier gradients.**  Every
  shipped theme in `themes/*.toml` now sets the size and date
  tier colours explicitly with palette-appropriate gradients,
  rather than falling through to a flat-ish bulk setter.
  `--no-gradient` collapses each column to its theme's
  `size-major`/`size-minor` and `date-flat` slots.
- **Broken config files are now fatal.**  Previously a config
  file with invalid TOML or an unsupported version would emit
  a warning and lx would continue with compiled defaults.  Now
  it exits with the relevant error.  A cycle in `[theme.X]`
  inheritance is similarly fatal (was: warn + continue).
  Operating without a config file at all is unaffected.
  Cause: silently ignoring a broken config file hides real
  bugs.  See [`docs/UPGRADING.md`](docs/UPGRADING.md) for the
  full migration note.

### Removed

- **`LX_COLORS` environment variable.**  The lx-specific parallel
  to `LS_COLORS` is gone.  Its ~60 two-letter codes (`ur`, `nk`,
  `da`, `Uy`, etc.) had exhausted the usable namespace —
  recent additions were already reaching for mixed case and
  awkward mnemonics, and new theming features
  (per-timestamp-column keys, per-tier gradients) couldn't be
  expressed in two letters at all.  Every code has a long-
  standing config-file equivalent in `[theme.NAME]` sections,
  which are strictly more powerful (inheritance, per-column
  overrides, X11/hex colours, smooth interpolation).
  `LS_COLORS` is unchanged.  See
  [`docs/UPGRADING.md`](docs/UPGRADING.md) for the migration
  guide with a worked example.


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
