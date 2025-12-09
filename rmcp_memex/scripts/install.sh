#!/bin/bash
set -euo pipefail

echo "=== rmcp_memex setup ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

echo "Building release binary..."
cargo build --release

if [[ "${1:-}" == "--bundle-macos" ]]; then
  echo "Creating macOS app bundle..."
  "$SCRIPT_DIR/build-macos.sh"
  BIN_PATH="$HOME/.rmcp_servers/MCPServer.app/Contents/MacOS/rmcp_memex"
else
  BIN_PATH="$REPO_ROOT/target/release/rmcp_memex"
fi

echo ""
echo "=== Done ==="
echo "Binary: $BIN_PATH"
echo ""
echo "Example MCP host config:"
cat <<JSON
{
  "mcpServers": {
    "rmcp_memex": {
      "command": "$BIN_PATH",
      "args": ["--log-level", "info"]
    }
  }
}
JSON

echo ""
echo "Notes:"
echo "- The MLX HTTP bridge is optional. By default, DRAGON_BASE_URL=http://localhost."
echo "- To force local-only embeddings, set DISABLE_MLX=1."
