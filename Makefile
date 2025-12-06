# Loctree Build System
# Handles protobuf dependency automatically for rmcp_memex

.PHONY: all build install clean test check fmt help setup-protoc

# Default target
all: build

# Setup vendored protoc path
PROTOC_VENDOR := $(shell cargo run --quiet --package protoc-bin-vendored --example get-path 2>/dev/null || echo "")

# Build all workspace members
build: setup-protoc
	cargo build --workspace --release

# Build only core loctree (no protobuf needed)
build-core:
	cargo build --release -p loctree -p loctree_server -p reports

# Determine cargo bin dir
CARGO_BIN ?= $(if $(CARGO_HOME),$(CARGO_HOME)/bin,$(HOME)/.cargo/bin)

LOCKDIR ?= /tmp/loctree-make.lock.d

# Install loctree binaries (core only - no memex, no protobuf needed)
install:
	@echo "==> Acquiring build lock at $(LOCKDIR)..."
	@if mkdir "$(LOCKDIR)" 2>/dev/null; then \
		trap 'rmdir "$(LOCKDIR)"' EXIT INT TERM; \
		set -e; \
		cargo build --release -p loctree; \
		mkdir -p "$(CARGO_BIN)"; \
		install -m 755 target/release/loctree "$(CARGO_BIN)/loctree"; \
		install -m 755 target/release/loct "$(CARGO_BIN)/loct"; \
		echo "Installed: loct, loctree → $(CARGO_BIN)"; \
	else \
		echo "Another loctree build/install is running (lock: $(LOCKDIR)). Aborting."; \
		exit 1; \
	fi

# Install everything including memex
install-all: setup-protoc
	@echo "==> Acquiring build lock at $(LOCKDIR)..."
	@if mkdir "$(LOCKDIR)" 2>/dev/null; then \
		trap 'rmdir "$(LOCKDIR)"' EXIT INT TERM; \
		set -e; \
		cargo install --path loctree_rs --force; \
		cargo install --path loctree_server --force; \
		cargo install --path rmcp_memex --force; \
		echo "Installed: loct, loctree, loctree-server, rmcp_memex"; \
	else \
		echo "Another loctree build/install is running (lock: $(LOCKDIR)). Aborting."; \
		exit 1; \
	fi

# Setup protoc - check system or use Homebrew
setup-protoc:
	@which protoc > /dev/null 2>&1 || { \
		echo "protoc not found. Installing via Homebrew..."; \
		brew install protobuf; \
	}

# Run tests
test:
	cargo test --workspace

# Check compilation
check:
	cargo check --workspace

# Format code
fmt:
	cargo fmt --all

# Clean build artifacts
clean:
	cargo clean

# Remove stale build lock
unlock:
	@rm -rf "$(LOCKDIR)" && echo "Lock removed" || echo "No lock"

# Help
help:
	@echo "Loctree Build System"
	@echo ""
	@echo "Usage:"
	@echo "  make build        - Build all (installs protobuf if needed)"
	@echo "  make build-core   - Build only loctree (no protobuf needed)"
	@echo "  make install      - Install loct & loctree binaries"
	@echo "  make install-all  - Install all binaries including memex"
	@echo "  make test         - Run all tests"
	@echo "  make clean        - Clean build artifacts"
	@echo ""
	@echo "Quick start:"
	@echo "  make install      - Just works™"
