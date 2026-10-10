.PHONY: build test clean clippy fmt check ci

# soroban-sdk v28 refuses a plain `cargo build`; the contract must be built
# through the Stellar CLI, which optimizes the wasm and writes the wasm hash.
# Requires stellar-cli v25.2.0+ (see docs/DEPLOYMENT.md).
build:
	stellar contract build

test:
	cargo test

clean:
	cargo clean

clippy:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo check --all-targets
ci: fmt clippy test check
