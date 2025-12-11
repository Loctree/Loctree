# Homebrew Formula Automation

This document describes the automated Homebrew formula update workflow for loctree.

## Overview

When a new version of loctree is published to crates.io via a GitHub release, the Homebrew formula is automatically updated via a pull request to [Homebrew/homebrew-core](https://github.com/Homebrew/homebrew-core).

## Workflow File

`.github/workflows/homebrew-release.yml`

## How It Works

1. **Trigger**: Activates when a GitHub release is published (tags matching `v*` or `loctree-v*`)
2. **Version Extraction**: Extracts version number from the release tag
3. **Download URL**: Constructs the crates.io download URL: `https://crates.io/api/v1/crates/loctree/{version}/download`
4. **SHA256 Calculation**: Automatically fetches the tarball and calculates SHA256 checksum
5. **PR Creation**: Creates a pull request to Homebrew/homebrew-core with the updated formula

## Prerequisites

### 1. Personal Access Token

The workflow requires a GitHub Personal Access Token with specific permissions:

**Required Scopes:**
- `repo` - Full control of private repositories
- `workflow` - Update GitHub Action workflows

**Setup Steps:**

1. Go to [GitHub Settings → Personal Access Tokens → Tokens (classic)](https://github.com/settings/tokens)
2. Click "Generate new token (classic)"
3. Give it a descriptive name: `Homebrew Formula Bot - loctree`
4. Select scopes:
   - ✅ `repo`
   - ✅ `workflow`
5. Click "Generate token"
6. **IMPORTANT**: Copy the token immediately (you won't see it again)

### 2. Add Secret to Repository

1. Go to [loctree repository settings](https://github.com/Loctree/Loctree-suite/settings/secrets/actions)
2. Click "New repository secret"
3. Name: `HOMEBREW_GITHUB_API_TOKEN`
4. Value: Paste the token from step 1
5. Click "Add secret"

### 3. Initial Formula Submission

**Note**: This workflow updates *existing* formulas. For the first-time submission to Homebrew:

1. Wait for a stable release (e.g., v0.7.0 or v1.0.0)
2. Manually create the formula using:
   ```bash
   brew create https://crates.io/api/v1/crates/loctree/0.7.0/download
   ```
3. Test the formula locally:
   ```bash
   brew install --build-from-source ./loctree.rb
   brew test loctree
   brew audit --strict loctree
   ```
4. Submit to homebrew-core:
   ```bash
   brew bump-formula-pr loctree \
     --url=https://crates.io/api/v1/crates/loctree/0.7.0/download \
     --sha256=<calculated-sha>
   ```

After the initial formula is merged, the automated workflow will handle all future updates.

## Manual Trigger

You can manually trigger the workflow for a specific version:

1. Go to [Actions → Update Homebrew Formula](https://github.com/Loctree/Loctree-suite/actions/workflows/homebrew-release.yml)
2. Click "Run workflow"
3. Enter the version (e.g., `0.6.8`)
4. Click "Run workflow"

## Monitoring

After the workflow runs:

1. **Check GitHub Actions**: [View workflow runs](https://github.com/Loctree/Loctree-suite/actions/workflows/homebrew-release.yml)
2. **Find the PR**: Search [Homebrew/homebrew-core PRs](https://github.com/Homebrew/homebrew-core/pulls?q=is%3Apr+loctree)
3. **Monitor CI**: Homebrew runs extensive tests on the formula
4. **Address Feedback**: Respond to any comments from Homebrew maintainers

## Troubleshooting

### Workflow Fails with Authentication Error

**Error**: `Authentication failed` or `Resource not accessible by integration`

**Solution**:
- Verify `HOMEBREW_GITHUB_API_TOKEN` is set correctly
- Check token has `repo` and `workflow` scopes
- Ensure token hasn't expired

### Version Mismatch

**Error**: `Could not find version X.Y.Z on crates.io`

**Solution**:
- Wait a few minutes after publishing to crates.io
- Verify the version is published: `cargo search loctree`
- Check crates.io directly: https://crates.io/crates/loctree

### Formula Already Up-to-Date

**Error**: `Formula is already at version X.Y.Z`

**Solution**: This is expected if someone already updated the formula. No action needed.

### Download URL Not Accessible

**Error**: `Could not download tarball`

**Solution**:
- Verify the crates.io URL is correct
- Check if crates.io is experiencing issues
- Ensure version is fully published (not just pushed)

## Formula Location

Once submitted and merged, the formula will be available at:
- **Repository**: https://github.com/Homebrew/homebrew-core
- **File**: `Formula/l/loctree.rb`
- **Install command**: `brew install loctree`

## Integration with Existing Workflow

This workflow is designed to work seamlessly with the existing `publish.yml` workflow:

1. `publish.yml` triggers on tag push (`v*` or `loctree-v*`)
2. Publishes to crates.io
3. Creates GitHub release
4. `homebrew-release.yml` triggers on release publication
5. Updates Homebrew formula

**Timeline**: Expect the entire process (tag → crates.io → GitHub release → Homebrew PR) to take 5-15 minutes.

## Example Workflow Run

```
Tag pushed: v0.7.0
  ↓
publish.yml runs
  ├─ Publishes to crates.io
  └─ Creates GitHub release
      ↓
homebrew-release.yml triggers
  ├─ Extracts version: 0.7.0
  ├─ Fetches: https://crates.io/api/v1/crates/loctree/0.7.0/download
  ├─ Calculates SHA256
  └─ Creates PR to Homebrew/homebrew-core
      ↓
Homebrew CI runs
  ├─ Builds formula
  ├─ Runs tests
  └─ Validates formula
      ↓
Maintainer reviews and merges
  ↓
Users can install: brew install loctree
```

## Security Considerations

### Token Security

- **Never commit tokens to the repository**
- Use GitHub Secrets for sensitive values
- Rotate tokens periodically (recommended: every 6-12 months)
- Use minimal required scopes

### Formula Security

Homebrew maintainers will review the formula for:
- Source integrity (SHA256 verification)
- Build process security
- No malicious code
- Proper licensing

## Future Improvements

Potential enhancements to consider:

1. **Auto-merge for patch versions**: Use GitHub Apps to auto-approve minor updates
2. **Release notes**: Include changelog in the PR description
3. **Notification**: Slack/Discord notification when PR is created
4. **Testing**: Add pre-flight checks before creating PR

## References

- [mislav/bump-homebrew-formula-action](https://github.com/mislav/bump-homebrew-formula-action)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew/homebrew-core Contributing Guide](https://github.com/Homebrew/homebrew-core/blob/master/CONTRIBUTING.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

## Support

For issues with:
- **Workflow automation**: Open an issue in this repository
- **Homebrew formula**: Open an issue in [Homebrew/homebrew-core](https://github.com/Homebrew/homebrew-core/issues)
- **Installation problems**: Check [Homebrew Troubleshooting](https://docs.brew.sh/Troubleshooting)
