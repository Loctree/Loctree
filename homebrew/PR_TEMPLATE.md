# loctree 0.6.8 (new formula)

## Description

**loctree** is an AI-oriented project analyzer that provides semantic code analysis optimized for AI agents and language models. It generates structured, hierarchical views of codebases that help AI understand project architecture, relationships, and context.

### Key Features

- **Semantic code analysis** - Extracts functions, types, imports, and their relationships
- **AI-optimized output** - Multiple formats (tree, markdown, JSON) designed for LLM consumption
- **Structural test coverage** - Identifies code segments without corresponding tests
- **Framework-aware** - Built-in presets for Tauri, React, Rust, Python, and more
- **Bundle analysis** - Analyzes JavaScript/TypeScript bundles via source maps
- **Contract validation** - Ensures frontend-backend API consistency

### Links

- **Repository**: https://github.com/LibraxisAI/loctree
- **Crates.io**: https://crates.io/crates/loctree
- **Documentation**: https://github.com/LibraxisAI/loctree/blob/main/README.md

### Why This Formula

loctree fills a gap in developer tooling by providing AI agents with structured code analysis that goes beyond simple file trees. It's particularly valuable for:

- AI-assisted development workflows
- Code review automation
- Project documentation generation
- Identifying untested code paths
- Understanding complex codebases

The tool is actively maintained, has a growing user base, and is published on crates.io with regular updates.

---

## Contribution Checklist

- [ ] I have read and followed the [Homebrew contribution guidelines](https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request)
- [ ] I have built the formula from source successfully: `brew install --build-from-source loctree`
- [ ] `brew audit --new loctree` passes without errors
- [ ] `brew test loctree` passes all tests
- [ ] The license (MIT) is OSI-approved
- [ ] The formula does not vendor dependencies (Rust crates are fetched via cargo)
- [ ] The binary does not phone home or collect telemetry
- [ ] I am the author/maintainer of this software (LibraxisAI team)
- [ ] The formula follows Homebrew naming conventions
- [ ] Version 0.6.8 is the latest stable release

---

## Testing Commands

To verify this formula locally before merging:

```bash
# Install from source
brew install --build-from-source loctree

# Run test suite
brew test loctree

# Audit the formula
brew audit --new loctree

# Verify installation
loctree --version
loctree --help

# Test basic functionality
loctree /path/to/test/project
```

---

## Additional Notes

- **Stability**: loctree has been in active development for several months with regular releases
- **Dependencies**: All Rust dependencies are managed via Cargo and fetched during build
- **Platform support**: Works on macOS (Intel/Apple Silicon) and Linux
- **License**: MIT license allows free distribution and modification
- **Maintenance**: Actively maintained by the LibraxisAI team with regular updates

---

## Formula Details

The formula is located at `Formula/loctree.rb` and includes:

- Source tarball from GitHub releases
- SHA256 checksum for verification
- Build dependencies (Rust toolchain via `rust` formula)
- Test suite verifying core functionality
- Man page installation (if available in future releases)

---

**I confirm that I have read the contribution guidelines and this PR meets all requirements for a new formula submission.**

---

Vibecrafted with AI Agents by VetCoders (c)2025 The LibraxisAI Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
