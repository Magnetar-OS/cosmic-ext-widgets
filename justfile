# The conventional COSMIC build entry point, trimmed to what a library has: no
# binary, no desktop entry, so no install/uninstall.

export NAME := 'cosmic-ext-widgets'

default: build-release

build-debug *args:
    cargo build --locked {{args}}

build-release *args: (build-debug '--release' args)

# Pedantic as warnings, not denials — the ecosystem standard.
check *args:
    cargo clippy --all-targets --locked {{args}} -- -W clippy::pedantic

test *args:
    cargo test --locked {{args}}
