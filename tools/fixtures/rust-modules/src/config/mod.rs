//! Configuration module
//! This file uses `mod validation;` which should NOT create a cycle

// Declare submodule - this is a "mod" edge, not an "import" edge
mod validation;

// Re-export from submodule - this should be recognized as reexport
pub use validation::validate;

/// Configuration struct
#[derive(Debug, Default)]
pub struct Config {
    pub debug: bool,
}

impl Config {
    /// Validate the configuration
    pub fn is_valid(&self) -> bool {
        validation::validate(self)
    }
}
