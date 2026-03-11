# Quick Start Guide

Get the loctree npm package published in 5 steps.

## Prerequisites

- npm account (`npm login`)
- loctree Rust binaries published to GitHub releases
- Node.js 14+ installed

## Step 1: Verify GitHub Releases

Ensure release assets exist at:
```
https://github.com/Loctree/Loctree/releases/tag/v0.8.16
```

Required files:
- `loctree-darwin-aarch64.tar.gz`
- `loctree-linux-x86_64.tar.gz`
- `loctree-windows-x86_64.exe.zip`

## Step 2: Generate Platform Packages

```bash
./CREATE_PLATFORM_PACKAGES.sh
```

This creates the currently supported platform package directories.

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

Check that the GitHub release exists with the current supported assets.

### "Permission denied"

Run `chmod +x` on the binary path shown in the error.

## Next Steps

- Read [PUBLISHING.md](./PUBLISHING.md) for detailed publishing guide
- Read [PACKAGE_OVERVIEW.md](./PACKAGE_OVERVIEW.md) for architecture details
- Set up CI/CD with `.github/workflows/test-install.yml`

---

**Need help?** Open an issue at https://github.com/Loctree/Loctree/issues
