//! TUI Configuration Wizard for rmcp_memex
//!
//! Interactive terminal UI for configuring MCP server and host integrations.

mod app;
mod host_detection;
mod ui;

pub use app::{WizardConfig, run_wizard};
pub use host_detection::{HostDetection, HostFormat, HostKind, detect_hosts};
