# Publishing Guide for loctree npm Package

This guide explains how to publish the loctree npm package and its platform-specific binaries.

## Architecture Overview

This package follows the **esbuild/swc pattern** of using `optionalDependencies` for platform-specific binaries:

1. **Main package** (`loctree`): Contains the wrapper JavaScript code
2. **Platform packages** (`@loctree/*-*`): Each contains a platform-specific binary

The main package lists all platform packages as `optionalDependencies`, so npm/pnpm/yarn will automatically install only the one matching the user's platform.

## Prerequisites

1. **npm account** with publish permissions
2. **GitHub releases** with loctree binaries for all platforms
3. **Node.js 14+** installed locally

## Publishing Steps

### Step 1: Verify GitHub Releases

Ensure the loctree Rust project has published release binaries for version `0.8.11` (or your target version) with these exact filenames:

```
loctree-aarch64-apple-darwin           (macOS ARM64)
loctree-x86_64-apple-darwin            (macOS Intel)
loctree-aarch64-unknown-linux-gnu      (Linux ARM64 glibc)
loctree-aarch64-unknown-linux-musl     (Linux ARM64 musl)
loctree-x86_64-unknown-linux-gnu       (Linux x64 glibc)
loctree-x86_64-unknown-linux-musl      (Linux x64 musl)
loctree-aarch64-pc-windows-msvc.exe    (Windows ARM64)
loctree-x86_64-pc-windows-msvc.exe     (Windows x64)
```

Check: https://github.com/Loctree/Loctree/releases/tag/v0.8.11

### Step 2: Create Platform-Specific Packages

Run the helper script to create all platform package directories:

```bash
./CREATE_PLATFORM_PACKAGES.sh
```

This creates:
```
platform-packages/
├── darwin-arm64/
├── darwin-x64/
├── linux-arm64-gnu/
├── linux-arm64-musl/
├── linux-x64-gnu/
├── linux-x64-musl/
├── win32-arm64-msvc/
└── win32-x64-msvc/
```

### Step 3: Test Platform Package Installation (Optional)

Before publishing, test that the download mechanism works:

```bash
cd platform-packages/darwin-arm64
npm install
# Should download the binary from GitHub releases
ls -lh loctree
./loctree --version
```

### Step 4: Publish Platform Packages

You MUST publish platform packages BEFORE the main package (because the main package depends on them).

```bash
# Publish each platform package
cd platform-packages/darwin-arm64
npm publish --access public

cd ../darwin-x64
npm publish --access public

cd ../linux-arm64-gnu
npm publish --access public

cd ../linux-arm64-musl
npm publish --access public

cd ../linux-x64-gnu
npm publish --access public

cd ../linux-x64-musl
npm publish --access public

cd ../win32-arm64-msvc
npm publish --access public

cd ../win32-x64-msvc
npm publish --access public
```

**Alternative (automated):**

```bash
for dir in platform-packages/*/; do
  (cd "$dir" && npm publish --access public)
done
```

### Step 5: Publish Main Package

After all platform packages are published:

```bash
cd /path/to/loctree-npm-package
npm publish
```

### Step 6: Verify Installation

Test on different platforms if possible:

```bash
# Create a test directory
mkdir /tmp/loctree-test
cd /tmp/loctree-test
npm init -y
npm install loctree

# Verify binary works
npx loctree --version
node -e "console.log(require('loctree').getBinaryPath())"
```

## Version Updates

When releasing a new version (e.g., `0.6.15`):

1. **Update all package.json files** (main + 8 platform packages):
   ```bash
   # Use sed or manually update version in:
   # - package.json
   # - platform-packages/*/package.json
   ```

2. **Update VERSION in postinstall.js** (if using version-based URLs)

3. **Re-publish platform packages first**, then the main package

## Troubleshooting

### "Package not found" errors

- Ensure platform packages are published BEFORE the main package
- Check that package names match exactly: `@loctree/darwin-arm64`, etc.
- Verify packages are public: `npm access public @loctree/darwin-arm64`

### Binary download failures

- Verify GitHub release exists with correct tag: `v0.8.11` (note the `v` prefix)
- Check binary filenames match the `BINARY_MAPPINGS` in `postinstall.js`
- Test download URL manually: `curl -L https://github.com/Loctree/Loctree/releases/download/v0.8.11/loctree-x86_64-apple-darwin -o test`

### optionalDependencies not installing

- Some package managers can disable optionalDependencies
- Check with: `npm install --no-optional` (should fail)
- Normal install: `npm install` (should work)

## Alternative: Direct Binary Bundling

If GitHub releases are unreliable, you can bundle binaries directly in platform packages:

1. Download all binaries locally
2. Place each binary in its respective platform package directory
3. Remove or simplify the `postinstall.js` script
4. Increase package size limits in `.npmrc` if needed

This increases package sizes but eliminates download dependencies.

## Package Maintenance

### Sync with Rust releases

Monitor https://github.com/Loctree/Loctree/releases for new versions.

When a new version is released:
1. Update all version numbers
2. Test on multiple platforms
3. Publish platform packages
4. Publish main package
5. Test installation

### Deprecating old versions

```bash
npm deprecate loctree@0.6.13 "Please upgrade to 0.8.11"
```

## Resources

- [npm publishing docs](https://docs.npmjs.com/cli/v8/commands/npm-publish)
- [optionalDependencies](https://docs.npmjs.com/cli/v8/configuring-npm/package-json#optionaldependencies)
- [esbuild npm package strategy](https://esbuild.github.io/getting-started/)
- [How to publish binaries on npm (Sentry blog)](https://sentry.engineering/blog/publishing-binaries-on-npm)
