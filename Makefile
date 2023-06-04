.DEFAULT_GOAL := all

.PHONY: build-dev
build-dev:
	cargo build

.PHONY: build-prod
build-prod:
	cargo build --release

.PHONY: format
format:
	cargo fmt

.PHONY: lint
lint:
	@cargo fmt --version
	cargo fmt --all -- --check
	@cargo clippy --version
	cargo clippy --tests -- -D warnings -A incomplete_features -W clippy::dbg_macro

.PHONY: test
test:
	cargo test

.PHONY: all
all: format lint test
