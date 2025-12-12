# Quick Start Guide

Get the loctree npm package published in 5 steps.

## Prerequisites

- npm account (`npm login`)
- loctree Rust binaries published to GitHub releases
- Node.js 14+ installed

## Step 1: Verify GitHub Releases

Ensure binaries exist at:
```
https://github.com/Loctree/Loctree/releases/tag/v0.6.14
```

Required files:
- `loctree-aarch64-apple-darwin`
- `loctree-x86_64-apple-darwin`
- `loctree-aarch64-unknown-linux-gnu`
- `loctree-aarch64-unknown-linux-musl`
- `loctree-x86_64-unknown-linux-gnu`
- `loctree-x86_64-unknown-linux-musl`
- `loctree-aarch64-pc-windows-msvc.exe`
- `loctree-x86_64-pc-windows-msvc.exe`

## Step 2: Generate Platform Packages

```bash
./CREATE_PLATFORM_PACKAGES.sh
```

This creates 8 platform-specific package directories.

## Step 3: Test Locally (Optional)

```bash
cd platform-packages/darwin-arm64
npm install
./loctree --version
```

## Step 4: Publish

### Option A: Automated (Recommended)

```bash
./scripts/publish-all.sh
```

### Option B: Manual

```bash
# Publish platform packages first
for dir in platform-packages/*/; do
  (cd "$dir" && npm publish --access public)
done

# Then publish main package
npm publish
```

## Step 5: Verify

```bash
mkdir /tmp/test-loctree
cd /tmp/test-loctree
npm init -y
npm install loctree
npx loctree --version
```

## Done!

Your package is now published and ready to use:

```bash
npm install loctree
npx loctree --help
```

## Troubleshooting

### "Platform package not found"

Wait 30-60 seconds for npm registry to propagate, then try again.

### "Binary download failed"

Check that GitHub release v0.6.14 exists with all 8 binaries.

### "Permission denied"

Run `chmod +x` on the binary path shown in the error.

## Next Steps

- Read [PUBLISHING.md](./PUBLISHING.md) for detailed publishing guide
- Read [PACKAGE_OVERVIEW.md](./PACKAGE_OVERVIEW.md) for architecture details
- Set up CI/CD with `.github/workflows/test-install.yml`

---

**Need help?** Open an issue at https://github.com/Loctree/Loctree/issues
