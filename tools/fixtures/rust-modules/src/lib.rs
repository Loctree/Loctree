//! Test fixture for Rust module patterns
//! Tests that `mod foo;` declarations are not treated as imports

// Module declarations - these should NOT create import edges
mod config;

// Re-export - this SHOULD be recognized as a reexport, not dead code
pub use config::Config;
pub use config::validation::validate;

/// Main entry point
pub fn init() -> Config {
    Config::default()
}
