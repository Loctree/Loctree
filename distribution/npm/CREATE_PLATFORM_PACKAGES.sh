#!/bin/bash
# Script to create all platform-specific package directories

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKSPACE_TOML="$ROOT_DIR/Cargo.toml"

# Keep npm package version aligned with workspace.package.version
VERSION="$(
  awk '
    /^\[workspace\.package\]/ { in_ws=1; next }
    in_ws && /^\[/ { in_ws=0 }
    in_ws && /^version[[:space:]]*=/ {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$WORKSPACE_TOML"
)"

if [ -z "$VERSION" ]; then
  echo "Failed to resolve workspace version from $WORKSPACE_TOML" >&2
  exit 1
fi

PLATFORMS=(
  "darwin-arm64:macOS Apple Silicon (ARM64):darwin:arm64"
  "darwin-x64:macOS Intel (x64):darwin:x64"
  "linux-arm64-gnu:Linux ARM64 (glibc):linux:arm64"
  "linux-arm64-musl:Linux ARM64 (musl/Alpine):linux:arm64"
  "linux-x64-gnu:Linux x64 (glibc):linux:x64"
  "linux-x64-musl:Linux x64 (musl/Alpine):linux:x64"
  "win32-arm64-msvc:Windows ARM64:win32:arm64"
  "win32-x64-msvc:Windows x64:win32:x64"
)

for platform_spec in "${PLATFORMS[@]}"; do
  IFS=: read -r platform desc os cpu <<< "$platform_spec"
  
  dir="platform-packages/$platform"
  mkdir -p "$dir"
  
  cat > "$dir/package.json" << PACKAGE_EOF
{
  "name": "@loctree/$platform",
  "version": "$VERSION",
  "description": "loctree binary for $desc",
  "keywords": ["loctree", "$os", "$cpu"],
  "license": "(MIT OR Apache-2.0)",
  "os": ["$os"],
  "cpu": ["$cpu"],
  "repository": {
    "type": "git",
    "url": "https://github.com/Loctree/Loctree.git"
  },
  "files": [
    "loctree$([ "$os" = "win32" ] && echo ".exe" || echo "")"
  ],
  "scripts": {
    "postinstall": "node postinstall.js"
  }
}
PACKAGE_EOF

  # Copy postinstall script
  cp platform-packages/postinstall.js "$dir/"
  
  echo "Created $dir"
done

echo ""
echo "All platform-specific packages created!"
echo ""
echo "Next steps:"
echo "1. Run this script to create all package directories"
echo "2. For each platform package, run 'npm publish --access public'"
echo "3. Then publish the main package with 'npm publish'"
