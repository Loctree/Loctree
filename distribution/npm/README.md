# loctree

Structural code intelligence for AI agents.

This package is the canonical npm distribution surface for Loctree. It installs
the matching platform package, which then downloads the corresponding GitHub
release asset for your machine from the `Loctree/loct` thin release repo while
keeping the source of truth in `Loctree/Loctree`.

## Supported npm targets

- macOS Apple Silicon: `@loctree/darwin-arm64`
- Linux x64 glibc: `@loctree/linux-x64-gnu`
- Windows x64: `@loctree/win32-x64-msvc`

We only claim the targets CI actually builds today.

## Install

```bash
npm install loctree
# or
pnpm add loctree
```

For global CLI usage:

```bash
npm install -g loctree
```

## CLI examples

```bash
npx loct .
npx loct health
npx loct slice src/App.tsx --consumers
npx loct report --serve --port 4173
```

## What you get

- dependency-aware structural analysis
- dead code and cycle signals
- report generation
- Tauri bridge analysis
- MCP-ready artifacts for AI workflows

## Troubleshooting

If installation fails:

1. verify the matching GitHub release assets exist
2. ensure your package manager did not disable `optionalDependencies`
3. fall back to `cargo install loctree`

## Links

- Source: https://github.com/Loctree/Loctree
- CLI releases: https://github.com/Loctree/loct/releases
- Docs: https://docs.rs/loctree
- Website: https://loct.io
