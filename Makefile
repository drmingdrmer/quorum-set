all: test lint doc

test:
	cargo test

build:
	cargo build --release

check:
	RUSTFLAGS="-D warnings" cargo check

lint: fmt clippy

fmt:
	cargo fmt

clippy:
	cargo clippy --no-deps --all-targets -- -D warnings

coverage:
	cargo llvm-cov --html --output-dir target/coverage
	cargo llvm-cov report --summary-only --json --output-path target/coverage/summary.json

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --all --no-deps

clean:
	cargo clean

.PHONY: all test build check lint fmt clippy coverage doc clean
