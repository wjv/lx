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

### Light backgrounds

| File                    | Theme name         | Description                                          |
|-------------------------|--------------------|------------------------------------------------------|
| `catppuccin-latte.toml` | `catppuccin-latte` | Warm pastels on light background                     |
| `gruvbox-light.toml`    | `gruvbox-light`    | Retro warm palette, light variant                    |
| `nord-light.toml`       | `nord-light`       | Arctic palette, light variant                        |
| `solarized-light.toml`  | `solarized-light`  | Ethan Schoonover's light palette; per-column dates † |

### Dark backgrounds

| File                    | Theme name         | Description                                         |
|-------------------------|--------------------|-----------------------------------------------------|
| `catppuccin-mocha.toml` | `catppuccin-mocha` | Warm pastels on dark background                     |
| `dracula.toml`          | `dracula`          | Dark theme with vibrant colours                     |
| `gruvbox-dark.toml`     | `gruvbox-dark`     | Retro warm palette, dark variant                    |
| `nord.toml`             | `nord`             | Arctic, north-bluish palette                        |
| `solarized-dark.toml`   | `solarized-dark`   | Ethan Schoonover's dark palette; per-column dates † |

† **Per-column date styling.**  The two Solarized themes use the
per-timestamp-column theming feature to give each of the four
date columns (modified, accessed, changed, created) its own hue
family for recent files, so that `lx -lll` (which displays all
four columns side by side) reads at a glance:

- **modified** → cyan/blue (the canonical Solarized date colour)
- **accessed** → green/cyan ("reading = growth")
- **changed**  → magenta/violet (metadata shifts)
- **created**  → orange/yellow (origin, warm)

Older files on every column fade to the same shared grey family,
so the differentiation stays where it matters — recent activity.
See `solarized-light.toml` and `solarized-dark.toml` for the
worked example, and the "Timestamp colours" section of
[`docs/GUIDE.md`](../docs/GUIDE.md) for the theme key family.

The other curated themes all use single bulk `date-*` keys that
apply uniformly to every timestamp column — consistent with
their original designs.

### Both backgrounds

| File                  | Theme name        | Description                                 |
|-----------------------|-------------------|---------------------------------------------|
| `the-exa-future.toml` | `the-exa-future`  | Tribute to the original exa, in 24-bit RGB  |

### Builtin overrides

Two drop-ins that override the compiled-in `lx-256` and `lx-24bit`
themes with brighter gradients tuned specifically for dark
backgrounds.  Useful if you find the default builtins too
muted on black:

| File                  | Theme name      | Description                                 |
|-----------------------|-----------------|---------------------------------------------|
| `lx-256-dark.toml`    | `lx-256-dark`   | Dark-tuned variant of the builtin lx-256    |
| `lx-24bit-dark.toml`  | `lx-24bit-dark` | Dark-tuned variant of the builtin lx-24bit  |

## Compiled-in themes

Three themes ship inside the `lx` binary itself — no drop-in needed:

| Theme       | Description                                                    |
|-------------|----------------------------------------------------------------|
| `exa`       | Strict 8-colour ANSI; the heritage exa look                    |
| `lx-256`    | 256-colour, refined and balanced for both light and dark       |
| `lx-24bit`  | 24-bit truecolour, even smoother gradients than `lx-256`       |

The compiled-in `lx` personality auto-selects the best variant for
your terminal: `lx-24bit` if `$COLORTERM` is `truecolor` or `24bit`,
otherwise `lx-256` if `$TERM` matches `*-256color`, otherwise `exa`.

## Creating your own

Start from any of these files or from scratch:

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

Use `lx --dump-theme=exa` to see the compiled-in defaults as a
starting point.  See **lxconfig.toml**(5) for the full list of
theme keys.
