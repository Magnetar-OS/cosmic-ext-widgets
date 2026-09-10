# The conventional COSMIC build entry point, trimmed to what a library has: no
# binary, no desktop entry, so no install/uninstall.

export NAME := 'cosmic-ext-widgets'

default: build-release

build-debug *args:
    cargo build --locked {{args}}

build-release *args: (build-debug '--release' args)

# Pedantic is declared in Cargo.toml, so it applies here and in the editor
# alike; this only adds the targets the plain build leaves out.
check *args:
    cargo clippy --all-targets --locked {{args}}

test *args:
    cargo test --locked {{args}}

# The three widths in a real window. See `examples/sidebar.rs`.
run-example *args:
    cargo run --example sidebar --locked {{args}}

# The public documentation, warnings denied as CI denies them.
doc *args:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked {{args}}

# What CI runs, in the order CI runs it.
ci: && check test doc
    cargo fmt --all --check
    cargo build --lib --locked --no-default-features
