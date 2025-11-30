{ pkgs }: {
  deps = [
    # Use rustup instead of system Rust - allows rust-toolchain.toml to work
    pkgs.rustup

    # Trunk (Leptos/WASM bundler)
    pkgs.trunk

    # Build dependencies
    pkgs.openssl
    pkgs.pkg-config
    pkgs.libiconv

    # For serving static files
    pkgs.simple-http-server
  ];

  env = {
    RUST_BACKTRACE = "1";
    # Ensure rustup uses project toolchain
    RUSTUP_TOOLCHAIN = "";
  };
}
