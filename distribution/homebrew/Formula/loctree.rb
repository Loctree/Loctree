# This is the source-of-truth Homebrew formula for loctree distribution.
# Validate it here, then sync it to the tap repository.
#
# Usage for initial submission:
#   1. Update version and sha256 for the target release
#   2. Test locally: brew install --build-from-source ./distribution/homebrew/Formula/loctree.rb
#   3. Sync to the tap repo
#
class Loctree < Formula
  desc "Fast, language-aware codebase analyzer for detecting dead exports and circular imports"
  homepage "https://loct.io"
  url "https://crates.io/api/v1/crates/loctree/0.8.15/download"
  sha256 "21dceabcd170ff08c30f0d265bd7e08f4e8c2f2a06e8d6ed5a02dbde48a1074f"
  license any_of: ["MIT", "Apache-2.0"]
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
