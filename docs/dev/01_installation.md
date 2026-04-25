# Loctree - Installation Guide

Complete installation guide for the public Loctree OSS workspace.

## Quick Start

```bash
# Fastest public install path: CLI + MCP server
curl -fsSL https://loct.io/install.sh | sh

# Cargo alternative (reproducible lockfile build)
cargo install --locked loctree loctree-mcp

# Or from source
git clone https://github.com/Loctree/loctree-ast.git
cd loctree-ast
make install
```

Public install channels follow the latest published release, which can lag
behind the workspace version on this branch. Verify the exact published version
on crates.io, npm, or GitHub Releases when you need release-exact behavior.

## What Gets Installed

| Binary | Crate | Description |
|--------|-------|-------------|
| `loct` | loctree | Primary CLI - fast, agent-optimized |
| `loctree` | loctree | Compatibility alias for `loct` |
| `loctree-mcp` | loctree-mcp | MCP server for AI agents (Claude, Cursor, etc.) |

## Installation Methods

### 1. One-Liner Installer

```bash
curl -fsSL https://loct.io/install.sh | sh
```

The installer defaults to `loctree + loctree-mcp`. Set `INSTALL_MCP=0` only if
you explicitly want CLI-only.

### 2. Cargo (Recommended)

```bash
cargo install --locked loctree loctree-mcp
```

### 3. Homebrew (macOS/Linux)

```bash
brew install loctree/cli/loct
brew install loctree/mcp/loctree-mcp
```

Use one global channel per machine. If you already installed Loctree globally
via Homebrew, avoid `npm install -g loctree` on the same setup unless you first
remove the existing global binaries.

### 4. npm (CLI only)

```bash
npm install -g loctree
```

Supported npm targets are defined by the latest published npm release. Alpine
/musl should use Cargo or direct release assets instead.

This installs the CLI only. Install `loctree-mcp` separately via Cargo,
Homebrew, or GitHub Releases if your workflow needs MCP.

### 5. Direct GitHub Release Assets

The monorepo release page is the public fallback for installable CLI and MCP
tarballs. Thin release repos are part of the release choreography and may lag
while assets are being mirrored:

- CLI: `Loctree/loct`
- MCP: `Loctree/loctree-mcp`

### 6. From Source

```bash
git clone https://github.com/Loctree/loctree-ast.git
cd loctree-ast
make install
```

## Clean-Room macOS Note

The public release binaries use vendored `libgit2`, so macOS release artifacts
do not depend on Homebrew runtime paths such as `/opt/homebrew/opt/libgit2/...`.

For a reproducible local verification on Apple Silicon:

```bash
make smoke-release-macos-arm64
```

## Workspace Structure

```text
Loctree/
├── loctree_rs/          # Core library + CLI (loct, loctree)
├── loctree-mcp/         # MCP server for loctree
├── rmcp-common/         # Shared MCP/common utilities
├── reports/             # Leptos-based HTML reports
└── distribution/        # Release/install channels and packaging docs
```

## Configuration

### MCP Server Setup (Claude Code / Cursor)

Add to your MCP config (`~/.config/claude/claude_desktop_config.json` or similar):

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

## Verification

```bash
# Check versions
loct --version
loctree --version
loctree-mcp --version

# Test loctree on current directory
loct

# Test MCP server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | loctree-mcp
```

## Makefile Targets

```bash
make install        # Install loct, loctree, loctree-mcp
make install-cli    # Install only loct + loctree
make install-mcp    # Install only loctree-mcp
make build          # Build all crates (release)
make build-core     # Build only core
make precheck       # Fast repo-wide gate
make test           # Run all workspace tests
make check          # Run the full quality gate
make fmt            # Format code
make clean          # Clean build artifacts
make mcp-build      # Build loctree-mcp
make smoke-release-macos-arm64  # Verify macOS arm64 release portability
```

## Troubleshooting

### Build Lock Conflict

```text
Another build running (PID xxxx). Aborting.
```

Solution:

```bash
make unlock
```

### Cargo Install Conflicts

If you have both crates.io and local versions:

```bash
cargo uninstall loctree loctree-mcp
make install
```

## Platform Support

| Platform | CLI | MCP | Notes |
|----------|-----|-----|-------|
| macOS (Apple Silicon) | Full | Full | Primary releaseability target |
| macOS (Intel) | Full | Full | Built in release workflow |
| Linux (x86_64 glibc) | Full | Full | Built in release workflow |
| Windows (x86_64) | Full | Full | Built in release workflow |

## Updating

```bash
# From crates.io
cargo install --locked loctree loctree-mcp --force

# From source
git pull
make install
```

## Uninstalling

```bash
# Cargo-installed binaries
cargo uninstall loctree loctree-mcp

# Or manually
rm ~/.cargo/bin/loct
rm ~/.cargo/bin/loctree
rm ~/.cargo/bin/loctree-mcp
```

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Loctree Team
