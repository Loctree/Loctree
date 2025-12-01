#!/usr/bin/env bash
# Bump version across the repo, run checks, build, publish crate, and commit (no push).
# Usage: ./scripts/version-bump.sh <new-version>
# Requirements: clean git tree, CARGO_REGISTRY_TOKEN set, Rust toolchain, bash.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <new-version>"
  exit 1
fi

if [[ ! -f "$ROOT_DIR/loctree_rs/Cargo.toml" ]]; then
  echo "Run this script from the repository root (loctree_rs/Cargo.toml not found)." >&2
  exit 1
fi

# Require clean tree before modifying
if ! git -C "$ROOT_DIR" diff --quiet || ! git -C "$ROOT_DIR" diff --cached --quiet; then
  echo "Working tree is dirty. Commit or stash changes before bumping version." >&2
  exit 1
fi

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "CARGO_REGISTRY_TOKEN is not set. Set it before publishing." >&2
  exit 1
fi

echo "==> Bumping version to $VERSION"

"$ROOT_DIR/scripts/sync-version.sh" "$VERSION"

echo "==> Formatting"
cargo fmt --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
cargo fmt --manifest-path "$ROOT_DIR/reports/Cargo.toml"
cargo fmt --manifest-path "$ROOT_DIR/landing/Cargo.toml"

echo "==> Clippy"
cargo clippy --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --all-targets -- -D warnings
cargo clippy --manifest-path "$ROOT_DIR/reports/Cargo.toml" --all-targets -- -D warnings
cargo clippy --manifest-path "$ROOT_DIR/landing/Cargo.toml" --all-targets -- -D warnings

echo "==> Tests"
cargo test --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
cargo test --manifest-path "$ROOT_DIR/reports/Cargo.toml"
cargo test --manifest-path "$ROOT_DIR/landing/Cargo.toml"

echo "==> Build release (loctree_rs)"
cargo build --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --release

echo "==> Publish crate loctree v$VERSION"
cargo publish --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --locked

echo "==> Git commit (no push)"
git -C "$ROOT_DIR" add -A
git -C "$ROOT_DIR" commit -m "Bump version to $VERSION"

echo ""
echo "Done. Remember to push and tag if desired:"
echo "  git push origin HEAD"
echo "  git tag v$VERSION && git push origin v$VERSION"
