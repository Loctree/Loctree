# This is a reference Homebrew formula for loctree
# For the initial submission to homebrew-core, or for creating a custom tap
#
# Usage for initial submission:
#   1. Update version and sha256 for the target release
#   2. Test locally: brew install --build-from-source ./Formula/loctree.rb
#   3. Submit: brew bump-formula-pr loctree --url=... --sha256=...
#
# This file is NOT the official formula - that lives in Homebrew/homebrew-core

class Loctree < Formula
  desc "Fast, language-aware codebase analyzer for detecting dead exports and circular imports"
  homepage "https://loctree.io"
  # NOTE: Update URL and SHA256 before submitting to homebrew-core
  # Use: curl -sL "https://crates.io/api/v1/crates/loctree/0.8.4/download" | shasum -a 256
  url "https://crates.io/api/v1/crates/loctree/0.8.4/download"
  sha256 "UPDATE_SHA256_BEFORE_RELEASE"
  license "MIT"
  head "https://github.com/Loctree/Loctree-suite.git", branch: "main"

  # Binary name is 'loctree' and 'loct' (alias)
  # Installed from: loctree_rs/src/bin/

  depends_on "rust" => :build

  def install
    # Build the loctree binary from the loctree_rs workspace member
    system "cargo", "install", *std_cargo_args(path: "loctree_rs")
  end

  test do
    # Test basic functionality
    (testpath/"test.js").write <<~EOS
      export function hello() {
        return "world";
      }
    EOS

    # Run loctree on the test file
    output = shell_output("#{bin}/loctree #{testpath}")
    assert_match "test.js", output

    # Test version flag
    assert_match version.to_s, shell_output("#{bin}/loctree --version")

    # Test help flag
    assert_match "Fast, language-aware codebase analyzer", shell_output("#{bin}/loctree --help")
  end
end
