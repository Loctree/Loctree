class Loctree < Formula
  desc "AI-oriented project analyzer for semantic code analysis"
  homepage "https://loctree.io"
  # Update sha256 before release: curl -sL "https://static.crates.io/crates/loctree/loctree-0.8.11.crate" | shasum -a 256
  url "https://static.crates.io/crates/loctree/loctree-0.8.11.crate"
  sha256 "UPDATE_SHA256_BEFORE_RELEASE"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Verify version output works
    assert_match version.to_s, shell_output("#{bin}/loct --version")

    # Create a simple TypeScript file and run analysis
    (testpath/"test.ts").write("export const foo = 'bar';")
    system bin/"loct", testpath

    # Verify both binaries exist
    assert_predicate bin/"loctree", :exist?
    assert_predicate bin/"loct", :exist?
  end
end
