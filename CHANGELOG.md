# Changelog

All notable changes to lx are documented here. lx is forked from
[exa](https://github.com/ogham/exa) v0.10.1.

## [Unreleased]

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
