# Changelog

All notable changes to lx are documented here. lx is forked from
[exa](https://github.com/ogham/exa) v0.10.1.

## [Unreleased]

## [0.1.0] — YYYY-MM-DD

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
