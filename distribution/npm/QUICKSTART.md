# npm Quick Start

Fast path to publish the `loctree` npm surface after the CLI thin release
assets already exist.

## Prerequisites

- npm publish access
- CLI thin release assets published to `Loctree/loct`
- Node.js 20+ available (matches the release workflow)

## 1. Sync Versions

Run from the repo root:

```bash
VERSION="$(node -p "require('./distribution/npm/package.json').version")"
cd distribution/npm
node sync-version.mjs "$VERSION"
./CREATE_PLATFORM_PACKAGES.sh "$VERSION"
```

## 2. Verify Thin Assets

For `v$VERSION`, the CLI release repo should already contain:

- `loct-darwin-aarch64.tar.gz`
- `loct-linux-x86_64.tar.gz`
- `loct-windows-x86_64.exe.zip`

Release page:

```text
https://github.com/Loctree/loct/releases/tag/v$VERSION
```

## 3. Publish Platform Packages

```bash
for dir in platform-packages/*/; do
  (cd "$dir" && npm publish --access public)
done
```

Wait a few seconds for npm registry propagation.

## 4. Publish the Main Package

```bash
npm publish --access public
```

## 5. Smoke Test

```bash
mkdir -p /tmp/loctree-npm-smoke
cd /tmp/loctree-npm-smoke
npm init -y
npm install loctree
npx loct --version
```

## Canonical Automation

The normal path is still the repo release workflow, not manual publishing:

```bash
make version TYPE=patch TAG=1 PUSH=1
```

That tag push runs `.github/workflows/publish.yml`, which refreshes versions,
publishes platform packages, and then publishes the main package.

## Need More Detail?

Read [PUBLISHING.md](./PUBLISHING.md) for the full operator guide and the live
sources of truth.
