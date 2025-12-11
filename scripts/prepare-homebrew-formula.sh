#!/usr/bin/env bash
# Helper script to prepare Homebrew formula with correct SHA256
#
# Usage:
#   ./scripts/prepare-homebrew-formula.sh [VERSION]
#
# Example:
#   ./scripts/prepare-homebrew-formula.sh 0.6.8

set -euo pipefail

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  # Try to extract from Cargo.toml
  if [ -f "loctree_rs/Cargo.toml" ]; then
    VERSION=$(grep '^version' loctree_rs/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    echo "Detected version from Cargo.toml: $VERSION"
  else
    echo "Usage: $0 <VERSION>"
    echo "Example: $0 0.6.8"
    exit 1
  fi
fi

CRATES_URL="https://crates.io/api/v1/crates/loctree/${VERSION}/download"
FORMULA_FILE="Formula/loctree.rb"

echo "=== Preparing Homebrew Formula for loctree v${VERSION} ==="
echo ""

# Check if formula template exists
if [ ! -f "$FORMULA_FILE" ]; then
  echo "❌ Formula template not found at: $FORMULA_FILE"
  exit 1
fi

# Download the crate tarball
echo "📥 Downloading crate from crates.io..."
TEMP_DIR=$(mktemp -d)
TARBALL="${TEMP_DIR}/loctree-${VERSION}.crate"

if curl -fsSL -o "$TARBALL" "$CRATES_URL"; then
  echo "✅ Downloaded successfully"
else
  echo "❌ Failed to download from: $CRATES_URL"
  echo "   Make sure version $VERSION is published on crates.io"
  rm -rf "$TEMP_DIR"
  exit 1
fi

# Calculate SHA256
echo "🔐 Calculating SHA256..."
if command -v sha256sum >/dev/null 2>&1; then
  SHA256=$(sha256sum "$TARBALL" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  SHA256=$(shasum -a 256 "$TARBALL" | awk '{print $1}')
else
  echo "❌ Neither sha256sum nor shasum found"
  rm -rf "$TEMP_DIR"
  exit 1
fi

echo "✅ SHA256: $SHA256"

# Update the formula
echo "📝 Updating formula..."
sed -i.bak \
  -e "s|url \"https://crates.io/api/v1/crates/loctree/[^\"]*\"|url \"${CRATES_URL}\"|" \
  -e "s|sha256 \"[^\"]*\"|sha256 \"${SHA256}\"|" \
  "$FORMULA_FILE"

# Clean up backup and temp files
rm -f "${FORMULA_FILE}.bak"
rm -rf "$TEMP_DIR"

echo "✅ Formula updated successfully!"
echo ""
echo "=== Next Steps ==="
echo ""
echo "1. Test the formula locally:"
echo "   brew install --build-from-source ./${FORMULA_FILE}"
echo "   brew test loctree"
echo "   brew audit --strict --online loctree"
echo ""
echo "2. If tests pass, submit to homebrew-core:"
echo "   brew bump-formula-pr loctree \\"
echo "     --url=\"${CRATES_URL}\" \\"
echo "     --sha256=\"${SHA256}\""
echo ""
echo "Or wait for the automated GitHub Actions workflow to handle it!"
