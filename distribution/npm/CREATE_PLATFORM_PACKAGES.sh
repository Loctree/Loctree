#!/bin/bash
# Script to create the currently supported platform package directories

set -e

VERSION="${1:-0.8.16}"

PLATFORMS=(
  "darwin-arm64:macOS Apple Silicon (ARM64):darwin:arm64"
  "linux-x64-gnu:Linux x64 (glibc):linux:x64"
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
  "license": "MIT OR Apache-2.0",
  "os": ["$os"],
  "cpu": ["$cpu"],
  "repository": {
    "type": "git",
    "url": "git+https://github.com/Loctree/Loctree.git"
  },
  "files": [
    "loctree$([ "$os" = "win32" ] && echo ".exe" || echo "")",
    "postinstall.js"
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
echo "1. Run this script to refresh all supported package directories"
echo "2. Publish each platform package"
echo "3. Publish the main package"
