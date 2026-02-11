#!/bin/bash
# Automated publishing script for loctree npm packages

set -e

VERSION=${1:-"0.8.11"}
DRY_RUN=${DRY_RUN:-false}

echo "=== loctree npm Publishing Script ==="
echo "Version: $VERSION"
echo "Dry run: $DRY_RUN"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're logged in to npm
if ! npm whoami &> /dev/null; then
  echo -e "${RED}Error: Not logged in to npm${NC}"
  echo "Run: npm login"
  exit 1
fi

NPM_USER=$(npm whoami)
echo -e "${GREEN}Logged in as: $NPM_USER${NC}"
echo ""

# Function to publish a package
publish_package() {
  local package_dir=$1
  local package_name=$2

  echo -e "${YELLOW}Publishing $package_name...${NC}"

  cd "$package_dir"

  if [ "$DRY_RUN" = true ]; then
    echo "  [DRY RUN] Would publish: $package_name"
    npm pack --dry-run
  else
    npm publish --access public
    echo -e "${GREEN}  ✓ Published $package_name${NC}"
  fi

  cd - > /dev/null
  echo ""
}

# Step 1: Verify GitHub releases
echo "Step 1: Verifying GitHub releases..."
RELEASE_URL="https://github.com/Loctree/Loctree/releases/tag/v$VERSION"

if command -v curl &> /dev/null; then
  if curl -s -o /dev/null -w "%{http_code}" "$RELEASE_URL" | grep -q "200"; then
    echo -e "${GREEN}  ✓ Release v$VERSION exists${NC}"
  else
    echo -e "${RED}  ✗ Release v$VERSION not found at $RELEASE_URL${NC}"
    echo "  Create the release first, then try again"
    exit 1
  fi
else
  echo -e "${YELLOW}  ⚠ curl not found, skipping release verification${NC}"
fi
echo ""

# Step 2: Create platform packages
echo "Step 2: Creating platform packages..."
if [ -x ./CREATE_PLATFORM_PACKAGES.sh ]; then
  ./CREATE_PLATFORM_PACKAGES.sh
else
  echo -e "${YELLOW}  ⚠ CREATE_PLATFORM_PACKAGES.sh not found or not executable${NC}"
fi
echo ""

# Step 3: Publish platform packages
echo "Step 3: Publishing platform packages..."

PLATFORMS=(
  "darwin-arm64:@loctree/darwin-arm64"
  "darwin-x64:@loctree/darwin-x64"
  "linux-arm64-gnu:@loctree/linux-arm64-gnu"
  "linux-arm64-musl:@loctree/linux-arm64-musl"
  "linux-x64-gnu:@loctree/linux-x64-gnu"
  "linux-x64-musl:@loctree/linux-x64-musl"
  "win32-arm64-msvc:@loctree/win32-arm64-msvc"
  "win32-x64-msvc:@loctree/win32-x64-msvc"
)

for platform_spec in "${PLATFORMS[@]}"; do
  IFS=: read -r platform_dir package_name <<< "$platform_spec"
  publish_package "platform-packages/$platform_dir" "$package_name"
done

# Step 4: Wait for packages to propagate
if [ "$DRY_RUN" = false ]; then
  echo "Step 4: Waiting for packages to propagate..."
  echo "  Sleeping for 30 seconds to ensure npm registry is updated..."
  sleep 30
  echo ""
fi

# Step 5: Publish main package
echo "Step 5: Publishing main package..."
publish_package "." "loctree"

# Step 6: Verify installation
if [ "$DRY_RUN" = false ]; then
  echo "Step 6: Verifying installation..."

  TEST_DIR=$(mktemp -d)
  cd "$TEST_DIR"

  echo "  Testing in: $TEST_DIR"
  npm init -y > /dev/null
  npm install loctree

  if npx loctree --version; then
    echo -e "${GREEN}  ✓ Installation verified successfully${NC}"
  else
    echo -e "${RED}  ✗ Installation verification failed${NC}"
    exit 1
  fi

  cd - > /dev/null
  rm -rf "$TEST_DIR"
fi

echo ""
echo -e "${GREEN}=== Publishing Complete! ===${NC}"
echo ""
echo "Next steps:"
echo "1. Test installation on different platforms"
echo "2. Update documentation if needed"
echo "3. Announce the release"
echo ""
echo "Test installation with:"
echo "  npm install loctree@$VERSION"
