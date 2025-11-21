#!/usr/bin/env bash
set -euo pipefail
umask 022

# loctree Python installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_py.sh | sh
# Env overrides:
#   INSTALL_DIR   where to place the runnable `loctree-py` wrapper (default: $HOME/.local/bin)
#   LOCTREE_HOME  where to store the downloaded loctree.py (default: $HOME/.local/lib/loctree-py)

INSTALL_DIR=${INSTALL_DIR:-"$HOME/.local/bin"}
LOCTREE_HOME=${LOCTREE_HOME:-"$HOME/.local/lib/loctree-py"}
RAW_URL="https://raw.githubusercontent.com/LibraxisAI/loctree/main/loctree.py"

info() { printf "[loctree-py] %s\n" "$*"; }
warn() { printf "[loctree-py][warn] %s\n" "$*" >&2; }

command -v python3 >/dev/null 2>&1 || { warn "python3 not found"; exit 1; }

mkdir -p "$LOCTREE_HOME" "$INSTALL_DIR"
script_path="$LOCTREE_HOME/loctree.py"

info "Downloading loctree.py"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$RAW_URL" -o "$script_path"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$RAW_URL" -O "$script_path"
else
  warn "Need curl or wget"; exit 1;
fi

wrapper="$INSTALL_DIR/loctree-py"
cat >"$wrapper" <<WRAP
#!/usr/bin/env bash
set -euo pipefail
exec python3 "$script_path" "$@"
WRAP
chmod +x "$wrapper"

info "Installed script: $script_path"
info "Wrapper: $wrapper"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) :;;
  *) warn "Add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\"";;
esac
info "Try: loctree-py . --summary"
