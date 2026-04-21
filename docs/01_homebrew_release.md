# Homebrew Release Architecture

Loctree no longer treats `homebrew-core` as the primary install path.

The shipping architecture is now:

- source + CI + versioning: `Loctree/loctree-ast`
- CLI binary releases: `Loctree/loct`
- MCP binary releases: `Loctree/loctree-mcp`
- CLI tap: `Loctree/homebrew-cli`
- MCP tap: `Loctree/homebrew-mcp`

## User-Facing Commands

```bash
brew install loctree/cli/loct
brew install loctree/mcp/loctree-mcp
```

## Why This Shape

- The monorepo stays focused on code and CI instead of serving as a public asset bucket.
- CLI and MCP now have separate binary channels, which keeps install paths honest.
- Homebrew formulas can target exactly one product each.
- Release automation becomes deterministic: build once in the monorepo, distribute outward.

## Release Sequence

The human trigger remains the same:

```bash
make version TYPE=minor TAG=1 PUSH=1
```

That tag push triggers:

1. crate publishing in `Loctree/loctree-ast`
2. binary builds for CLI and MCP
3. asset upload to `Loctree/loct` and `Loctree/loctree-mcp`
4. npm publish from `distribution/npm`
5. monorepo release publication
6. Homebrew tap sync into `Loctree/homebrew-cli` and `Loctree/homebrew-mcp`

## Homebrew Formula Source of Truth

The formulas are rendered by:

```bash
scripts/render-homebrew-formula.sh
```

The workflow computes release SHA256 checksums from the thin repos and writes the
resulting files directly into the tap repos. The tap repos should not maintain
hand-edited version drift.

## First Release Bootstrap

Before the first release on this layout, create these GitHub repositories:

- `Loctree/loct`
- `Loctree/loctree-mcp`
- `Loctree/homebrew-cli`
- `Loctree/homebrew-mcp`

Also configure `HOMEBREW_GITHUB_API_TOKEN` in `Loctree/loctree-ast` with write access
to all four repositories.

## Supported Homebrew Targets

- macOS Apple Silicon
- macOS Intel
- Linux x86_64

## Operational Notes

- The monorepo release is an orchestration/changelog release, not the main binary channel.
- npm should prefer CLI assets from `Loctree/loct`; the monorepo release stays a
  temporary fallback while mirror lag catches up.
- If a tap sync fails, fix the thin release assets first, then re-run `homebrew-release.yml`.
