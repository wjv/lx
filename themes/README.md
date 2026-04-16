# `lx` themes

Curated colour themes for `lx`. Each file is a standalone TOML fragment
containing a `[theme.NAME]` (and sometimes `[style.NAME]`) section.

## Installation

Copy one or more theme files to your config drop-in directory:

```sh
# Create the drop-in directory (if it doesn't exist)
mkdir -p ~/.config/lx/conf.d

# Copy a theme
cp themes/dracula.toml ~/.config/lx/conf.d/
```

## Activation

Use the theme via `--theme=NAME` on the command line, or set it in a
personality in your config file:

```toml
[personality.lx]
theme = "dracula"
```

## Available themes

### Built-in (both backgrounds)

Three themes ship inside the `lx` binary — no drop-in needed.
The default personality auto-selects the best variant for your
terminal:
* `lx-24bit` if `$COLORTERM` is `truecolor` or `24bit`,
* `lx-256` if `$TERM` matches `*-256color`,
* otherwise `exa`.

| File            | Theme name | Description                                   |
|-----------------|------------|-----------------------------------------------|
| `exa.toml`      | `exa`      | Strict 8-colour ANSI; the heritage `exa` look |
| `lx-256.toml`   | `lx-256`   | 256-colour, balanced for light and dark       |
| `lx-24bit.toml` | `lx-24bit` | 24-bit truecolour, smoothest gradients        |

The TOML files are reference copies of what the binary already
provides. You *could* copy these to `~/.config/lx/conf.d/` and
use them as a basis for customisation. However, a more elegant solution is 
to create your own theme that *inherits* from one of the builtin themes — 
see [docs/GUIDE.md](../docs/GUIDE.md) for the details on how this works.

### Light backgrounds

| File                    | Theme name         | Description                                        |
|-------------------------|--------------------|----------------------------------------------------|
| `catppuccin-latte.toml` | `catppuccin-latte` | Warm pastels on light background                   |
| `gruvbox-light.toml`    | `gruvbox-light`    | Retro warm palette, light variant                  |
| `nord-light.toml`       | `nord-light`       | Arctic palette, light variant                      |
| `solarized-light.toml`  | `solarized-light`  | Ethan Schoonover's light palette; per-column dates |

The Solarized Light theme uses the per-timestamp-column theming feature to 
give each of the four date columns (modified, accessed, changed, created) 
its own hue family for recent files.

The other themes use single bulk `date-*` keys that apply uniformly to all 
four timestamp columns.

### Dark backgrounds

| File                    | Theme name         | Description                                       |
|-------------------------|--------------------|---------------------------------------------------|
| `catppuccin-mocha.toml` | `catppuccin-mocha` | Warm pastels on dark background                   |
| `dracula.toml`          | `dracula`          | Dark theme with vibrant colours                   |
| `gruvbox-dark.toml`     | `gruvbox-dark`     | Retro warm palette, dark variant                  |
| `nord.toml`             | `nord`             | Arctic, north-bluish palette                      |
| `solarized-dark.toml`   | `solarized-dark`   | Ethan Schoonover's dark palette; per-column dates |
| `lx-256-dark.toml`      | `lx-256-dark`      | Dark-tuned variant of the builtin lx-256          |
| `lx-24bit-dark.toml`    | `lx-24bit-dark`    | Dark-tuned variant of the builtin lx-24bit        |

The last two mirror the compiled-in `lx-256` and `lx-24bit`
themes, but with brighter gradients tuned specifically for dark
backgrounds.  Useful if you find the default builtins too
muted on black!

The Solarized Dark theme again uses per-timestamp-column theming, just like 
its Light counterpart.

### Both light and dark backgrounds

A special bonus theme to celebrate the original `exa`, now in modern 24-bit RGB!

| File                   | Theme name        | Description                                |
|------------------------|-------------------|--------------------------------------------|
| `the-exa-future.toml`  | `the-exa-future`  | Tribute to the original exa, in 24-bit RGB |

## Creating your own

You can of course create your own theme from scratch. Using these curated 
themes as a basis is a good way to get started!

```toml
[theme.my-theme]
inherits = "exa"       # or "lx-256" or "lx-24bit"
use-style = "my-theme" # reference the style below
directory = "bold #hexcolour"
date = "#hexcolour"

[style.my-theme]
class.image = "#hexcolour"
class.video = "#hexcolour"
```

See **lxconfig.toml**(5) for the full list of theme keys, and
[docs/GUIDE.md](../docs/GUIDE.md) for a tutorial on theme creation.