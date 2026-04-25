# npm Package Overview

`distribution/npm` is the canonical npm release surface for Loctree.

## Shape

- main package: `loctree`
- command alias: `loct`
- platform packages:
  - `@loctree/darwin-arm64`
  - `@loctree/darwin-x64`
  - `@loctree/linux-x64-gnu`
  - `@loctree/win32-x64-msvc`

The main package depends on those platform packages through
`optionalDependencies`. Each platform package first tries the matching GitHub
release asset from `Loctree/loct`, then falls back to the monorepo release in
`Loctree/loctree-ast` if the thin repo has not mirrored that asset yet.

## Why this shape

- one publish home under `distribution/npm`
- no accidental root-level npm publish
- only claim the targets CI actually builds

## Release contract

1. Build release assets in GitHub Actions.
2. Sync versions with `node distribution/npm/sync-version.mjs <version>`.
3. Regenerate platform package manifests with `./distribution/npm/CREATE_PLATFORM_PACKAGES.sh <version>`.
4. Publish platform packages first.
5. Publish the main `loctree` package last.
