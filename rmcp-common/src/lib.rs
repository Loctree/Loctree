//! Shared types for rmcp-mux and rmcp-memex crates.
//!
//! This crate provides common types used by both rmcp-mux and rmcp-memex,
//! including host detection types and configuration formats.

mod host;

pub use host::{HostFormat, HostKind};
