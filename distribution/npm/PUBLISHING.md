# Publishing Guide for the loctree npm Package

The npm package is a CLI wrapper around thin release assets from
`Loctree/loct`. The canonical publish path is the monorepo release workflow:

- verify release tag and versions
- publish thin release assets
- refresh npm package versions
- publish platform packages
- publish the main `loctree` package

See `.github/workflows/publish.yml` for the live source of truth.

## Current Layout

The npm surface follows the `optionalDependencies` pattern:

1. `loctree` — main package with the JavaScript wrapper
2. `@loctree/darwin-arm64`
3. `@loctree/darwin-x64`
4. `@loctree/linux-x64-gnu`
5. `@loctree/win32-x64-msvc`

The main package depends on the platform packages. Each platform package
downloads its matching thin release asset from `Loctree/loct`, with a
monorepo-release fallback in `Loctree/loctree-ast` while mirroring catches up.

## Required Thin Assets

For a release tag `vX.Y.Z`, the publish workflow expects these CLI assets:

- `loct-darwin-aarch64.tar.gz`
- `loct-darwin-x86_64.tar.gz`
- `loct-linux-x86_64.tar.gz`
- `loct-windows-x86_64.zip`

The npm platform packages currently consume:

- `loct-darwin-aarch64.tar.gz`
- `loct-darwin-x86_64.tar.gz`
- `loct-linux-x86_64.tar.gz`
- `loct-windows-x86_64.zip`

## Standard Publish Flow

The normal operator path is a tagged release:

```bash
make version TYPE=minor TAG=1 PUSH=1
```

That triggers `.github/workflows/publish.yml`, which then:

1. syncs `distribution/npm/package.json` to the release version
2. regenerates `platform-packages/*` with `CREATE_PLATFORM_PACKAGES.sh`
3. publishes each platform package
4. publishes the main `loctree` npm package

## Manual Fallback

Use this only when you intentionally need a local/manual npm publish:

```bash
VERSION="$(node -p "require('./distribution/npm/package.json').version")"
cd distribution/npm
node sync-version.mjs "$VERSION"
./CREATE_PLATFORM_PACKAGES.sh "$VERSION"
```

Publish platform packages first:

```bash
for dir in platform-packages/*/; do
  (cd "$dir" && npm publish --access public)
done
```

Then publish the main package:

```bash
npm publish --access public
```

## Verification

Sanity-check the release surface before or after publishing:

```bash
VERSION="$(node -p "require('./distribution/npm/package.json').version")"
echo "https://github.com/Loctree/loct/releases/tag/v${VERSION}"

mkdir -p /tmp/loctree-npm-smoke
cd /tmp/loctree-npm-smoke
npm init -y
npm install loctree
npx loct --version
node -e "console.log(require('loctree').getBinaryPath())"
```

## Notes

- Do not hard-code old version numbers in docs or helper commands.
- Do not publish the main package before the platform packages.
- The source of truth for asset filenames is `distribution/npm/platform-packages/postinstall.js`.
- The source of truth for publish choreography is `.github/workflows/publish.yml`.
