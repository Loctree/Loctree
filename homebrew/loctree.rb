class Loctree < Formula
  desc "AI-oriented project analyzer for semantic code analysis"
  homepage "https://github.com/m-szymanska/loctree"
  url "https://static.crates.io/crates/loctree/loctree-0.6.10.crate"
  sha256 "b97228ccf82ed224c2ccdf43010baab280cb0cc7a29004928bcf430472727d7e"
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
