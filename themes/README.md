# `lx` themes

Curated colour themes for `lx`. Each file is a standalone TOML fragment
containing a `[theme.NAME]` and `[style.NAME]` section.

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

| File                    | Theme name         | Description                      |
|-------------------------|--------------------|----------------------------------|
| `catppuccin-mocha.toml` | `catppuccin-mocha` | Warm pastels on dark background  |
| `dracula.toml`          | `dracula`          | Dark theme with vibrant colours  |
| `gruvbox-dark.toml`     | `gruvbox-dark`     | Retro, warm colour palette       |
| `nord.toml`             | `nord`             | Arctic, north-bluish palette     |
| `solarized-dark.toml`   | `solarized-dark`   | Ethan Schoonover's dark palette  |
| `solarized-light.toml`  | `solarized-light`  | Ethan Schoonover's light palette |

All themes inherit from `exa` (the compiled-in default) and override
the UI element colours and file-type style colours.

## Creating your own

Start from any of these files or from scratch:

```toml
[theme.my-theme]
inherits = "exa"       # start from defaults
use-style = "my-theme" # reference the style below
directory = "bold #hexcolour"
date = "#hexcolour"

[style.my-theme]
class.image = "#hexcolour"
class.video = "#hexcolour"
```

Use `lx --dump-style=exa` to see the compiled-in defaults as a
starting point.
