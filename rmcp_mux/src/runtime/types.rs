//! Common types and constants for the runtime module.

use serde_json::Value;

/// Maximum queued messages before rejecting new client requests.
pub const MAX_QUEUE: usize = 1024;

/// Maximum pending requests before rejecting new client requests.
pub const MAX_PENDING: usize = 2048;

/// Events from the server to be processed by the router.
pub enum ServerEvent {
    Message(Value),
    Reset(String),
}
