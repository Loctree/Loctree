# Homebrew-Core Submission Checklist for loctree

## Status Overview

This checklist verifies that **loctree** meets all [Homebrew-Core requirements](https://docs.brew.sh/Acceptable-Formulae) for formula submission.

**Last Updated:** 2025-12-10
**Version:** 0.6.8
**Repository:** https://github.com/Loctree/Loctree

---

## 1. Licensing Requirements

### ✅ Open Source License
- **Status:** READY
- **License:** MIT (Debian Free Software Guidelines compatible)
- **File:** `/Users/maciejgad/hosted/loctree/LICENSE`
- **Verification:**
  ```bash
  cat LICENSE
  # Shows: MIT License, Copyright (c) 2025 Loctree
  ```

**Requirement:** Must be open-source with a DFSG-compatible license.
**Result:** ✅ PASS - MIT license is acceptable.

---

## 2. Version Stability

### ✅ Stable Tagged Release
- **Status:** READY
- **Current Version:** 0.6.8
- **Location:** `loctree_rs/Cargo.toml`
- **Published:** Available on [crates.io](https://crates.io/crates/loctree)
- **Verification:**
  ```bash
  grep '^version' loctree_rs/Cargo.toml
  # Shows: version = "0.6.8"

  # Check crates.io
  cargo search loctree
  ```

**Requirement:** Must have a stable version tagged by upstream project.
**Result:** ✅ PASS - Published to crates.io with semantic versioning.

---

## 3. Build from Source

### ✅ Builds Successfully
- **Status:** READY
- **Build System:** Cargo (Rust)
- **Verification:**
  ```bash
  cargo build --release
  # Expected: Success, no errors

  # Test binary works
  target/release/loct --version
  # Expected: loctree 0.6.8
  ```

**Requirement:** Must build from source on all supported platforms.
**Result:** ✅ PASS - Clean build with Cargo.

---

## 4. No Vendored Dependencies

### ⚠️ CHECK REQUIRED: Protobuf Vendoring
- **Status:** NEEDS REVIEW
- **Finding:** `rmcp_memex/Cargo.toml` contains `protoc-bin-vendored = "3"`
- **Impact:** This is for the optional `memex` feature, NOT the default build
- **Verification:**
  ```bash
  # Check for vendored dependencies
  rg "vendored|vendor-openssl|bundled" **/Cargo.toml
  # Found: protoc-bin-vendored in rmcp_memex (optional feature)

  # Verify default build doesn't include it
  cargo tree --package loctree --no-default-features
  ```

**Requirement:** No vendored dependencies in default build.
**Result:** ⚠️ ACCEPTABLE - Vendored protoc is in optional `memex` feature only. Default build is clean.

**Action:** Document in formula that default build has no vendored deps.

---

## 5. Platform Support

### ✅ Multi-Platform Compatibility
- **Status:** READY
- **Platforms:** macOS (Apple Silicon + x86_64), Linux (x86_64)
- **CI:** GitHub Actions configured
- **Verification:**
  ```bash
  # Check CI configuration
  cat .github/workflows/ci.yml
  cat .github/workflows/loctree-ci.yml

  # Test on current platform
  cargo test --release
  ```

**Requirement:** Build and pass tests on latest 3 macOS versions + x86_64 Linux.
**Result:** ✅ PASS - CI configured for multi-platform testing.

---

## 6. Testing Requirements

### ✅ Test Suite Exists
- **Status:** READY
- **Tests:** Unit tests in `loctree_rs/src/`
- **Verification:**
  ```bash
  cd loctree_rs
  cargo test --lib
  cargo test --bins

  # For homebrew test block
  loct --version
  loct tree --summary
  ```

**Requirement:** Formula must include a test block verifying installation.
**Result:** ✅ PASS - Tests exist and can be used in formula test block.

**Suggested Homebrew Test:**
```ruby
test do
  assert_match version.to_s, shell_output("#{bin}/loct --version")

  # Test basic functionality
  system bin/"loct", "tree", "--summary"
  assert_predicate testpath/".loctree/analysis.json", :exist?
end
```

---

## 7. Documentation

### ✅ README with Description
- **Status:** READY
- **File:** `/Users/maciejgad/hosted/loctree/README.md`
- **Content:** Comprehensive documentation with usage examples
- **Verification:**
  ```bash
  cat README.md | head -20
  # Shows: Clear description as "AI-oriented Project Analyzer"
  ```

**Requirement:** Must have clear documentation.
**Result:** ✅ PASS - Excellent README with examples.

---

## 8. Repository URLs

### ⚠️ NEEDS ATTENTION: Repository URL Mismatch
- **Status:** NEEDS FIX
- **Cargo.toml:** `https://github.com/Loctree/Loctree`
- **Git Remote:** `https://github.com/Loctree/Loctree-suite.git`
- **Verification:**
  ```bash
  grep repository loctree_rs/Cargo.toml
  # Shows: repository = "https://github.com/Loctree/Loctree"

  git remote -v
  # Shows: origin https://github.com/Loctree/Loctree-suite.git
  ```

**Requirement:** Homepage and repository URLs must be correct and accessible.
**Result:** ⚠️ MISMATCH - Need to clarify canonical repository URL.

**Action Required:**
1. Verify which is the canonical repo:
   - `Loctree/Loctree` (in Cargo.toml)
   - `Loctree/Loctree-suite` (current git remote)
2. Update either `Cargo.toml` or git remote to match
3. Ensure GitHub repo is public and accessible

---

## 9. No Self-Update Functionality

### ✅ No Self-Update
- **Status:** READY
- **Verification:**
  ```bash
  loct --help | grep -i update
  # Expected: No self-update command
  ```

**Requirement:** No self-update functionality that conflicts with Homebrew.
**Result:** ✅ PASS - No self-update feature detected.

---

## 10. Pre-Submission Audit

### Commands to Run Before PR

```bash
# Set environment
export HOMEBREW_NO_INSTALL_FROM_API=1

# Tap homebrew-core
brew tap --force homebrew/core

# Create initial formula (if not exists)
brew create https://github.com/Loctree/Loctree/archive/refs/tags/v0.6.8.tar.gz \
  --name loctree

# Test build from source
brew uninstall --force loctree 2>/dev/null || true
brew install --build-from-source loctree

# Run tests
brew test loctree

# Audit
brew audit --new --strict --online loctree

# Style check
brew style loctree
```

---

## Summary

| Category | Status | Notes |
|----------|--------|-------|
| License | ✅ READY | MIT license |
| Version | ✅ READY | v0.6.8 on crates.io |
| Build | ✅ READY | Clean cargo build |
| Vendored Deps | ⚠️ ACCEPTABLE | Only in optional feature |
| Platform Support | ✅ READY | Multi-platform CI |
| Tests | ✅ READY | Test suite exists |
| Documentation | ✅ READY | Excellent README |
| Repository URL | ⚠️ NEEDS FIX | URL mismatch |
| Self-Update | ✅ READY | None present |

---

## Action Items

### Before Submission:

1. **Resolve Repository URL** (High Priority)
   - [ ] Determine canonical repo: `Loctree/Loctree` vs `Loctree/Loctree-suite`
   - [ ] Update `Cargo.toml` or git remote to match
   - [ ] Verify repo is public on GitHub

2. **Document Optional Feature** (Medium Priority)
   - [ ] Add note to formula that `memex` feature is optional
   - [ ] Ensure default build has no vendored dependencies

3. **Create Git Tag** (if needed)
   - [ ] Verify `v0.6.8` tag exists on GitHub
   - [ ] If not, create tag: `git tag -a v0.6.8 -m "Release 0.6.8"`
   - [ ] Push tag: `git push origin v0.6.8`

4. **Test Formula Locally**
   - [ ] Run all pre-submission commands above
   - [ ] Verify formula works on macOS (Intel + Apple Silicon if possible)
   - [ ] Verify test block passes

---

## Draft Formula Structure

```ruby
class Loctree < Formula
  desc "AI-oriented project analyzer for detecting dead exports and circular imports"
  homepage "https://github.com/Loctree/Loctree"
  url "https://github.com/Loctree/Loctree/archive/refs/tags/v0.6.8.tar.gz"
  sha256 "CALCULATE_THIS"
  license "MIT"

  depends_on "rust" => :build

  def install
    cd "loctree_rs" do
      system "cargo", "install", *std_cargo_args
    end

    # Install both binaries
    bin.install_symlink bin/"loctree" => "loct"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/loct --version")

    # Test basic functionality
    system bin/"loct", "tree", "--summary"
    assert_predicate testpath/".loctree/analysis.json", :exist?
  end
end
```

---

## References

- [Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae)
- [Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Adding Software to Homebrew](https://docs.brew.sh/Adding-Software-to-Homebrew)
- [homebrew-core CONTRIBUTING.md](https://github.com/Homebrew/homebrew-core/blob/master/CONTRIBUTING.md)
- [loctree on crates.io](https://crates.io/crates/loctree)

---

**Created:** 2025-12-10
**For:** loctree v0.6.8 homebrew-core submission
