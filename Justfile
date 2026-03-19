# lx project tasks
# Run with: just <recipe>

# Build the man page from markdown source
man:
    pandoc man/lx.1.md -s -t man -o man/lx.1
    @echo "Generated man/lx.1"

# Preview the man page
man-preview: man
    man ./man/lx.1

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

# Install locally (debug)
install:
    cargo install --path .

# Install locally (release)
install-release:
    cargo install --path . --release

# Generate shell completions
completions:
    @mkdir -p completions
    cargo run -- --completions bash > completions/lx.bash
    cargo run -- --completions zsh > completions/_lx
    cargo run -- --completions fish > completions/lx.fish
    @echo "Generated completions/"

# Create personality symlinks in ~/bin
symlinks:
    @mkdir -p ~/bin
    @for name in ll lll la tree ls; do \
        ln -sf $$(which lx) ~/bin/$$name 2>/dev/null || true; \
    done
    @echo "Created symlinks in ~/bin: ll lll la tree ls"

# Generate default config file
init-config:
    cargo run -- --init-config
