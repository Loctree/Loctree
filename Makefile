# Loctree Build System (base edition - core crates only)

.PHONY: all build install build-core clean test check fmt help unlock

# Default target
all: build

# Build all workspace members (loctree_rs, loctree_server, reports)
build:
	cargo build --workspace --release

# Build only the core binaries quickly
build-core:
	cargo build --release -p loctree -p loctree_server -p reports

# Determine cargo bin dir
CARGO_BIN ?= $(if $(CARGO_HOME),$(CARGO_HOME)/bin,$(HOME)/.cargo/bin)
LOCKFILE ?= /tmp/loctree-make.lock

# Install CLI + MCP server
install:
	@if [ -f "$(LOCKFILE)" ]; then \
		old_pid=$$(cat "$(LOCKFILE)" 2>/dev/null); \
		if [ -n "$$old_pid" ] && kill -0 "$$old_pid" 2>/dev/null; then \
			echo "Another build running (PID $$old_pid). Aborting."; \
			exit 1; \
		fi; \
		echo "Removing stale lock (PID $$old_pid dead)"; \
		rm -f "$(LOCKFILE)"; \
	fi
	@echo $$$$ > "$(LOCKFILE)"
	@trap 'rm -f $(LOCKFILE)' EXIT; \
	set -e; \
	cargo install --path loctree_rs --force; \
	cargo install --path loctree_server --force; \
	echo "Installed: loct, loctree, loctree-server → $(CARGO_BIN)"

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test --workspace

# Check compilation
check:
	cargo check --workspace

# Format code
fmt:
	cargo fmt --all

# Remove stale build lock
unlock:
	@rm -f "$(LOCKFILE)" && echo "Lock removed" || echo "No lock"

# Help
help:
	@echo "Loctree Build System (base)"
	@echo ""
	@echo "Usage:"
	@echo "  make build        - Build all workspace crates"
	@echo "  make build-core   - Build loctree binaries only"
	@echo "  make install      - Install loct, loctree, loctree-server"
	@echo "  make test         - Run all tests"
	@echo "  make clean        - Clean build artifacts"
	@echo ""
	@echo "Quick start:"
	@echo "  make install      - Just works™"
