# Homebrew Automation Setup

Quick reference for setting up automated Homebrew formula updates.

## TL;DR

1. **Create GitHub Personal Access Token**
   - Go to: https://github.com/settings/tokens/new
   - Name: `Homebrew Formula Bot - loctree`
   - Scopes: `repo` + `workflow`
   - Generate and copy token

2. **Add Secret to Repository**
   - Go to: https://github.com/Loctree/Loctree-suite/settings/secrets/actions
   - New secret: `HOMEBREW_GITHUB_API_TOKEN`
   - Paste token
   - Save

3. **Done!**
   - Next release will automatically create a Homebrew PR
   - Monitor: https://github.com/Homebrew/homebrew-core/pulls

## Important Notes

### First-Time Submission

The automation updates *existing* formulas. For the initial submission to Homebrew:

**Option A: Automated (when ready)**
```bash
# Just create a release - the workflow handles everything
git tag v0.7.0
git push origin v0.7.0
```

**Option B: Manual submission**
```bash
# Use the helper script
./scripts/prepare-homebrew-formula.sh 0.7.0

# Test locally
brew install --build-from-source ./Formula/loctree.rb
brew test loctree
brew audit --strict loctree

# Submit to homebrew-core
brew bump-formula-pr loctree \
  --url=https://crates.io/api/v1/crates/loctree/0.7.0/download \
  --sha256=<from-script-output>
```

### When to Submit to Homebrew

Consider submitting when:
- ✅ Version is stable (v0.7.0+ or v1.0.0)
- ✅ CI passes consistently
- ✅ Documentation is complete
- ✅ Tests are comprehensive
- ✅ Ready for broader adoption

Don't submit too early - Homebrew maintainers prefer stable releases.

## Files Created

1. **`.github/workflows/homebrew-release.yml`**
   - Automated workflow triggered on GitHub releases
   - Calculates SHA256 from crates.io tarball
   - Creates PR to Homebrew/homebrew-core

2. **`Formula/loctree.rb`**
   - Reference Homebrew formula template
   - Use for local testing and initial submission

3. **`scripts/prepare-homebrew-formula.sh`**
   - Helper script to calculate SHA256 and update formula
   - Run before manual submission

4. **`docs/HOMEBREW_RELEASE.md`**
   - Complete documentation of the process
   - Troubleshooting guide
   - Integration details

## Workflow Integration

```
Developer creates release (v0.7.0)
  ↓
publish.yml workflow runs
  ├─ Publishes to crates.io
  ├─ Builds binaries
  └─ Creates GitHub release
      ↓
homebrew-release.yml triggers (on release.published)
  ├─ Extracts version from tag
  ├─ Downloads tarball from crates.io
  ├─ Calculates SHA256
  └─ Creates PR to Homebrew/homebrew-core
      ↓
Homebrew CI validates the formula
  ↓
Maintainers review and merge
  ↓
Users install: brew install loctree
```

## Testing the Workflow

### Dry Run (Manual Trigger)

1. Go to: https://github.com/Loctree/Loctree-suite/actions/workflows/homebrew-release.yml
2. Click "Run workflow"
3. Enter version: `0.6.8`
4. Click "Run workflow"
5. Monitor execution

### Production Run (Real Release)

```bash
# Create and push tag (triggers both workflows)
git tag v0.7.0
git push origin v0.7.0

# Watch workflows
# - publish.yml: https://github.com/Loctree/Loctree-suite/actions/workflows/publish.yml
# - homebrew-release.yml: https://github.com/Loctree/Loctree-suite/actions/workflows/homebrew-release.yml
```

## Security

- ✅ Token stored in GitHub Secrets (encrypted)
- ✅ Token scoped to minimum required permissions
- ✅ Workflow runs in isolated environment
- ✅ No secrets exposed in logs

**Token Rotation**: Recommended every 6-12 months

## Support

- **Automation issues**: Open issue in this repo
- **Formula issues**: https://github.com/Homebrew/homebrew-core/issues
- **Installation help**: https://docs.brew.sh/Troubleshooting

## Full Documentation

See: [docs/HOMEBREW_RELEASE.md](../docs/HOMEBREW_RELEASE.md)
