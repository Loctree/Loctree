//! STDIO proxy for connecting to the mux socket.

use std::path::PathBuf;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use rmcp::transport::async_rw::JsonRpcMessageCodec;
use serde_json::Value;
use tokio::io::{stdin, stdout};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::warn;

/// Proxy STDIO to the mux socket using JSON-RPC framing.
pub async fn run_proxy(socket: PathBuf) -> Result<()> {
    let stream = UnixStream::connect(&socket)
        .await
        .with_context(|| format!("failed to connect to {}", socket.display()))?;
    let (sr, sw) = stream.into_split();
    let mut sock_reader = FramedRead::new(sr, JsonRpcMessageCodec::<Value>::new());
    let mut sock_writer = FramedWrite::new(sw, JsonRpcMessageCodec::<Value>::new());
    let mut stdin_reader = FramedRead::new(stdin(), JsonRpcMessageCodec::<Value>::new());
    let mut stdout_writer = FramedWrite::new(stdout(), JsonRpcMessageCodec::<Value>::new());

    let to_socket = async {
        while let Some(msg) = stdin_reader.next().await {
            let msg = match msg {
                Ok(v) => v,
                Err(e) => {
                    warn!("stdin decode error: {e}");
                    break;
                }
            };
            if let Err(e) = sock_writer.send(msg).await {
                warn!("socket write error: {e}");
                break;
            }
        }
    };

    let to_stdout = async {
        while let Some(msg) = sock_reader.next().await {
            let msg = match msg {
                Ok(v) => v,
                Err(e) => {
                    warn!("socket decode error: {e}");
                    break;
                }
            };
            if let Err(e) = stdout_writer.send(msg).await {
                warn!("stdout write error: {e}");
                break;
            }
        }
    };

    tokio::select! {
        _ = to_socket => {},
        _ = to_stdout => {},
    }

    Ok(())
}
