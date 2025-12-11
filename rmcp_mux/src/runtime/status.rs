//! Status file writing functionality.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;
use tokio::sync::watch;
use tracing::warn;

use crate::state::StatusSnapshot;

/// Write a status snapshot to a file atomically.
pub async fn write_status_file(path: &Path, snapshot: &StatusSnapshot) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create status dir {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp");
    let data = serde_json::to_vec_pretty(snapshot)?;
    fs::write(&tmp, data)
        .await
        .with_context(|| format!("failed to write status tmp {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .await
        .with_context(|| format!("failed to atomically replace status {}", path.display()))?;
    Ok(())
}

/// Spawn a background task that writes status snapshots to a file whenever they change.
pub fn spawn_status_writer(
    mut rx: watch::Receiver<StatusSnapshot>,
    path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // write initial snapshot
        let mut current = rx.borrow().clone();
        if let Err(e) = write_status_file(&path, &current).await {
            warn!("failed to write initial status file: {e}");
        }
        while rx.changed().await.is_ok() {
            current = rx.borrow().clone();
            if let Err(e) = write_status_file(&path, &current).await {
                warn!("failed to write status file: {e}");
            }
        }
    })
}
