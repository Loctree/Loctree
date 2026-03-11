# GitHub Actions Workflows

This directory contains automated workflows for the loctree project.

## Active Workflows

### Release & Publishing

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **publish.yml** | Tag push (`v*`) | Publish crates, build release assets, publish npm, create GitHub release | ✅ Active |
| **homebrew-release.yml** | GitHub release published | Sync Homebrew formula to `Loctree/homebrew-loctree` | ✅ Active |

### CI & Quality

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **ci.yml** | Push, PR | Workspace fmt, clippy, tests on self-hosted Linux + macOS | ✅ Active |
| **loctree-ci.yml** | Push, PR | Self-analysis dogfooding on self-hosted Linux + macOS | ✅ Active |
| **semgrep.yml** | Push, PR | Security scanning on self-hosted Linux | ✅ Active |

### AI Assistants

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **claude.yml** | Manual dispatch | Claude AI assistance | ✅ Active |
| **gemini-*.yml** | Issues, PR comments | Gemini AI triage and review | ✅ Active |
| **codex-auto-fix.yml** | PR comments | Automated code fixes | ✅ Active |

## Setup Required

### Release Secrets

The release pipeline expects these repository secrets in `Loctree/Loctree`:

- `CARGO_REGISTRY_TOKEN`
- `NPM_TOKEN`
- `HOMEBREW_GITHUB_API_TOKEN`
- `MACOS_CERT_P12_BASE64`
- `MACOS_CERT_PASSWORD`
- `MACOS_KEYCHAIN_PASSWORD`
- `MACOS_DEVELOPER_ID_APPLICATION`
- `APPLE_ID`
- `APPLE_TEAM_ID`
- `APPLE_APP_SPECIFIC_PASSWORD`

Self-hosted runners are currently expected to be available under:

- `[self-hosted, macOS, ARM64, dragon]`
- `[self-hosted, Linux, X64, ops]`

macOS release notes:

- The signing import step hydrates the Apple `DeveloperIDG2CA.cer` intermediate that matches the current `MACOS_CERT_P12_BASE64` chain.
- Public releases currently ship macOS Apple Silicon artifacts only. We intentionally do not build a `darwin-x64` target in this repo.

## Release Process

When you push a version tag, multiple workflows are triggered automatically:

```bash
git tag v0.7.0
git push origin v0.7.0
```

**Workflow sequence:**
1. `publish.yml` runs first
   - Verifies checked-in versions match the tag
   - Publishes `report-leptos`, `loctree`, and `loctree-mcp`
   - Builds release assets on self-hosted Linux/macOS plus hosted Windows
   - Publishes npm from `distribution/npm`
   - Creates GitHub release
2. `homebrew-release.yml` triggers on release
   - Syncs the formula into `Loctree/homebrew-loctree`

**Timeline**: ~10-15 minutes from tag push to Homebrew PR creation

## Monitoring

- **Workflow runs**: https://github.com/Loctree/Loctree/actions
- **Homebrew PRs**: https://github.com/Loctree/homebrew-loctree/pulls
- **Release status**: https://github.com/Loctree/Loctree/releases

## Documentation

- [HOMEBREW_SETUP.md](HOMEBREW_SETUP.md) - Quick setup guide for Homebrew automation
- [../docs/HOMEBREW_RELEASE.md](../docs/HOMEBREW_RELEASE.md) - Complete Homebrew documentation
- [../CONTRIBUTING.md](../CONTRIBUTING.md) - General contribution guidelines
