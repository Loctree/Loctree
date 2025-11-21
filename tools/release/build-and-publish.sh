#!/usr/bin/env bash
set -euo pipefail

# Usage: tools/release/build-and-publish.sh <version> <config.json> <github-token>
# Example: tools/release/build-and-publish.sh v0.2.0 tools/release/config-example.json $GITHUB_TOKEN

VERSION=${1:?"version tag (e.g. v0.2.0) required"}
CONFIG=${2:?"config json path required"}
GITHUB_TOKEN=${3:?"GitHub token required"}

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

bin_name=$(jq -r '.binary' "$CONFIG")
artifact_name=$(jq -r '.artifact_name' "$CONFIG")
repo=$(jq -r '.repo' "$CONFIG")
tap=$(jq -r '.tap' "$CONFIG")
targets=$(jq -r '.targets[]' "$CONFIG")

release_dir="$ROOT/tmp/release/$VERSION"
rm -rf "$release_dir"
mkdir -p "$release_dir"

for target in $targets; do
  echo "[build] target=$target"
  cross build --release --target "$target"
  out_dir="target/$target/release"
  cp "$out_dir/$bin_name" "$release_dir/$artifact_name-$target"
  (cd "$release_dir" && tar -czf "$artifact_name-$target.tar.gz" "$artifact_name-$target")
  rm "$release_dir/$artifact_name-$target"
done

export GITHUB_TOKEN

if ! gh release view "$VERSION" -R "$repo" >/dev/null 2>&1; then
  gh release create "$VERSION" -R "$repo" --title "$VERSION" --notes "Automated release $VERSION"
fi

for target in $targets; do
  asset="$release_dir/$artifact_name-$target.tar.gz"
  echo "[upload] $asset"
  gh release upload "$VERSION" "$asset" --clobber -R "$repo"
fi

echo "[tap] Updating Homebrew tap formula"
python3 tools/release/update-tap.py "$VERSION" "$CONFIG"

echo "Done."
