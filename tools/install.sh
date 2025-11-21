#!/usr/bin/env bash
set -euo pipefail
umask 022

# loctree install script
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
# Env overrides:
#   INSTALL_DIR   where to place the runnable `loctree` wrapper (default: $HOME/.local/bin)
#   CARGO_HOME    override cargo home (default: ~/.cargo)

INSTALL_DIR=${INSTALL_DIR:-"$HOME/.local/bin"}
CARGO_HOME=${CARGO_HOME:-"$HOME/.cargo"}
CARGO_BIN="$CARGO_HOME/bin"
REPO_URL="https://github.com/LibraxisAI/loctree"

info() { printf "[loctree] %s\n" "$*"; }
warn() { printf "[loctree][warn] %s\n" "$*" >&2; }

command -v cargo >/dev/null 2>&1 || {
  warn "cargo not found. Install Rust (e.g. https://rustup.rs) then re-run.";
  exit 1;
}

info "Installing loctree from $REPO_URL (cargo install --git)"
# We don't lock here; the project is small and uses minimal deps. Add --locked if you prefer.
cargo install --git "$REPO_URL" --force loctree >/dev/null

installed_bin="$CARGO_BIN/loctree"
if [[ ! -x $installed_bin ]]; then
  warn "loctree binary not found at $installed_bin after install";
  exit 1;
fi

mkdir -p "$INSTALL_DIR"
wrapper="$INSTALL_DIR/loctree"
cat >"$wrapper" <<WRAP
#!/usr/bin/env bash
exec "$installed_bin" "$@"
WRAP
chmod +x "$wrapper"

info "Installed binary: $installed_bin"
info "Wrapper: $wrapper"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) :;;
  *)
    warn "Add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
    if [ -w "${HOME}/.zshrc" ]; then
      warn "Attempting to append to ~/.zshrc"
      printf '\n# loctree installer\nexport PATH="%s:$PATH"\n' "$INSTALL_DIR" >> "$HOME/.zshrc"
      warn "Appended to ~/.zshrc — reload shell or run: source ~/.zshrc"
    fi;;
 esac

info "Done. Try: loctree . --ext rs,ts --summary"
