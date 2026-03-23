# Changelog

All notable changes to lx are documented here. lx is forked from
[exa](https://github.com/ogham/exa) v0.10.1.

## [Unreleased]

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
