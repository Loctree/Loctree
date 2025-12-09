//! Tests for the runtime module.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tempfile::tempdir;
use tokio::net::UnixListener;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::{Mutex, Semaphore, watch};

use crate::config::{
    CliOptions, Config, ResolvedParams, ServerConfig, expand_path, load_config, resolve_params,
};
use crate::state::{
    MuxState, Pending, ServerStatus, StatusSnapshot, error_response, publish_status, reset_state,
    set_id, snapshot_for_state,
};

use super::client::handle_client_message;
use super::health_check;
use super::server::handle_server_message;
use super::status::spawn_status_writer;

/// Test CLI options struct for resolve_params tests
#[derive(Default)]
struct TestCli {
    socket: Option<PathBuf>,
    cmd: Option<String>,
    args: Vec<String>,
    max_active_clients: usize,
    lazy_start: Option<bool>,
    max_request_bytes: Option<usize>,
    request_timeout_ms: Option<u64>,
    restart_backoff_ms: Option<u64>,
    restart_backoff_max_ms: Option<u64>,
    max_restarts: Option<u64>,
    log_level: String,
    tray: bool,
    service_name: Option<String>,
    service: Option<String>,
    status_file: Option<PathBuf>,
}

impl CliOptions for TestCli {
    fn socket(&self) -> Option<PathBuf> {
        self.socket.clone()
    }
    fn cmd(&self) -> Option<String> {
        self.cmd.clone()
    }
    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
    fn max_active_clients(&self) -> usize {
        self.max_active_clients
    }
    fn lazy_start(&self) -> Option<bool> {
        self.lazy_start
    }
    fn max_request_bytes(&self) -> Option<usize> {
        self.max_request_bytes
    }
    fn request_timeout_ms(&self) -> Option<u64> {
        self.request_timeout_ms
    }
    fn restart_backoff_ms(&self) -> Option<u64> {
        self.restart_backoff_ms
    }
    fn restart_backoff_max_ms(&self) -> Option<u64> {
        self.restart_backoff_max_ms
    }
    fn max_restarts(&self) -> Option<u64> {
        self.max_restarts
    }
    fn log_level(&self) -> String {
        self.log_level.clone()
    }
    fn tray(&self) -> bool {
        self.tray
    }
    fn service_name(&self) -> Option<String> {
        self.service_name.clone()
    }
    fn service(&self) -> Option<String> {
        self.service.clone()
    }
    fn status_file(&self) -> Option<PathBuf> {
        self.status_file.clone()
    }
}

fn test_state_with_max(max: usize) -> Arc<Mutex<MuxState>> {
    Arc::new(Mutex::new(MuxState::new(
        max,
        "test".into(),
        1_048_576,
        Duration::from_secs(30),
        Duration::from_secs(1),
        Duration::from_secs(30),
        5,
        0,
        None,
    )))
}

fn test_state() -> Arc<Mutex<MuxState>> {
    test_state_with_max(5)
}

fn capture_client(state: &mut MuxState) -> (u64, UnboundedReceiver<Value>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let id = state.register_client(tx);
    (id, rx)
}

fn tmp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    env::temp_dir().join(format!("{}-{}", name, nanos))
}

fn params_with_socket(socket: PathBuf) -> ResolvedParams {
    ResolvedParams {
        socket,
        cmd: "echo".into(),
        args: vec![],
        max_clients: 5,
        tray_enabled: false,
        log_level: "info".into(),
        service_name: "test".into(),
        lazy_start: false,
        max_request_bytes: 1_048_576,
        request_timeout: Duration::from_secs(30),
        restart_backoff: Duration::from_secs(1),
        restart_backoff_max: Duration::from_secs(30),
        max_restarts: 3,
        status_file: None,
    }
}

#[tokio::test]
async fn set_id_updates_object() {
    let mut obj = serde_json::json!({"id": "old"});
    set_id(&mut obj, Value::String("new".into()));
    assert_eq!(obj.get("id"), Some(&Value::String("new".into())));
}

#[tokio::test]
async fn error_response_has_code_and_message() {
    let resp = error_response(Value::Number(1.into()), "boom");
    assert_eq!(resp.get("id"), Some(&Value::Number(1.into())));
    assert_eq!(
        resp.get("error").and_then(|e| e.get("message")),
        Some(&Value::String("boom".into()))
    );
}

#[tokio::test]
async fn initialize_response_is_cached_and_fanned_out() {
    let state = test_state();
    let active_clients = Arc::new(Semaphore::new(5));
    let (status_tx, _status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };
    let mut st = state.lock().await;
    let (cid1, mut rx1) = capture_client(&mut st);
    let (cid2, mut rx2) = capture_client(&mut st);

    st.pending.insert(
        "g1".into(),
        Pending {
            client_id: cid1,
            local_id: Value::String("loc1".into()),
            is_initialize: true,
            started_at: Instant::now(),
        },
    );
    st.init_waiting.push((cid2, Value::String("loc2".into())));
    st.initializing = true;
    drop(st);

    let server_msg = serde_json::json!({
        "id": "g1",
        "result": { "ok": true }
    });
    assert!(
        handle_server_message(server_msg, &state, &active_clients, &status_tx)
            .await
            .is_ok()
    );

    let m1 = rx1.recv().await.expect("client1 message");
    assert_eq!(m1.get("id"), Some(&Value::String("loc1".into())));

    let m2 = rx2.recv().await.expect("client2 message");
    assert_eq!(m2.get("id"), Some(&Value::String("loc2".into())));

    let st = state.lock().await;
    assert!(st.cached_initialize.is_some());
    assert!(!st.initializing);
    assert!(st.init_waiting.is_empty());
}

#[tokio::test]
async fn non_initialize_response_routed_without_caching() {
    let state = test_state();
    let active_clients = Arc::new(Semaphore::new(5));
    let (status_tx, _status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };
    let mut st = state.lock().await;
    let (cid1, mut rx1) = capture_client(&mut st);
    st.pending.insert(
        "g2".into(),
        Pending {
            client_id: cid1,
            local_id: Value::Number(7.into()),
            is_initialize: false,
            started_at: Instant::now(),
        },
    );
    drop(st);

    let server_msg = serde_json::json!({"id": "g2", "result": 123});
    assert!(
        handle_server_message(server_msg, &state, &active_clients, &status_tx)
            .await
            .is_ok()
    );

    let msg = rx1.recv().await.expect("client message");
    assert_eq!(msg.get("id"), Some(&Value::Number(7.into())));
    let st = state.lock().await;
    assert!(st.cached_initialize.is_none());
}

#[tokio::test]
async fn reset_state_broadcasts_errors() {
    let state = test_state();
    let active_clients = Arc::new(Semaphore::new(5));
    let (status_tx, _status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };
    let mut st = state.lock().await;
    let (cid1, mut rx1) = capture_client(&mut st);
    let (cid2, mut rx2) = capture_client(&mut st);
    st.pending.insert(
        "g3".into(),
        Pending {
            client_id: cid1,
            local_id: Value::Number(1.into()),
            is_initialize: false,
            started_at: Instant::now(),
        },
    );
    st.init_waiting.push((cid2, Value::Number(2.into())));
    drop(st);

    reset_state(&state, "reset", &active_clients, &status_tx).await;

    let m1 = rx1.recv().await.expect("pending error");
    let m2 = rx2.recv().await.expect("waiter error");
    assert_eq!(
        m1.get("error").and_then(|e| e.get("message")),
        Some(&Value::String("reset".into()))
    );
    assert_eq!(
        m2.get("error").and_then(|e| e.get("message")),
        Some(&Value::String("reset".into()))
    );
}

#[test]
fn expand_path_expands_home() {
    let home = tmp_path("home-test");
    fs::create_dir_all(&home).expect("create home temp dir");
    // SAFETY: Test runs single-threaded, no concurrent access to env
    unsafe { env::set_var("HOME", &home) };
    let expanded = expand_path("~/socket.sock");
    assert!(expanded.starts_with(&home));
}

#[test]
fn load_config_parses_json_yaml_toml() {
    let base = tmp_path("cfg");
    fs::create_dir_all(&base).expect("create base dir");

    let json_path = base.join("c.json");
    let yaml_path = base.join("c.yaml");
    let toml_path = base.join("c.toml");

    let json = r#"{
  "servers": {
    "s": {"socket": "/tmp/a", "cmd": "npx", "args": ["@mcp"], "max_active_clients": 2, "tray": true, "service_name": "s"}
  }
}"#;
    let yaml = r#"servers:
  s:
    socket: "/tmp/a"
    cmd: "npx"
    args: ["@mcp"]
    max_active_clients: 2
    tray: true
    service_name: "s"
"#;
    let toml = r#"[servers.s]
socket = "/tmp/a"
cmd = "npx"
args = ["@mcp"]
max_active_clients = 2
tray = true
service_name = "s"
"#;

    fs::write(&json_path, json).expect("write json config");
    fs::write(&yaml_path, yaml).expect("write yaml config");
    fs::write(&toml_path, toml).expect("write toml config");

    assert!(load_config(&json_path).unwrap().is_some());
    assert!(load_config(&yaml_path).unwrap().is_some());
    assert!(load_config(&toml_path).unwrap().is_some());
}

#[test]
fn load_config_missing_returns_none() {
    let missing = tmp_path("nope.json");
    assert!(load_config(&missing).unwrap().is_none());
}

#[test]
fn resolve_params_overrides_from_config() {
    let cfg = Config {
        servers: HashMap::from([(
            "svc".into(),
            ServerConfig {
                socket: Some("/tmp/override.sock".into()),
                cmd: Some("npx".into()),
                args: Some(vec!["@mcp".into()]),
                max_active_clients: Some(7),
                tray: Some(true),
                service_name: Some("svc-name".into()),
                log_level: Some("debug".into()),
                lazy_start: Some(false),
                max_request_bytes: Some(1_048_576),
                request_timeout_ms: Some(30_000),
                restart_backoff_ms: Some(1_000),
                restart_backoff_max_ms: Some(30_000),
                max_restarts: Some(5),
                status_file: None,
            },
        )]),
    };

    let cli = TestCli {
        socket: None,
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: Some("svc".into()),
        status_file: None,
    };

    let params = resolve_params(&cli, Some(&cfg)).expect("resolve params from config");
    assert_eq!(params.socket, PathBuf::from("/tmp/override.sock"));
    assert_eq!(params.cmd, "npx");
    assert_eq!(params.args, vec!["@mcp".to_string()]);
    assert_eq!(params.max_clients, 7);
    assert!(params.tray_enabled);
    assert_eq!(params.service_name, "svc-name");
    assert_eq!(params.log_level, "debug");
}

#[test]
fn resolve_params_requires_service_with_config() {
    let cfg = Config {
        servers: HashMap::new(),
    };
    let cli = TestCli {
        socket: None,
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: None,
        status_file: None,
    };
    let err = resolve_params(&cli, Some(&cfg)).unwrap_err();
    assert!(err.to_string().contains("--service is required"));
}

#[test]
fn resolve_params_cli_overrides_socket() {
    let cfg = Config {
        servers: HashMap::from([(
            "svc".into(),
            ServerConfig {
                socket: Some("/tmp/cfg.sock".into()),
                cmd: Some("npx".into()),
                args: None,
                max_active_clients: None,
                tray: None,
                service_name: None,
                log_level: None,
                lazy_start: None,
                max_request_bytes: None,
                request_timeout_ms: None,
                restart_backoff_ms: None,
                restart_backoff_max_ms: None,
                max_restarts: None,
                status_file: None,
            },
        )]),
    };
    let cli = TestCli {
        socket: Some(PathBuf::from("/tmp/cli.sock")),
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: Some("svc".into()),
        status_file: None,
    };
    let params = resolve_params(&cli, Some(&cfg)).expect("resolve");
    assert_eq!(params.socket, PathBuf::from("/tmp/cli.sock"));
}

#[test]
fn resolve_params_applies_defaults_without_config() {
    let cli = TestCli {
        socket: Some(PathBuf::from("/tmp/test.sock")),
        cmd: Some("echo".into()),
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: None,
        status_file: None,
    };
    let params = resolve_params(&cli, None).expect("resolve without config");
    assert_eq!(params.socket, PathBuf::from("/tmp/test.sock"));
    assert_eq!(params.cmd, "echo");
    assert_eq!(params.max_clients, 5);
    assert!(!params.tray_enabled);
    assert_eq!(params.log_level, "info");
    assert!(!params.lazy_start);
    assert_eq!(params.max_request_bytes, 1_048_576);
    assert_eq!(params.request_timeout, Duration::from_millis(30_000));
    assert_eq!(params.restart_backoff, Duration::from_millis(1_000));
    assert_eq!(params.restart_backoff_max, Duration::from_millis(30_000));
    assert_eq!(params.max_restarts, 5);
}

#[test]
fn resolve_params_prefers_cli_over_config_for_timeouts() {
    let cfg = Config {
        servers: HashMap::from([(
            "svc".into(),
            ServerConfig {
                socket: Some("/tmp/s.sock".into()),
                cmd: Some("npx".into()),
                args: None,
                max_active_clients: None,
                tray: None,
                service_name: None,
                log_level: None,
                lazy_start: None,
                max_request_bytes: Some(100),
                request_timeout_ms: Some(100),
                restart_backoff_ms: Some(100),
                restart_backoff_max_ms: Some(100),
                max_restarts: Some(100),
                status_file: None,
            },
        )]),
    };
    let cli = TestCli {
        socket: None,
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: Some(true),
        max_request_bytes: Some(999),
        request_timeout_ms: Some(999),
        restart_backoff_ms: Some(999),
        restart_backoff_max_ms: Some(999),
        max_restarts: Some(999),
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: Some("svc".into()),
        status_file: None,
    };
    let params = resolve_params(&cli, Some(&cfg)).expect("resolve");
    assert!(params.lazy_start);
    assert_eq!(params.max_request_bytes, 999);
    assert_eq!(params.request_timeout, Duration::from_millis(999));
    assert_eq!(params.restart_backoff, Duration::from_millis(999));
    assert_eq!(params.restart_backoff_max, Duration::from_millis(999));
    assert_eq!(params.max_restarts, 999);
}

#[test]
fn resolve_params_errors_when_socket_missing() {
    let cli = TestCli {
        socket: None,
        cmd: Some("echo".into()),
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: None,
        status_file: None,
    };
    let err = resolve_params(&cli, None).unwrap_err();
    assert!(err.to_string().contains("socket"));
}

#[test]
fn resolve_params_errors_when_cmd_missing() {
    let cli = TestCli {
        socket: Some(PathBuf::from("/tmp/x.sock")),
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: None,
        status_file: None,
    };
    let err = resolve_params(&cli, None).unwrap_err();
    assert!(err.to_string().contains("cmd"));
}

#[test]
fn resolve_params_errors_when_service_missing_in_config() {
    let cfg = Config {
        servers: HashMap::new(),
    };
    let cli = TestCli {
        socket: None,
        cmd: None,
        args: vec![],
        max_active_clients: 5,
        lazy_start: None,
        max_request_bytes: None,
        request_timeout_ms: None,
        restart_backoff_ms: None,
        restart_backoff_max_ms: None,
        max_restarts: None,
        log_level: "info".into(),
        tray: false,
        service_name: None,
        service: Some("nosuchservice".into()),
        status_file: None,
    };
    let err = resolve_params(&cli, Some(&cfg)).unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn publish_status_counts_active() {
    let state = test_state_with_max(3);
    let active = Arc::new(Semaphore::new(3));
    let (tx, rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };

    let p1 = active.clone().acquire_owned().await.expect("first permit");
    let p2 = active.clone().acquire_owned().await.expect("second permit");
    publish_status(&state, &active, &tx).await;
    let snap = rx.borrow().clone();
    assert_eq!(snap.active_clients, 2);
    drop(p1);
    drop(p2);
}

#[tokio::test]
async fn reset_state_updates_last_reset_and_status() {
    let state = test_state_with_max(2);
    let active = Arc::new(Semaphore::new(2));
    let (status_tx, mut status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };

    let _ = status_rx.borrow().clone();

    reset_state(&state, "restart-test", &active, &status_tx).await;

    status_rx.changed().await.expect("status update");
    let snap = status_rx.borrow().clone();
    assert_eq!(snap.last_reset.as_deref(), Some("restart-test"));
    assert!(!snap.initializing);
    assert_eq!(snap.pending_requests, 0);
}

#[tokio::test]
async fn initialize_served_from_cache_does_not_queue() {
    let state = test_state();
    let active = Arc::new(Semaphore::new(5));
    let (status_tx, _status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };
    let (to_server_tx, mut to_server_rx) = mpsc::channel::<Value>(1);

    let mut st = state.lock().await;
    let (cid, mut rx) = capture_client(&mut st);
    st.cached_initialize = Some(serde_json::json!({"id": "server-init", "result": "ok"}));
    drop(st);

    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "client-init",
        "method": "initialize",
        "params": {}
    });

    let max_req = { state.lock().await.max_request_bytes };
    handle_client_message(
        cid,
        msg,
        &state,
        &to_server_tx,
        &active,
        &status_tx,
        max_req,
    )
    .await
    .expect("handle cached init");

    assert!(to_server_rx.try_recv().is_err());

    let resp = rx.recv().await.expect("cached init response");
    assert_eq!(resp.get("id"), Some(&Value::String("client-init".into())));

    let st = state.lock().await;
    assert!(st.cached_initialize.is_some());
    assert!(!st.initializing);
    assert!(st.pending.is_empty());
}

#[tokio::test]
async fn reset_state_clears_initialize_and_pending() {
    let state = test_state();
    let active = Arc::new(Semaphore::new(5));
    let (status_tx, _status_rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };

    let mut st = state.lock().await;
    let (cid, mut rx) = capture_client(&mut st);
    st.cached_initialize = Some(serde_json::json!({"id": "init", "result": true}));
    st.initializing = true;
    st.pending.insert(
        "g-pending".into(),
        Pending {
            client_id: cid,
            local_id: Value::String("local-id".into()),
            is_initialize: true,
            started_at: Instant::now(),
        },
    );
    st.init_waiting
        .push((cid, Value::String("waiter-id".into())));
    drop(st);

    reset_state(&state, "reset-reason", &active, &status_tx).await;

    let errs: Vec<_> = rx.recv().await.into_iter().collect();
    assert!(!errs.is_empty());

    let st = state.lock().await;
    assert!(st.pending.is_empty());
    assert!(st.init_waiting.is_empty());
    assert!(st.cached_initialize.is_none());
    assert!(!st.initializing);
    assert_eq!(st.last_reset.as_deref(), Some("reset-reason"));
}

#[tokio::test]
async fn publish_status_includes_queue_and_pid() {
    let state = test_state();
    {
        let mut st = state.lock().await;
        st.queue_depth = 7;
        st.child_pid = Some(4242);
    }
    let active = Arc::new(Semaphore::new(5));
    let (tx, rx) = {
        let st = state.lock().await;
        watch::channel(snapshot_for_state(&st, 0))
    };
    publish_status(&state, &active, &tx).await;
    let snap = rx.borrow().clone();
    assert_eq!(snap.queue_depth, 7);
    assert_eq!(snap.child_pid, Some(4242));
    assert_eq!(snap.max_request_bytes, 1_048_576);
    assert_eq!(snap.restart_backoff_ms, 1_000);
    assert_eq!(snap.restart_backoff_max_ms, 30_000);
    assert_eq!(snap.max_restarts, 5);
}

#[tokio::test]
async fn status_file_writer_persists_snapshot() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let path = dir.path().join("status.json");
    let base = StatusSnapshot {
        service_name: "svc".into(),
        server_status: ServerStatus::Starting,
        restarts: 0,
        connected_clients: 0,
        active_clients: 0,
        max_active_clients: 5,
        pending_requests: 0,
        cached_initialize: false,
        initializing: false,
        last_reset: None,
        queue_depth: 0,
        child_pid: Some(99),
        max_request_bytes: 1_048_576,
        restart_backoff_ms: 1_000,
        restart_backoff_max_ms: 30_000,
        max_restarts: 5,
    };
    let (tx, rx) = watch::channel(base.clone());
    let handle = spawn_status_writer(rx, path.clone());

    let mut updated = base.clone();
    updated.queue_depth = 3;
    tx.send(updated.clone()).ok();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let text = fs::read_to_string(&path).expect("status file");
    assert!(text.contains("\"queue_depth\": 3"));
    assert!(text.contains("\"child_pid\": 99"));

    handle.abort();
}

#[tokio::test]
async fn health_check_succeeds_when_socket_listens() {
    let dir = tempdir().expect("tempdir");
    let socket = dir.path().join("health.sock");
    let listener = UnixListener::bind(&socket).expect("bind listener");
    let accept = tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    let params = params_with_socket(socket.clone());
    health_check(&params).await.expect("health ok");
    accept.abort();
}

#[tokio::test]
async fn health_check_fails_for_missing_socket() {
    let dir = tempdir().expect("tempdir");
    let socket = dir.path().join("missing.sock");
    let params = params_with_socket(socket);
    let err = health_check(&params).await.expect_err("should fail");
    assert!(
        err.to_string().contains("failed to connect"),
        "unexpected error: {err}"
    );
}
