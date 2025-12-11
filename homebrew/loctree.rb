class Loctree < Formula
  desc "AI-oriented project analyzer for semantic code analysis"
  homepage "https://github.com/Loctree/Loctree"
  url "https://static.crates.io/crates/loctree/loctree-0.6.8.crate"
  sha256 "62567bda601553bd3723d80e4cd33ef4cf5a65787063254638c2b711d8ff3951"
  license "MIT"

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
