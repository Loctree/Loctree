fn main() {
    // Provide a bundled `protoc` for dependencies that expect it (e.g. lancedb/lance).
    // This avoids requiring users to install protobuf-compiler system-wide.
    if let Ok(path) = protoc_bin_vendored::protoc_bin_path() {
        // Propagate to dependent build scripts.
        println!("cargo:rustc-env=PROTOC={}", path.display());
    }
    println!("cargo:rerun-if-changed=src/build.rs");
}
