BINARY := skillctl

.PHONY: build install test fmt lint clean help

build: ## Build binary
	@cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin $(BINARY)

install: ## Install release binary to ~/.local/bin
	@cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin $(BINARY) --release
	@mkdir -p $(HOME)/.local/bin
	@cp rust/target/release/$(BINARY) $(HOME)/.local/bin/$(BINARY)

test: ## Run all tests
	@cargo test --manifest-path rust/Cargo.toml --all

fmt: ## Format Rust source files
	@cargo fmt --manifest-path rust/Cargo.toml --all

lint: ## Run cargo check for all targets
	@cargo check --manifest-path rust/Cargo.toml --all-targets

clean: ## Remove Rust build artifacts
	@rm -rf rust/target

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
	  awk 'BEGIN {FS = ":[^:]*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
