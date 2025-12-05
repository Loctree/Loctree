use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, Semaphore, mpsc, watch};

#[cfg_attr(not(feature = "tray"), allow(dead_code))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerStatus {
    Starting,
    Running,
    Restarting,
    Failed(String),
    Stopped,
}

#[cfg_attr(not(feature = "tray"), allow(dead_code))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub service_name: String,
    pub server_status: ServerStatus,
    pub restarts: u64,
    pub connected_clients: usize,
    pub active_clients: usize,
    pub max_active_clients: usize,
    pub pending_requests: usize,
    pub cached_initialize: bool,
    pub initializing: bool,
    pub last_reset: Option<String>,
    pub queue_depth: usize,
    pub child_pid: Option<u32>,
    pub max_request_bytes: usize,
    pub restart_backoff_ms: u64,
    pub restart_backoff_max_ms: u64,
    pub max_restarts: u64,
}

/// Central runtime state shared between the async mux loops.
///
/// - `queue_depth` caps queued client messages to avoid unbounded memory growth
///   under bursty hosts.
/// - `max_request_bytes` and `request_timeout` are enforced per forwarded
///   request to prevent slowloris/DoS patterns.
/// - Restart backoff (`restart_backoff`..`restart_backoff_max`) and
///   `max_restarts` gate child respawns so a flapping server cannot burn CPU.
#[derive(Clone)]
pub struct MuxState {
    pub next_client_id: u64,
    pub next_global_id: u64,
    pub clients: HashMap<u64, mpsc::UnboundedSender<Value>>,
    pub pending: HashMap<String, Pending>,
    pub cached_initialize: Option<Value>,
    pub init_waiting: Vec<(u64, Value)>,
    pub initializing: bool,
    pub server_status: ServerStatus,
    pub restarts: u64,
    pub last_reset: Option<String>,
    pub max_active_clients: usize,
    pub service_name: String,
    pub max_request_bytes: usize,
    pub request_timeout: Duration,
    pub restart_backoff: Duration,
    pub restart_backoff_max: Duration,
    pub max_restarts: u64,
    pub queue_depth: usize,
    pub child_pid: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub client_id: u64,
    pub local_id: Value,
    pub is_initialize: bool,
    pub started_at: std::time::Instant,
}

impl MuxState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_active_clients: usize,
        service_name: String,
        max_request_bytes: usize,
        request_timeout: Duration,
        restart_backoff: Duration,
        restart_backoff_max: Duration,
        max_restarts: u64,
        queue_depth: usize,
        child_pid: Option<u32>,
    ) -> Self {
        Self {
            next_client_id: 1,
            next_global_id: 1,
            clients: HashMap::new(),
            pending: HashMap::new(),
            cached_initialize: None,
            init_waiting: Vec::new(),
            initializing: false,
            server_status: ServerStatus::Starting,
            restarts: 0,
            last_reset: None,
            max_active_clients,
            service_name,
            max_request_bytes,
            request_timeout,
            restart_backoff,
            restart_backoff_max,
            max_restarts,
            queue_depth,
            child_pid,
        }
    }

    pub fn register_client(&mut self, tx: mpsc::UnboundedSender<Value>) -> u64 {
        let id = self.next_client_id;
        self.next_client_id += 1;
        self.clients.insert(id, tx);
        id
    }

    pub fn unregister_client(&mut self, client_id: u64) {
        self.clients.remove(&client_id);
        self.pending.retain(|_, p| p.client_id != client_id);
        self.init_waiting.retain(|(cid, _)| *cid != client_id);
    }

    pub fn next_request_id(&mut self) -> u64 {
        let id = self.next_global_id;
        self.next_global_id += 1;
        id
    }
}

pub fn set_id(msg: &mut Value, id: Value) {
    if let Some(obj) = msg.as_object_mut() {
        obj.insert("id".to_string(), id);
    }
}

pub fn error_response(id: Value, message: &str) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32000,
            "message": message,
        }
    })
}

pub fn snapshot_for_state(st: &MuxState, active_clients: usize) -> StatusSnapshot {
    StatusSnapshot {
        service_name: st.service_name.clone(),
        server_status: st.server_status.clone(),
        restarts: st.restarts,
        connected_clients: st.clients.len(),
        active_clients,
        max_active_clients: st.max_active_clients,
        pending_requests: st.pending.len(),
        cached_initialize: st.cached_initialize.is_some(),
        initializing: st.initializing,
        last_reset: st.last_reset.clone(),
        queue_depth: st.queue_depth,
        child_pid: st.child_pid,
        max_request_bytes: st.max_request_bytes,
        restart_backoff_ms: st.restart_backoff.as_millis() as u64,
        restart_backoff_max_ms: st.restart_backoff_max.as_millis() as u64,
        max_restarts: st.max_restarts,
    }
}

pub async fn publish_status(
    state: &Arc<Mutex<MuxState>>,
    active_clients: &Arc<Semaphore>,
    status_tx: &watch::Sender<StatusSnapshot>,
) {
    let st = state.lock().await;
    let active = st
        .max_active_clients
        .saturating_sub(active_clients.available_permits());
    let snapshot = snapshot_for_state(&st, active);
    drop(st);
    let _ = status_tx.send(snapshot);
}

pub async fn reset_state(
    state: &Arc<Mutex<MuxState>>,
    reason: &str,
    active_clients: &Arc<Semaphore>,
    status_tx: &watch::Sender<StatusSnapshot>,
) {
    let mut st = state.lock().await;
    let pending = std::mem::take(&mut st.pending);
    let waiters = std::mem::take(&mut st.init_waiting);
    st.cached_initialize = None;
    st.initializing = false;
    st.last_reset = Some(reason.to_string());
    st.queue_depth = 0;
    st.child_pid = None;

    for (_, p) in pending {
        if let Some(tx) = st.clients.get(&p.client_id) {
            tx.send(error_response(p.local_id, reason)).ok();
        }
    }
    for (cid, lid) in waiters {
        if let Some(tx) = st.clients.get(&cid) {
            tx.send(error_response(lid, reason)).ok();
        }
    }
    drop(st);
    publish_status(state, active_clients, status_tx).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_request_id_increments_sequentially() {
        let mut state = MuxState::new(
            5,
            "test-service".into(),
            1_048_576,
            Duration::from_secs(30),
            Duration::from_millis(1_000),
            Duration::from_millis(30_000),
            5,
            0,
            None,
        );

        let first = state.next_request_id();
        let second = state.next_request_id();

        assert_eq!(first + 1, second);
    }

    #[test]
    fn error_response_uses_jsonrpc_2_0() {
        let resp = error_response(Value::Number(1.into()), "oops");
        assert_eq!(resp.get("jsonrpc"), Some(&Value::String("2.0".into())));
    }
}
