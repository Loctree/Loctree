#!/usr/bin/env bash
# Flexible version bump script with scoped targets.
# Usage: ./scripts/version-bump.sh [--patch|--minor|--major] [--all|--loctree|--report|--landing]
# Defaults: --patch --all
# Rules:
#   - --all / --loctree update UI occurrences (reports footer, landing easter egg/version) via sync-version
#   - --report / --landing do NOT touch UI occurrences
#   - --loctree does NOT bump Cargo versions for report/landing (but --all does)
#   - Only publishes the loctree crate when loctree is in scope.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

bump_type="patch"
scope="all"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --patch|--minor|--major)
      bump_type="${1#--}"
      shift
      ;;
    --all|--loctree|--report|--landing)
      scope="${1#--}"
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

include_loctree=false
include_report=false
include_landing=false
case "$scope" in
  all)
    include_loctree=true
    include_report=true
    include_landing=true
    ;;
  loctree) include_loctree=true ;;
  report) include_report=true ;;
  landing) include_landing=true ;;
esac

if [[ ! -f "$ROOT_DIR/loctree_rs/Cargo.toml" ]]; then
  echo "Run this script from the repository root." >&2
  exit 1
fi

# Require clean tree
if ! git -C "$ROOT_DIR" diff --quiet || ! git -C "$ROOT_DIR" diff --cached --quiet; then
  echo "Working tree is dirty. Commit or stash changes first." >&2
  exit 1
fi

bump_version() {
  local current="$1" kind="$2"
  IFS='.' read -r major minor patch <<<"$current"
  case "$kind" in
    patch) patch=$((patch + 1)) ;;
    minor) minor=$((minor + 1)); patch=0 ;;
    major) major=$((major + 1)); minor=0; patch=0 ;;
  esac
  echo "${major}.${minor}.${patch}"
}

read_version() {
  grep '^version = ' "$1" | head -1 | cut -d'"' -f2
}

update_sed() {
  local file="$1" pattern="$2"
  if [ -f "$file" ]; then
    if sed --version 2>/dev/null | grep -q GNU; then
      sed -i "$pattern" "$file"
    else
      sed -i '' "$pattern" "$file"
    fi
    echo "  Updated: $file"
  fi
}

loctree_ver="$(read_version "$ROOT_DIR/loctree_rs/Cargo.toml")"
report_ver="$(read_version "$ROOT_DIR/reports/Cargo.toml")"
landing_ver="$(read_version "$ROOT_DIR/landing/Cargo.toml")"

new_loctree_ver="$loctree_ver"
new_report_ver="$report_ver"
new_landing_ver="$landing_ver"

if $include_loctree; then
  new_loctree_ver="$(bump_version "$loctree_ver" "$bump_type")"
fi
if $include_report; then
  new_report_ver="$(bump_version "$report_ver" "$bump_type")"
fi
if $include_landing; then
  new_landing_ver="$(bump_version "$landing_ver" "$bump_type")"
fi

echo "Bump type: $bump_type"
echo "Scope: $scope"
echo "Versions -> loctree: $new_loctree_ver | report: $new_report_ver | landing: $new_landing_ver"

# Update loctree + UI (only when loctree/all)
if $include_loctree; then
  "$ROOT_DIR/scripts/sync-version.sh" "$new_loctree_ver"
fi

# Update report Cargo (no UI)
if $include_report; then
  update_sed "$ROOT_DIR/reports/Cargo.toml" 's/^version = ".*"/version = "'$new_report_ver'"/'
fi

# Update landing Cargo (no UI)
if $include_landing; then
  update_sed "$ROOT_DIR/landing/Cargo.toml" 's/^version = ".*"/version = "'$new_landing_ver'"/'
fi

echo "==> Formatting"
$include_loctree && cargo fmt --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
$include_report && cargo fmt --manifest-path "$ROOT_DIR/reports/Cargo.toml"
$include_landing && cargo fmt --manifest-path "$ROOT_DIR/landing/Cargo.toml"

echo "==> Clippy"
$include_loctree && cargo clippy --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --all-targets -- -D warnings
$include_report && cargo clippy --manifest-path "$ROOT_DIR/reports/Cargo.toml" --all-targets -- -D warnings
$include_landing && cargo clippy --manifest-path "$ROOT_DIR/landing/Cargo.toml" --all-targets -- -D warnings

echo "==> Tests"
$include_loctree && cargo test --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
$include_report && cargo test --manifest-path "$ROOT_DIR/reports/Cargo.toml"
$include_landing && cargo test --manifest-path "$ROOT_DIR/landing/Cargo.toml"

if $include_loctree; then
  echo "==> Build release (loctree_rs)"
  cargo build --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --release

  if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    echo "CARGO_REGISTRY_TOKEN not set; skipping publish" >&2
  else
    echo "==> Publish crate loctree v$new_loctree_ver"
    cargo publish --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --locked
  fi
fi

echo "==> Git commit (no push)"
git -C "$ROOT_DIR" add -A
git -C "$ROOT_DIR" commit -m "Bump versions: loctree=$new_loctree_ver report=$new_report_ver landing=$new_landing_ver"

echo ""
echo "Done. Remember to push and tag if desired:"
echo "  git push origin HEAD"
if $include_loctree; then
  echo "  git tag v$new_loctree_ver && git push origin v$new_loctree_ver"
fi
