//! Validation submodule
//! This imports from parent - a valid Rust pattern that is NOT a cycle

// Import from parent module - this is a use statement going "up" the tree
// This should NOT be flagged as a cycle with mod.rs
use super::Config;

/// Validate configuration
pub fn validate(config: &Config) -> bool {
    // Simple validation
    !config.debug || true
}

/// Another function using Config
pub fn strict_validate(config: &Config) -> Result<(), String> {
    if validate(config) {
        Ok(())
    } else {
        Err("Invalid config".to_string())
    }
}
