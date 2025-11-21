#!/usr/bin/env bash
set -euo pipefail
umask 022

# loctree Node installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_node.sh | sh
# Env overrides:
#   INSTALL_DIR   where to place the runnable `loctree-node` wrapper (default: $HOME/.local/bin)
#   LOCTREE_HOME  where to store the downloaded loctree.mjs (default: $HOME/.local/lib/loctree-node)

INSTALL_DIR=${INSTALL_DIR:-"$HOME/.local/bin"}
LOCTREE_HOME=${LOCTREE_HOME:-"$HOME/.local/lib/loctree-node"}
RAW_URL="https://raw.githubusercontent.com/LibraxisAI/loctree/main/loctree.mjs"

info() { printf "[loctree-node] %s\n" "$*"; }
warn() { printf "[loctree-node][warn] %s\n" "$*" >&2; }

command -v node >/dev/null 2>&1 || { warn "node not found (install Node.js first)"; exit 1; }

mkdir -p "$LOCTREE_HOME" "$INSTALL_DIR"
script_path="$LOCTREE_HOME/loctree.mjs"

info "Downloading loctree.mjs"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$RAW_URL" -o "$script_path"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$RAW_URL" -O "$script_path"
else
  warn "Need curl or wget"; exit 1;
fi

wrapper="$INSTALL_DIR/loctree-node"
cat >"$wrapper" <<WRAP
#!/usr/bin/env bash
set -euo pipefail
exec node "$script_path" "${@:-.}"
WRAP
chmod +x "$wrapper"

info "Installed script: $script_path"
info "Wrapper: $wrapper"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) :;;
  *) warn "Add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\"";;
esac
info "Try: loctree-node . --summary"
