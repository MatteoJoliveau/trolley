default:
    @just --list --unsorted | grep -v '  default'

test *args:
    cargo nextest run --all-targets {{ args }}

example name:
    cargo run --example {{ name }} --all-features

fmt:
    cargo fmt

clippy:
    cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets --workspace
