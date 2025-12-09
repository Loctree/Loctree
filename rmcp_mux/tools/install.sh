#!/usr/bin/env bash
set -euo pipefail
umask 022

# rmcp-mux install script
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Loctree/rmcp-mux/main/tools/install.sh | sh
# Env overrides:
#   INSTALL_DIR   where to place the runnable `rmcp-mux` wrapper (default: $HOME/.local/bin)
#   CARGO_HOME    override cargo home (default: ~/.cargo)
#   MUX_REF       branch/tag/commit to install (default: main)
#   MUX_NO_LOCK   set to 1 to skip --locked

INSTALL_DIR=${INSTALL_DIR:-"$HOME/.local/bin"}
CARGO_HOME=${CARGO_HOME:-"$HOME/.cargo"}
CARGO_BIN="$CARGO_HOME/bin"
REPO_URL="https://github.com/LibraxisAI/rmcp-mux"
# Allow pinning a branch/tag/commit; defaults to main.
MUX_REF=${MUX_REF:-"main"}

info() { printf "[rmcp-mux] %s\n" "$*"; }
warn() { printf "[rmcp-mux][warn] %s\n" "$*" >&2; }

command -v cargo >/dev/null 2>&1 || {
  warn "cargo not found. Install Rust (e.g. https://rustup.rs) then re-run.";
  exit 1;
}

info "Installing rmcp-mux from $REPO_URL (ref: $MUX_REF)"
# --locked keeps dependency resolution reproducible; override with MUX_NO_LOCK=1 if needed.
lock_flag="--locked"
[ "${MUX_NO_LOCK:-0}" = "1" ] && lock_flag=""
# --rev accepts branches, tags, or commits.
cargo install --git "$REPO_URL" --rev "$MUX_REF" $lock_flag --force rmcp-mux >/dev/null

installed_bin="$CARGO_BIN/rmcp-mux"
if [[ ! -x $installed_bin ]]; then
  warn "rmcp-mux binary not found at $installed_bin after install";
  exit 1;
fi

mkdir -p "$INSTALL_DIR"
wrapper="$INSTALL_DIR/rmcp-mux"
cat >"$wrapper" <<WRAP
#!/usr/bin/env bash
exec "$installed_bin" "\$@"
WRAP
chmod +x "$wrapper"

info "Installed binary: $installed_bin"
info "Wrapper: $wrapper"

ensure_path_line() {
  local file="$1"
  local cargo="$CARGO_BIN"
  local install="$INSTALL_DIR"
  local tag="# rmcp-mux installer"

  if [ ! -w "$file" ]; then
    warn "Cannot update PATH in $file (not writable). Add manually: export PATH=\"$cargo:$install:\$PATH\""
    return
  fi

  if grep -q "rmcp-mux installer" "$file"; then
    return
  fi

  printf '\n%s\nexport PATH="%s:%s:$PATH"\n' "$tag" "$cargo" "$install" >>"$file"
  warn "Appended PATH to $file; reload shell or run: source $file"
}

case ":$PATH:" in
  *":$CARGO_BIN:"*) :;;
  *) warn "cargo bin not in PATH; adding to ~/.zshrc"; ensure_path_line "$HOME/.zshrc";;
esac

case ":$PATH:" in
  *":$INSTALL_DIR:"*) :;;
  *) warn "rmcp-mux wrapper dir not in PATH; adding to ~/.zshrc"; ensure_path_line "$HOME/.zshrc";;
esac

info "Done. Try: rmcp-mux --socket /tmp/mcp.sock --cmd npx -- @modelcontextprotocol/server-memory --tray"
