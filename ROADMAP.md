# Roadmap
<!-- vim: set fo+=at fo-=w tw=72 cc=73 :-->

These are the currently planned themes for the upcoming release cycles
of `lx`.

For shipped work, see [`CHANGELOG.md`](CHANGELOG.md).

## 0.11 release cycle — long view tier flags and hotkeys

### Long-view tiers as personalities

The long view tier flags (`-l`/`-ll`/`-lll`) are reimplemented in
terms of *personalities* rather than *formats*; you can define your own
long-view tiers as `[personality.NAME]` for maximum power and
flexibility. (Already in the nightly builds.)

> Formats as a user-facing configuration element will be de-emphasised
(though retained for power users).

### Hotkeys

A new "hotkey" system will allow for ad hoc personality capture to be
replayed by using flags `-0` through `-9`. An example of intended usage:

```sh
lx -ladD .*                 # show (only) dirs starting in '.'

# Hmm, I'm doing this a lot. Set a hotkey:

lx -ladD --set-hotkey=0     # set hotkey '0'

# Now I can just do:

lx -0 .*
```

Hotkeys are temporary: they last for your login session, and never
touch your config file. If a hotkey turns out to be worth keeping,
save it as a personality with `lx -0 --save-as=NAME`.

### An include filter, and cleaner glob patterns

A new flag (provisionally `-N`/`--include`) will do the opposite of
`-I`/`--ignore`: show *only* the files that match a glob. Unlike
positional arguments, which your shell expands, its patterns can be
saved in a personality or hotkey:

```sh
lx -laD -N '.*' --set-hotkey=0    # dot-directories only
lx -0                             # here, or anywhere: lx -0 ~/src
```

As part of this, the glob patterns accepted by `-I`, `-P` and `-N` will
be cleaned up and made consistent with shell globs. `*` will no longer
match dotfiles, and `|` as a pattern separator will give way to
repeating the flag and to `{a,b}` alternatives. Existing configs will be
migrated by `--upgrade-config`.

### Friendlier configuration inspection (may slip to a later release)

`--show-config` will lead with the settings actually in effect, and
show where each one came from: which personality, which `[[when]]`
block, which file. New `--list-personalities`, `--list-themes` and
friends will give a quick overview of what's defined, with
descriptions. And `--help` will become shorter and easier to scan.

## 0.12 release cycle — icons

The display of nerd font-based file/directory icons is a feature
inherited from `exa` that has as yet seen no further development in
`lx`. In this release, the icon system will be overhauled to make it
compatible with `lx`'s theming, and the theming will be expanded to
fully embrace icons.

Supporting non-nerd font icons (e.g. emoji) will be investigated.

## 0.13 release cycle — "`ls` for the 21st Century"

Modern filesystems support a number of features and abstractions that
traditional `ls` implementations simply don't address. What you see on
the command line is no longer the base truth.

Such features differ from filesystem to filesystem. Examples are:

- POSIX ACLs
- sparse files
- transparent compression
- copy-on-write clones
- immutable, append-only or system-protected files
- dataless cloud placeholders
- file quarantine status
- "firmlinks"
- tags

This release cycle will start to surface such features in the UI (under
full user control, of course). Some UI churn/experimentation may result!

To keep things manageable, initial development in this cycle will
support macOS/APFS *only*. Other operating systems and filesystems will
follow.

## Beyond 0.13

Modern-filesystem-artefacts work will continue past 0.13 — to
platforms beyond macOS/APFS, and to features that don't make it
into the initial cycle.

Have a feature in mind? [Open an
issue](https://github.com/wjv/lx/issues)!
