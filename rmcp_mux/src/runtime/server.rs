//! Server (MCP child process) management.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures::{SinkExt, StreamExt};
use rmcp::transport::async_rw::JsonRpcMessageCodec;
use serde_json::Value;
use tokio::process::Command as TokioCommand;
use tokio::sync::{Mutex, Semaphore, mpsc, watch};
use tokio::time::sleep;
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::state::{MuxState, ServerStatus, StatusSnapshot, publish_status, reset_state, set_id};

use super::client::update_queue_depth;
use super::types::ServerEvent;

/// Handle events from the server (messages and resets).
pub async fn handle_server_events(
    state: Arc<Mutex<MuxState>>,
    active_clients: Arc<Semaphore>,
    status_tx: watch::Sender<StatusSnapshot>,
    mut rx: mpsc::UnboundedReceiver<ServerEvent>,
) {
    while let Some(evt) = rx.recv().await {
        match evt {
            ServerEvent::Message(msg) => {
                if let Err(e) =
                    handle_server_message(msg, &state, &active_clients, &status_tx).await
                {
                    warn!("server message routing failed: {e}");
                }
            }
            ServerEvent::Reset(reason) => {
                reset_state(&state, &reason, &active_clients, &status_tx).await;
            }
        }
    }
}

/// Handle a single message from the server.
pub async fn handle_server_message(
    msg: Value,
    state: &Arc<Mutex<MuxState>>,
    active_clients: &Arc<Semaphore>,
    status_tx: &watch::Sender<StatusSnapshot>,
) -> Result<()> {
    if msg.get("id").is_none() {
        // notification -> broadcast
        let st = state.lock().await;
        for tx in st.clients.values() {
            tx.send(msg.clone()).ok();
        }
        return Ok(());
    }

    let id_val = msg
        .get("id")
        .cloned()
        .ok_or_else(|| anyhow!("missing id in server response"))?;
    let id_str = id_val
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| id_val.as_i64().map(|n| n.to_string()))
        .ok_or_else(|| anyhow!("unsupported id type"))?;

    let pending = {
        let mut st = state.lock().await;
        st.pending.remove(&id_str)
    };

    let Some(pending) = pending else {
        warn!("no pending request for id {id_str}");
        return Ok(());
    };

    let target_tx = {
        let st = state.lock().await;
        st.clients.get(&pending.client_id).cloned()
    };

    if let Some(tx) = target_tx {
        let mut resp = msg.clone();
        set_id(&mut resp, pending.local_id.clone());
        let is_init = pending.is_initialize;
        tx.send(resp.clone()).ok();

        if is_init {
            let mut st = state.lock().await;
            st.cached_initialize = Some(resp.clone());
            st.initializing = false;
            // Respond to waiting initialize callers
            let waiters = std::mem::take(&mut st.init_waiting);
            for (cid, lid) in waiters {
                if let Some(wait_tx) = st.clients.get(&cid) {
                    let mut clone_resp = resp.clone();
                    set_id(&mut clone_resp, lid);
                    wait_tx.send(clone_resp).ok();
                }
            }
        }
    }
    publish_status(state, active_clients, status_tx).await;
    Ok(())
}

/// Manage the MCP server child process with restart logic.
#[allow(clippy::too_many_arguments)]
pub async fn server_manager(
    cmd: String,
    args: Vec<String>,
    mut to_server_rx: mpsc::Receiver<Value>,
    to_server_meter: mpsc::Sender<Value>,
    server_events_tx: mpsc::UnboundedSender<ServerEvent>,
    state: Arc<Mutex<MuxState>>,
    active_clients: Arc<Semaphore>,
    status_tx: watch::Sender<StatusSnapshot>,
    shutdown: CancellationToken,
    lazy_start: bool,
    restart_backoff: Duration,
    restart_backoff_max: Duration,
    max_restarts: u64,
) -> Result<()> {
    let mut backoff = restart_backoff;
    let mut restarts = 0u64;

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        let mut first_msg: Option<Value> = None;
        if lazy_start && restarts == 0 {
            info!("lazy start enabled; waiting for first client message");
            let first = tokio::select! {
                _ = shutdown.cancelled() => None,
                msg = to_server_rx.recv() => msg,
            };
            if shutdown.is_cancelled() {
                break;
            }
            if let Some(msg) = first {
                first_msg = Some(msg);
                update_queue_depth(&state, &to_server_meter).await;
            } else {
                break;
            }
        }

        if max_restarts > 0 && restarts >= max_restarts {
            let mut st = state.lock().await;
            st.server_status = ServerStatus::Failed("max restarts reached".into());
            st.last_reset = Some("max restarts reached".into());
            st.child_pid = None;
            publish_status(&state, &active_clients, &status_tx).await;
            break;
        }

        info!("starting MCP server: {} {:?}", cmd, args);
        let mut child = TokioCommand::new(&cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("failed to spawn MCP server")?;

        {
            let mut st = state.lock().await;
            st.server_status = ServerStatus::Running;
            st.child_pid = child.id();
        }
        publish_status(&state, &active_clients, &status_tx).await;

        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture stdin"))?;
        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture stdout"))?;

        let mut writer = FramedWrite::new(child_stdin, JsonRpcMessageCodec::<Value>::new());
        let mut reader = FramedRead::new(child_stdout, JsonRpcMessageCodec::<Value>::new());

        let reader_task = {
            let server_events_tx = server_events_tx.clone();
            tokio::spawn(async move {
                loop {
                    let next = reader.next().await;
                    match next {
                        Some(Ok(msg)) => {
                            if server_events_tx.send(ServerEvent::Message(msg)).is_err() {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("server reader error: {e}");
                            break;
                        }
                        None => {
                            warn!("server stdout closed");
                            break;
                        }
                    }
                }
            })
        };

        let server_events_tx_clone = server_events_tx.clone();
        let mut child_wait = tokio::spawn(async move { child.wait().await });

        // write loop and monitor
        let mut should_restart = true;
        if let Some(msg) = first_msg.take()
            && let Err(e) = writer.send(msg).await
        {
            warn!("write to server failed on first message: {e}");
        }
        while !shutdown.is_cancelled() {
            tokio::select! {
                maybe_msg = to_server_rx.recv() => {
                    let Some(msg) = maybe_msg else {
                        update_queue_depth(&state, &to_server_meter).await;
                        should_restart = false;
                        break;
                    };
                    update_queue_depth(&state, &to_server_meter).await;
                    if let Err(e) = writer.send(msg).await {
                        warn!("write to server failed: {e}");
                        {
                            let mut st = state.lock().await;
                            st.server_status = ServerStatus::Failed(e.to_string());
                            st.last_reset = Some("write failure".into());
                        }
                        publish_status(&state, &active_clients, &status_tx).await;
                        break;
                    }
                }
                status = &mut child_wait => {
                    match status {
                        Ok(Ok(status)) => warn!("server exited with status {status}"),
                        Ok(Err(e)) => {
                            warn!("server wait error: {e}");
                            let mut st = state.lock().await;
                            st.server_status = ServerStatus::Failed(e.to_string());
                            st.last_reset = Some("wait error".into());
                        }
                        Err(join_err) => {
                            warn!("server wait join error: {join_err}");
                        }
                    }
                    publish_status(&state, &active_clients, &status_tx).await;
                    break;
                }
                _ = shutdown.cancelled() => { break; }
            }
        }

        // child cleanup
        child_wait.abort();
        reader_task.abort();

        // reset state
        server_events_tx_clone
            .send(ServerEvent::Reset("MCP server restarted".into()))
            .ok();
        {
            let mut st = state.lock().await;
            st.cached_initialize = None;
            st.initializing = false;
            st.child_pid = None;
            if shutdown.is_cancelled() || !should_restart {
                st.server_status = ServerStatus::Stopped;
            } else {
                st.server_status = ServerStatus::Restarting;
                st.restarts = st.restarts.saturating_add(1);
            }
        }
        update_queue_depth(&state, &to_server_meter).await;
        publish_status(&state, &active_clients, &status_tx).await;

        if shutdown.is_cancelled() || !should_restart {
            break;
        }
        restarts = restarts.saturating_add(1);
        info!("restarting MCP server after failure, backoff {:?}", backoff);
        sleep(backoff).await;
        backoff = (backoff * 2).min(restart_backoff_max);
    }

    Ok(())
}
