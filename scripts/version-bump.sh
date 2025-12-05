#!/usr/bin/env bash
# Flexible version bump script with scoped targets.
# Usage: ./scripts/version-bump.sh [--patch|--minor|--major] [--all|--loctree|--report|--landing|--memex|--server] [--dev] [--dry-run]
# Defaults: --patch --all (unless --dev with no bump flag → keep version, add -dev)
# Rules:
#   - --all / --loctree update UI occurrences (reports footer, landing easter egg/version) via sync-version
#   - --report / --landing do NOT touch UI occurrences
#   - --loctree does NOT bump Cargo versions for report/landing (but --all does)
#   - Only publishes the loctree crate when loctree is in scope.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

bump_type="patch"
bump_flag_set=false
scope="all"
dev_suffix=false
dry_run=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --patch|--minor|--major)
      bump_type="${1#--}"
      bump_flag_set=true
      shift
      ;;
    --all|--loctree|--report|--landing|--memex|--server)
      scope="${1#--}"
      shift
      ;;
    --dev)
      dev_suffix=true
      shift
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# If --dev is set without an explicit bump flag, keep current numeric version and just add -dev
if $dev_suffix && ! $bump_flag_set; then
  bump_type="none"
fi

include_loctree=false
include_report=false
include_landing=false
include_memex=false
include_server=false
case "$scope" in
  all)
    include_loctree=true
    include_report=true
    include_landing=true
    include_memex=true
    include_server=true
    ;;
  loctree) include_loctree=true ;;
  report) include_report=true ;;
  landing) include_landing=true ;;
  memex) include_memex=true ;;
  server) include_server=true ;;
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
  current="${current%-dev}" # strip existing -dev if present
  if [[ "$kind" == "none" ]]; then
    echo "$current"
    return
  fi
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
memex_ver="$(read_version "$ROOT_DIR/loctree_memex/Cargo.toml")"
server_ver="$(read_version "$ROOT_DIR/loctree_server/Cargo.toml")"

new_loctree_ver="$loctree_ver"
new_report_ver="$report_ver"
new_landing_ver="$landing_ver"
new_memex_ver="$memex_ver"
new_server_ver="$server_ver"

if $include_loctree; then
  new_loctree_ver="$(bump_version "$loctree_ver" "$bump_type")"
fi
if $include_report; then
  new_report_ver="$(bump_version "$report_ver" "$bump_type")"
fi
if $include_landing; then
  new_landing_ver="$(bump_version "$landing_ver" "$bump_type")"
fi
if $include_memex; then
  new_memex_ver="$(bump_version "$memex_ver" "$bump_type")"
fi
if $include_server; then
  new_server_ver="$(bump_version "$server_ver" "$bump_type")"
fi

echo "Bump type: $bump_type"
echo "Scope: $scope"
echo "Versions -> loctree: $new_loctree_ver | report: $new_report_ver | landing: $new_landing_ver | memex: $new_memex_ver | server: $new_server_ver"
if $dev_suffix; then
  new_loctree_ver="${new_loctree_ver%-dev}-dev"
  new_report_ver="${new_report_ver%-dev}-dev"
  new_landing_ver="${new_landing_ver%-dev}-dev"
  new_memex_ver="${new_memex_ver%-dev}-dev"
  new_server_ver="${new_server_ver%-dev}-dev"
  echo "Applying -dev suffix"
fi
if $dry_run; then
  echo "Dry-run: will skip publish/commit"
fi

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

# Update memex Cargo
if $include_memex; then
  update_sed "$ROOT_DIR/loctree_memex/Cargo.toml" 's/^version = ".*"/version = "'$new_memex_ver'"/'
fi

# Update server Cargo
if $include_server; then
  update_sed "$ROOT_DIR/loctree_server/Cargo.toml" 's/^version = ".*"/version = "'$new_server_ver'"/'
fi

echo "==> Formatting"
$include_loctree && cargo fmt --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
$include_report && cargo fmt --manifest-path "$ROOT_DIR/reports/Cargo.toml"
$include_landing && cargo fmt --manifest-path "$ROOT_DIR/landing/Cargo.toml"
$include_memex && cargo fmt --manifest-path "$ROOT_DIR/loctree_memex/Cargo.toml"
$include_server && cargo fmt --manifest-path "$ROOT_DIR/loctree_server/Cargo.toml"

echo "==> Clippy"
$include_loctree && cargo clippy --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --all-targets -- -D warnings
$include_report && cargo clippy --manifest-path "$ROOT_DIR/reports/Cargo.toml" --all-targets -- -D warnings
$include_landing && cargo clippy --manifest-path "$ROOT_DIR/landing/Cargo.toml" --all-targets -- -D warnings
$include_memex && cargo clippy --manifest-path "$ROOT_DIR/loctree_memex/Cargo.toml" --all-targets -- -D warnings
$include_server && cargo clippy --manifest-path "$ROOT_DIR/loctree_server/Cargo.toml" --all-targets -- -D warnings

echo "==> Tests/Build"
$include_loctree && cargo test --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml"
$include_report && cargo test --manifest-path "$ROOT_DIR/reports/Cargo.toml"
$include_landing && cargo test --manifest-path "$ROOT_DIR/landing/Cargo.toml"
$include_memex && cargo test --manifest-path "$ROOT_DIR/loctree_memex/Cargo.toml"
$include_server && cargo build --manifest-path "$ROOT_DIR/loctree_server/Cargo.toml"

if $include_loctree; then
  echo "==> Build release (loctree_rs)"
  cargo build --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --release

  if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    echo "CARGO_REGISTRY_TOKEN not set; skipping publish" >&2
  else
  echo "==> Publish crate loctree v$new_loctree_ver"
    if $dry_run; then
      echo "Dry-run: skipping publish"
    else
      cargo publish --manifest-path "$ROOT_DIR/loctree_rs/Cargo.toml" --locked
    fi
  fi
fi

if $dry_run; then
  echo "==> Dry-run: skipping git commit"
else
  echo "==> Git commit (no push)"
  git -C "$ROOT_DIR" add -A
  git -C "$ROOT_DIR" commit -m "Bump versions: loctree=$new_loctree_ver report=$new_report_ver landing=$new_landing_ver memex=$new_memex_ver server=$new_server_ver"
fi

# Generate changelog entry from conventional commits
generate_changelog_entry() {
  local version="$1"
  local date=$(date +%Y-%m-%d)
  local last_tag=$(git -C "$ROOT_DIR" describe --tags --abbrev=0 2>/dev/null || echo "")
  local range="${last_tag:+$last_tag..HEAD}"

  echo "## [$version] - $date"
  echo ""

  local added=""
  local changed=""
  local fixed=""
  local removed=""

  # Parse conventional commits
  while IFS= read -r commit; do
    [[ -z "$commit" ]] && continue
    local subject="${commit#* }"

    case "$subject" in
      feat:*|feat\(*\):*)
        local msg="${subject#feat:}"
        msg="${msg#feat(*):}"
        msg="${msg# }"
        added+="- ${msg}\n"
        ;;
      fix:*|fix\(*\):*)
        local msg="${subject#fix:}"
        msg="${msg#fix(*):}"
        msg="${msg# }"
        fixed+="- ${msg}\n"
        ;;
      refactor:*|perf:*|chore:*)
        local msg="${subject#*:}"
        msg="${msg# }"
        changed+="- ${msg}\n"
        ;;
      *BREAKING*|*breaking*)
        changed+="- **BREAKING**: ${subject}\n"
        ;;
    esac
  done < <(git -C "$ROOT_DIR" log --oneline $range 2>/dev/null)

  [[ -n "$added" ]] && echo "### Added" && echo -e "$added"
  [[ -n "$changed" ]] && echo "### Changed" && echo -e "$changed"
  [[ -n "$fixed" ]] && echo "### Fixed" && echo -e "$fixed"
  [[ -n "$removed" ]] && echo "### Removed" && echo -e "$removed"
}

# Insert changelog entry after ## [Released] line
if $include_loctree && [[ -f "$ROOT_DIR/CHANGELOG.md" ]]; then
  echo "==> Generating changelog entry"
  changelog_entry=$(generate_changelog_entry "$new_loctree_ver")

  if [[ -n "$changelog_entry" ]]; then
    # Create temp file with new entry inserted after ## [Released]
    awk -v entry="$changelog_entry" '
      /^## \[Released\]/ {
        print
        print ""
        print entry
        next
      }
      { print }
    ' "$ROOT_DIR/CHANGELOG.md" > "$ROOT_DIR/CHANGELOG.md.tmp"
    mv "$ROOT_DIR/CHANGELOG.md.tmp" "$ROOT_DIR/CHANGELOG.md"
    echo "  Updated: CHANGELOG.md"
  else
    echo "  No conventional commits found since last tag"
  fi
fi

echo ""
echo "Done. Remember to push and tag if desired:"
echo "  git push origin HEAD"
if $include_loctree; then
  echo "  git tag v$new_loctree_ver && git push origin v$new_loctree_ver"
fi
