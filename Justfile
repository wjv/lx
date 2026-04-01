# lx project tasks
# Run with: just <recipe>

# List recipes
default:
    @just --list -u

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

# Build debug binary (git only)
build:
    cargo build

# Build debug binary with jj support
build-jj:
    cargo build --features jj

# Build release binary (git only)
release:
    cargo build --release

# Build release binary with jj support
release-jj:
    cargo build --release --features jj

# Run all tests (default features)
test:
    cargo test --workspace -- --quiet

# Run all tests including jj feature
test-jj:
    cargo test --workspace --features jj -- --quiet

# Run all tests both with and without jj
test-all: test test-jj

# Run clippy
lint:
    cargo clippy

# Run clippy on all feature sets
lint-all:
    cargo clippy
    cargo clippy --features jj

# Install lx to ~/.local/bin with man pages (git only)
install: release man
    @mkdir -p ~/.local/bin
    @mkdir -p ~/.local/share/man/man1
    @mkdir -p ~/.local/share/man/man5
    cp target/release/lx ~/.local/bin/lx
    cp man/lx.1 ~/.local/share/man/man1/lx.1
    cp man/lxconfig.toml.5 ~/.local/share/man/man5/lxconfig.toml.5
    @echo "Installed lx to ~/.local/bin/lx"
    @echo "Installed man pages to ~/.local/share/man/"

# Install lx with jj support
install-jj: release-jj man
    @mkdir -p ~/.local/bin
    @mkdir -p ~/.local/share/man/man1
    @mkdir -p ~/.local/share/man/man5
    cp target/release/lx ~/.local/bin/lx
    cp man/lx.1 ~/.local/share/man/man1/lx.1
    cp man/lxconfig.toml.5 ~/.local/share/man/man5/lxconfig.toml.5
    @echo "Installed lx (with jj support) to ~/.local/bin/lx"
    @echo "Installed man pages to ~/.local/share/man/"

# Create personality symlinks in ~/.local/bin
install-personalities: install
    @ln -sf ~/.local/bin/lx ~/.local/bin/ll
    @ln -sf ~/.local/bin/lx ~/.local/bin/la
    @ln -sf ~/.local/bin/lx ~/.local/bin/lll
    @ln -sf ~/.local/bin/lx ~/.local/bin/tree
    @echo "Created personality symlinks in ~/.local/bin: ll la lll tree"

# Create personality symlinks in ~/.local/bin when built with jj support
install-personalities-jj: install-jj
    @ln -sf ~/.local/bin/lx ~/.local/bin/ll
    @ln -sf ~/.local/bin/lx ~/.local/bin/la
    @ln -sf ~/.local/bin/lx ~/.local/bin/lll
    @ln -sf ~/.local/bin/lx ~/.local/bin/tree
    @echo "Created personality symlinks in ~/.local/bin: ll la lll tree"

# Generate shell completions
completions:
    @mkdir -p completions
    cargo run -- --completions bash > completions/lx.bash
    cargo run -- --completions zsh > completions/_lx
    cargo run -- --completions fish > completions/lx.fish
    @echo "Generated completions/"

# Generate default config file
init-config:
    cargo run -- --init-config
