# GitHub Actions Workflows

This directory contains automated workflows for the loctree project.

## Active Workflows

### Release & Publishing

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **publish.yml** | Tag push (`v*`) | Publish to crates.io, build binaries, create GitHub release | ✅ Active |
| **homebrew-release.yml** | GitHub release published | Update Homebrew formula in homebrew-core | ✅ Active (requires setup) |

### CI & Quality

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **ci.yml** | Push, PR | Run tests, check formatting, clippy | ✅ Active |
| **loctree-ci.yml** | Push, PR | Run loctree analysis on itself | ✅ Active |
| **semgrep.yml** | Push, PR | Security scanning | ✅ Active |

### AI Assistants

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **claude.yml** | Manual dispatch | Claude AI assistance | ✅ Active |
| **gemini-*.yml** | Issues, PR comments | Gemini AI triage and review | ✅ Active |
| **codex-auto-fix.yml** | PR comments | Automated code fixes | ✅ Active |

## Setup Required

### Homebrew Automation

The `homebrew-release.yml` workflow requires a GitHub Personal Access Token:

1. **Create token**: https://github.com/settings/tokens/new
   - Scopes: `repo` + `workflow`
   - Name: `Homebrew Formula Bot - loctree`

2. **Add to secrets**: https://github.com/Loctree/Loctree-suite/settings/secrets/actions
   - Name: `HOMEBREW_GITHUB_API_TOKEN`
   - Value: (paste token)

See [HOMEBREW_SETUP.md](HOMEBREW_SETUP.md) for details.

## Release Process

When you push a version tag, multiple workflows are triggered automatically:

```bash
git tag v0.7.0
git push origin v0.7.0
```

**Workflow sequence:**
1. `publish.yml` runs first
   - Syncs all crate versions
   - Publishes to crates.io
   - Builds cross-platform binaries
   - Creates GitHub release
2. `homebrew-release.yml` triggers on release
   - Downloads tarball from crates.io
   - Calculates SHA256
   - Creates PR to Homebrew/homebrew-core

**Timeline**: ~10-15 minutes from tag push to Homebrew PR creation

## Monitoring

- **Workflow runs**: https://github.com/Loctree/Loctree-suite/actions
- **Homebrew PRs**: https://github.com/Homebrew/homebrew-core/pulls?q=loctree
- **Release status**: https://github.com/Loctree/Loctree-suite/releases

## Documentation

- [HOMEBREW_SETUP.md](HOMEBREW_SETUP.md) - Quick setup guide for Homebrew automation
- [../docs/HOMEBREW_RELEASE.md](../docs/HOMEBREW_RELEASE.md) - Complete Homebrew documentation
- [../CONTRIBUTING.md](../CONTRIBUTING.md) - General contribution guidelines
