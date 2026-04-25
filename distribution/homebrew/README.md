# Homebrew Distribution

The monorepo does not ship one generic Homebrew formula anymore.

Instead:

- CLI formula is rendered into `Loctree/homebrew-cli`
- MCP formula is rendered into `Loctree/homebrew-mcp`

The rendering source of truth lives in:

- `scripts/render-homebrew-formula.sh`
- `.github/workflows/homebrew-release.yml`

## Release Contract

1. `publish.yml` builds and uploads binary assets into the thin repos:
   - `Loctree/loct`
   - `Loctree/loctree-mcp`
2. `homebrew-release.yml` downloads those tarballs, computes SHA256 values, and
   writes the tap formulas.

## Local Test Flow

After a release exists, you can render formulas locally by exporting the same
SHA variables the workflow uses and running:

```bash
scripts/render-homebrew-formula.sh loct 0.9.0 /tmp/loct.rb
scripts/render-homebrew-formula.sh loctree-mcp 0.9.0 /tmp/loctree-mcp.rb
```

Then test with Homebrew:

```bash
brew install --build-from-source /tmp/loct.rb
brew install --build-from-source /tmp/loctree-mcp.rb
```
