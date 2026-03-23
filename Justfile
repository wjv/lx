# lx project tasks
# Run with: just <recipe>

# Build man pages from markdown source
man:
    pandoc man/lx.1.md -s -t man -o man/lx.1
    pandoc man/lxconfig.toml.5.md -s -t man -o man/lxconfig.toml.5
    @echo "Generated man/lx.1 and man/lxconfig.toml.5"

# Preview the lx(1) man page
man-preview: man
    man ./man/lx.1

# Preview the lxconfig.toml(5) man page
man-config-preview: man
    man ./man/lxconfig.toml.5

# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Run all tests
test:
    cargo test --workspace -- --quiet

# Run clippy
lint:
    cargo clippy

# Install locally
install:
    cargo install --path .

# Generate shell completions
completions:
    @mkdir -p completions
    cargo run -- --completions bash > completions/lx.bash
    cargo run -- --completions zsh > completions/_lx
    cargo run -- --completions fish > completions/lx.fish
    @echo "Generated completions/"

# Create personality symlinks in ~/.local/bin
symlinks:
    @mkdir -p ~/local/bin
    @for name in ll lll tree ls; do \
        ln -sf $$(which lx) ~/.local/bin/$$name 2>/dev/null || true; \
    done
    @echo "Created symlinks in ~/.local/bin: ll lll tree ls"

# Generate default config file
init-config:
    cargo run -- --init-config
