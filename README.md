# lx

**lx** ("alex") is a modern file lister for Unix, forked from
[exa](https://github.com/ogham/exa) by Benjamin Sago.

exa appears to be unmaintained. An active community fork,
[eza](https://github.com/eza-community/eza), exists; lx is forked from
exa's simpler codebase for greater freedom in CLI design experimentation.

## What's different?

lx diverges from exa in three main areas:

- **Redesigned CLI.** Compounding flags (`-l`/`-ll`/`-lll` for
  escalating detail), symmetric column visibility (`--no-*` and
  positive counterparts for every column), unified valued flags
  (`--group-dirs=first|last|none`, `--colour=always|auto|never`).

- **Configurable column layouts.** Named formats, personalities, and
  (planned) a configuration file to replace shell aliases with a
  single `config.yml` — including argv[0] dispatch so that symlinks
  like `ll`, `tree`, and `la` just work.

- **VCS-agnostic version control support.** `--vcs=auto|git|jj|none`
  with built-in backends for both Git (via `git2`) and
  [Jujutsu](https://github.com/jj-vcs/jj) (via the `jj` CLI).
  Auto-detection prefers jj when a `.jj/` workspace is present.

## Status

lx is in early development (0.1.x). The CLI surface is unstable and
may change between releases. Run `lx --help` for the current flags.

## Building

```sh
cargo build --release
```

Requires Rust 1.94 or later.

## Licence

MIT — same as the original exa.
