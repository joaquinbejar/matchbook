# Common tasks. Run `make` for the full check.
SOLANA_BIN ?=
export PATH := $(if $(SOLANA_BIN),$(SOLANA_BIN):,)$(PATH)

.PHONY: all
all: fmt-check lint test

.PHONY: program
program:
	cargo build-sbf --manifest-path program/Cargo.toml

.PHONY: test
test: program
	cargo test --workspace --all-features
	cd ts-sdk && npm test

.PHONY: model
model:
	MODEL_CASES=20000 cargo test -p matchbook-engine --test model --release

.PHONY: bench
bench: program
	cargo test -p matchbook-program --test bench -- --nocapture

.PHONY: lint
lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cd ts-sdk && npm run lint && npm run typecheck

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: fmt-check
fmt-check:
	cargo fmt --all --check

.PHONY: playground
playground:
	./playground/build.sh

.PHONY: deploy-devnet
deploy-devnet:
	./scripts/deploy.sh devnet
