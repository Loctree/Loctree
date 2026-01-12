# Loctree Build System
# Includes comprehensive MCP server management
#
# Created by M&K (c)2025 The LibraxisAI Team

.PHONY: all build install clean test check precheck fmt help setup-protoc
.PHONY: version version-show version-check
.PHONY: mcp-build mcp-install mcp-test
.PHONY: ai-hooks ai-hooks-claude ai-hooks-codex ai-hooks-gemini git-hooks

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

LOCKFILE ?= /tmp/loctree-make.lock

# Install loctree CLI + MCP server
# Lock is auto-cleaned on success, failure, or if stale (dead PID)
install: setup-protoc
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
	cargo install --path loctree-mcp --force; \
	echo "Installed: loct, loctree, loctree-mcp → $(CARGO_BIN)"; \
	$(MAKE) git-hooks

# Alias for backwards compatibility
install-all: install

# Setup protoc - check system or use Homebrew
setup-protoc:
	@which protoc > /dev/null 2>&1 || { \
		echo "protoc not found. Installing via Homebrew..."; \
		brew install protobuf; \
	}

# Run tests
test:
	cargo test --workspace

# Quick check (compilation only)
check:
	cargo check --workspace

# Full pre-push validation (fmt + clippy + check) - FAST, run before build!
# This catches 90% of issues in seconds instead of waiting for 20min build
precheck:
	@echo "=== Pre-push Check ==="
	@echo "[1/3] Checking formatting..."
	@cargo fmt --all --check || (echo "Run 'make fmt' to fix" && exit 1)
	@echo "[2/3] Running clippy..."
	@cargo clippy --workspace --all-targets -- -D warnings
	@echo "[3/3] Type checking..."
	@cargo check --workspace
	@echo "=== All checks passed ==="

# Format code
fmt:
	cargo fmt --all

# Clean build artifacts
clean:
	cargo clean

# Remove stale build lock
unlock:
	@rm -f "$(LOCKFILE)" && echo "Lock removed" || echo "No lock"

# Help
help:
	@echo "Loctree Build System"
	@echo ""
	@echo "Core Commands:"
	@echo "  make precheck     - Pre-push validation (fmt+clippy+check) - RUN FIRST!"
	@echo "  make build        - Build all (installs protobuf if needed)"
	@echo "  make build-core   - Build only loctree (no protobuf needed)"
	@echo "  make install      - Install loct, loctree & loctree-mcp"
	@echo "  make test         - Run all tests"
	@echo "  make check        - Quick type check (no clippy)"
	@echo "  make fmt          - Format all code"
	@echo "  make clean        - Clean build artifacts"
	@echo ""
	@echo "Version Management:"
	@echo "  make version-show       - Show all crate versions"
	@echo "  make version-check      - Check publish readiness (dry-run)"
	@echo "  make version SCOPE=X TYPE=Y  - Bump version"
	@echo "    SCOPE: loctree, report, mcp, lsp, all (default: all)"
	@echo "    TYPE:  patch (default), minor, major"
	@echo "    TAG=1, PUSH=1, FORCE=1, PUBLISH=1 - Additional options"
	@echo "  Examples:"
	@echo "    make version                       - Bump all crates (patch)"
	@echo "    make version SCOPE=loctree         - Bump loctree only"
	@echo "    make version SCOPE=mcp TYPE=minor  - Minor bump loctree-mcp"
	@echo ""
	@echo "MCP Build & Install:"
	@echo "  make mcp-build         - Build loctree-mcp"
	@echo "  make mcp-install       - Install loctree-mcp"
	@echo "  make mcp-test          - Test loctree-mcp via stdio"
	@echo ""
	@echo "AI CLI Integration:"
	@echo "  make git-hooks         - Install git pre-push validation hook"
	@echo "  make ai-hooks          - Interactive hook installer (Claude/Codex/Gemini)"
	@echo "  make ai-hooks-claude   - Install Claude Code hooks"
	@echo ""
	@echo "Quick start:"
	@echo "  make install           - Install loct + loctree-mcp"

# ============================================================================
# Version Management
# ============================================================================

VERSION_SCRIPT := ./scripts/version-bump.sh

# Default values (override via make version SCOPE=mcp TYPE=minor)
SCOPE ?= all
TYPE ?= patch

# Show all crate versions and dependency graph
version-show:
	@$(VERSION_SCRIPT) --show-deps

# Check publish readiness (dry-run)
# Usage: make version-check SCOPE=mcp
version-check:
	@$(VERSION_SCRIPT) --dry-run --$(SCOPE) --$(TYPE)

# Bump version
# Usage: make version SCOPE=loctree TYPE=minor
#        make version SCOPE=mcp TYPE=patch TAG=1 PUSH=1
# Options: SCOPE (all|loctree|mcp|report|lsp)
#          TYPE  (patch|minor|major)
#          TAG   (1 to create git tag)
#          PUSH  (1 to push to remote)
#          FORCE (1 to skip dirty tree check)
#          PUBLISH (1 to publish to crates.io, default: skip)
version:
	@$(VERSION_SCRIPT) --$(SCOPE) --$(TYPE) $(if $(TAG),--tag) $(if $(PUSH),--push) $(if $(FORCE),--force) $(if $(PUBLISH),,--no-publish)

# ============================================================================
# MCP Build & Install (loctree-mcp only)
# ============================================================================

# Build loctree-mcp
mcp-build:
	@echo "Building loctree-mcp..."
	cargo build --release -p loctree-mcp
	@echo "Done. Binary in target/release/"

# Install loctree-mcp (alias - use 'make install' instead)
mcp-install:
	cargo install --path loctree-mcp --force
	@echo "Installed: loctree-mcp → $(CARGO_BIN)"

# Test loctree-mcp via stdio
mcp-test:
	@echo "Testing loctree-mcp..."
	@echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"make-test","version":"1.0"}}}' \
		| $(CARGO_BIN)/loctree-mcp 2>/dev/null | head -1 || echo "Test failed"

# ============================================================================
# AI Hooks Installation (Claude, Codex, Gemini)
# ============================================================================

AI_HOOKS_SCRIPT := ./scripts/install-ai-hooks.sh

# Interactive installation for all detected CLIs
ai-hooks:
	@chmod +x $(AI_HOOKS_SCRIPT)
	@$(AI_HOOKS_SCRIPT)

# Install for specific CLIs (non-interactive)
ai-hooks-claude:
	@chmod +x $(AI_HOOKS_SCRIPT)
	@CLI=claude $(AI_HOOKS_SCRIPT)

ai-hooks-codex:
	@chmod +x $(AI_HOOKS_SCRIPT)
	@CLI=codex $(AI_HOOKS_SCRIPT)

ai-hooks-gemini:
	@chmod +x $(AI_HOOKS_SCRIPT)
	@CLI=gemini $(AI_HOOKS_SCRIPT)

# Install all detected CLIs (non-interactive)
ai-hooks-all:
	@chmod +x $(AI_HOOKS_SCRIPT)
	@CLI=all $(AI_HOOKS_SCRIPT)

# ============================================================================
# Git Hooks Installation
# ============================================================================

# Install git hooks (pre-commit fmt + pre-push validation)
git-hooks:
	@echo "Installing git hooks..."
	@ln -sf ../../tools/hooks/pre-commit .git/hooks/pre-commit
	@ln -sf ../../tools/hooks/pre-push .git/hooks/pre-push
	@chmod +x tools/hooks/pre-commit tools/hooks/pre-push
	@echo "✓ pre-commit + pre-push hooks installed"
