//! Command handlers split by domain
//!
//! This module organizes command handlers into domain-specific submodules
//! to keep the codebase maintainable.

pub mod ai;
pub mod analysis;
pub mod deprecation;
pub mod diff;
pub mod output;
pub mod query;
pub mod watch;
