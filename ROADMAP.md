# Roadmap
<!-- vim: set fo+=at fo-=w tw=72 cc=73 :-->

These are the currently planned themes for the upcoming release cycles
of `lx`.

For shipped work, see [`CHANGELOG.md`](CHANGELOG.md).

## 0.11 release cycle — long view tier flags and hotkeys

This release cycle will see the reimplementation of the long view tier
flags (`-l`/`-ll`/`-lll`) in terms of *personalities* rather than
*formats*; you will be able to define your own long-view tiers as
`[personality.NAME]` for maximum power and flexibility.

> Formats as a user-facing configuration element will be de-emphasised
(though retained for power users).

A new "hotkey" system will allow for ad hoc personality capture to be
replayed by using flags `-0` through `-9`. An example of intended usage:

```sh
lx -ladD .*                 # show (only) dirs starting in '.'

# Hmm, I'm doing this a lot. Capture a hotkey:

lx -ladD --save-hotkey=0    # save hotkey '0'

# Now I can just do:

lx -0 .*
```

A new flag (provisionally: `-N`) will enable the capture of positional
arguments as glob patterns, analogous to the existing `-I` and `-P`.
This will enable more flexible personalities, and should be especially
useful when using hotkeys.

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
