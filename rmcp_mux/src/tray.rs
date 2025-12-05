use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use image::ImageFormat;
use tokio_util::sync::CancellationToken;
use tray_icon::{
    Icon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::state::{ServerStatus, StatusSnapshot};

#[derive(Clone, Debug)]
pub(crate) struct LoadedIcon {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(crate) fn spawn_tray(
    status_rx: tokio::sync::watch::Receiver<StatusSnapshot>,
    shutdown: CancellationToken,
    icon: Option<LoadedIcon>,
) -> thread::JoinHandle<()> {
    // Keep only the latest snapshot to avoid unbounded growth if the tray UI lags.
    let (snap_tx, snap_rx) = bounded(1);
    let (stop_tx, stop_rx) = bounded(1);

    // Most recent snapshot forwarded to blocking tray thread.
    tokio::spawn(async move {
        let mut rx = status_rx;
        send_latest(&snap_tx, rx.borrow().clone());
        while rx.changed().await.is_ok() {
            send_latest(&snap_tx, rx.borrow().clone());
        }
    });

    // Stop signal when shutdown is requested.
    tokio::spawn({
        let stop_tx = stop_tx.clone();
        let shutdown = shutdown.clone();
        async move {
            shutdown.cancelled().await;
            let _ = stop_tx.send(());
        }
    });

    thread::spawn(move || tray_loop(snap_rx, stop_rx, shutdown, icon))
}

fn send_latest(tx: &Sender<StatusSnapshot>, snap: StatusSnapshot) {
    match tx.try_send(snap) {
        Ok(_) => {}
        Err(TrySendError::Full(_)) => {
            // Channel holds a single snapshot; if full, drop the new one.
        }
        Err(TrySendError::Disconnected(_)) => {}
    }
}

struct TrayUi {
    _tray: tray_icon::TrayIcon,
    header: MenuItem,
    status: MenuItem,
    clients: MenuItem,
    pending: MenuItem,
    init_state: MenuItem,
    restarts: MenuItem,
    quit_id: MenuId,
}

impl TrayUi {
    fn update(&self, snapshot: &StatusSnapshot) {
        self.header
            .set_text(format!("Service: {}", snapshot.service_name));
        self.status.set_text(status_line(snapshot));
        self.clients.set_text(client_line(snapshot));
        self.pending.set_text(pending_line(snapshot));
        self.init_state.set_text(init_line(snapshot));
        self.restarts.set_text(restart_line(snapshot));
    }
}

fn tray_loop(
    snap_rx: Receiver<StatusSnapshot>,
    stop_rx: Receiver<()>,
    shutdown: CancellationToken,
    icon: Option<LoadedIcon>,
) {
    let mut current = match snap_rx.recv() {
        Ok(s) => s,
        Err(_) => return,
    };

    let ui = match build_tray(&current, icon.as_ref()) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("tray init failed: {e}");
            return;
        }
    };

    loop {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == ui.quit_id {
                shutdown.cancel();
                return;
            }
        }

        crossbeam_channel::select! {
            recv(stop_rx) -> _ => { return; }
            recv(snap_rx) -> msg => {
                match msg {
                    Ok(snap) => {
                        current = snap;
                        ui.update(&current);
                    }
                    Err(_) => return,
                }
            }
            default(Duration::from_millis(150)) => {}
        }
    }
}

fn build_tray(snapshot: &StatusSnapshot, icon_data: Option<&LoadedIcon>) -> Result<TrayUi> {
    let menu = Menu::new();
    let header = MenuItem::new(format!("Service: {}", snapshot.service_name), false, None);
    let status_item = MenuItem::new(status_line(snapshot), false, None);
    let clients_item = MenuItem::new(client_line(snapshot), false, None);
    let pending_item = MenuItem::new(pending_line(snapshot), false, None);
    let init_item = MenuItem::new(init_line(snapshot), false, None);
    let restart_item = MenuItem::new(restart_line(snapshot), false, None);
    let quit_item = MenuItem::new("Quit mux", true, None);
    let sep = PredefinedMenuItem::separator();

    for item in [
        &header,
        &status_item,
        &clients_item,
        &pending_item,
        &init_item,
        &restart_item,
    ] {
        menu.append(item)?;
    }
    menu.append(&sep)?;
    menu.append(&quit_item)?;

    let icon = if let Some(data) = icon_data {
        Icon::from_rgba(data.data.clone(), data.width, data.height)?
    } else {
        default_icon()
    };
    let tray = TrayIconBuilder::new()
        .with_tooltip(format!("rmcp_mux – {}", snapshot.service_name))
        .with_icon(icon)
        .with_menu(Box::new(menu.clone()))
        .build()?;

    Ok(TrayUi {
        _tray: tray,
        header,
        status: status_item,
        clients: clients_item,
        pending: pending_item,
        init_state: init_item,
        restarts: restart_item,
        quit_id: quit_item.id().clone(),
    })
}

fn status_line(snapshot: &StatusSnapshot) -> String {
    let status_text = match &snapshot.server_status {
        ServerStatus::Starting => "Starting".to_string(),
        ServerStatus::Running => "Running".to_string(),
        ServerStatus::Restarting => "Restarting".to_string(),
        ServerStatus::Stopped => "Stopped".to_string(),
        ServerStatus::Failed(reason) => format!("Failed: {reason}"),
    };
    format!("Status: {status_text}")
}

fn client_line(snapshot: &StatusSnapshot) -> String {
    format!(
        "Clients: {} (active {}/{})",
        snapshot.connected_clients, snapshot.active_clients, snapshot.max_active_clients
    )
}

fn pending_line(snapshot: &StatusSnapshot) -> String {
    format!("Pending requests: {}", snapshot.pending_requests)
}

fn init_line(snapshot: &StatusSnapshot) -> String {
    let cache = if snapshot.cached_initialize {
        "cached"
    } else {
        "uncached"
    };
    let init = if snapshot.initializing {
        "initializing"
    } else {
        "idle"
    };
    format!("Initialize: {cache}, {init}")
}

fn restart_line(snapshot: &StatusSnapshot) -> String {
    match &snapshot.last_reset {
        Some(reason) => format!("Restarts: {} (last: {})", snapshot.restarts, reason),
        None => format!("Restarts: {}", snapshot.restarts),
    }
}

fn default_icon() -> Icon {
    let (w, h) = (16, 16);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let gradient = 0x60 + ((x + y) % 32) as u8;
            data.extend_from_slice(&[0x4b, gradient, 0xff, 0xff]);
        }
    }
    Icon::from_rgba(data, w as u32, h as u32).expect("valid icon")
}

pub(crate) fn find_tray_icon() -> Option<LoadedIcon> {
    let candidates = [
        PathBuf::from("public/rmcp_mux_icon.png"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("public/rmcp_mux_icon.png")))
            .unwrap_or_else(|| PathBuf::from("public/rmcp_mux_icon.png")),
    ];

    for path in candidates {
        if let Some(icon) = load_icon_from_file(&path) {
            return Some(icon);
        }
    }
    None
}

pub(crate) fn load_icon_from_file(path: &Path) -> Option<LoadedIcon> {
    let data = std::fs::read(path).ok()?;
    let img = image::load_from_memory_with_format(&data, ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(LoadedIcon {
        data: rgba.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("{}-{}", name, nanos))
    }

    #[test]
    fn load_icon_from_file_works_for_png() {
        let path = tmp_path("icon.png");
        let mut file = std::fs::File::create(&path).expect("create icon file");
        // Tiny 2x2 RGBA white image encoded as PNG via image crate
        let buf = vec![255u8; 2 * 2 * 4];
        let img = image::RgbaImage::from_raw(2, 2, buf).expect("build raw image");
        img.write_to(&mut file, image::ImageFormat::Png)
            .expect("write png");

        let icon = load_icon_from_file(&path);
        assert!(icon.is_some());
    }

    #[test]
    fn status_and_lines_render() {
        let base = StatusSnapshot {
            service_name: "s".into(),
            server_status: ServerStatus::Starting,
            restarts: 0,
            connected_clients: 1,
            active_clients: 1,
            max_active_clients: 3,
            pending_requests: 2,
            cached_initialize: false,
            initializing: true,
            last_reset: None,
            queue_depth: 0,
            child_pid: None,
            max_request_bytes: 1_048_576,
            restart_backoff_ms: 1_000,
            restart_backoff_max_ms: 30_000,
            max_restarts: 5,
        };
        assert!(status_line(&base).contains("Starting"));
        assert!(client_line(&base).contains("active 1/3"));
        assert!(pending_line(&base).contains("2"));
        assert!(init_line(&base).contains("uncached"));
        assert!(restart_line(&base).contains("0"));

        let mut running = base.clone();
        running.server_status = ServerStatus::Failed("x".into());
        running.cached_initialize = true;
        running.initializing = false;
        running.last_reset = Some("fail".into());
        assert!(status_line(&running).contains("Failed"));
        assert!(init_line(&running).contains("cached"));
        assert!(restart_line(&running).contains("fail"));
    }

    #[test]
    fn default_icon_does_not_panic() {
        let icon = default_icon();
        let _ = icon;
    }
}
