//! Persistence and export functions for the wizard.

use std::io::Write;

use anyhow::{Context, Result};

use crate::config::{Config, expand_path, safe_copy};
use crate::scan::{HostFile, HostFormat, rewire_host};

use super::types::{AppState, ConfirmChoice, HealthCheckChoice, Panel, WizardStep};

// ─────────────────────────────────────────────────────────────────────────────
// Config building
// ─────────────────────────────────────────────────────────────────────────────

pub fn build_config_for_export(app: &AppState) -> Config {
    let mut cfg = Config::default();
    for svc in &app.services {
        if svc.selected {
            cfg.servers.insert(svc.name.clone(), svc.config.clone());
        }
    }
    cfg
}

// ─────────────────────────────────────────────────────────────────────────────
// Client rewiring
// ─────────────────────────────────────────────────────────────────────────────

pub fn rewire_selected_clients(app: &AppState) -> Result<()> {
    for client in &app.clients {
        if client.selected && !client.already_rewired {
            let host_file = HostFile {
                kind: client.kind.clone(),
                path: client.config_path.clone(),
                format: match client.config_path.extension().and_then(|e| e.to_str()) {
                    Some("toml") => HostFormat::Toml,
                    _ => HostFormat::Json,
                },
            };

            // Use the socket_dir from app state
            match rewire_host(&host_file, &app.socket_dir, "rmcp_mux_proxy", &[], false) {
                Ok(outcome) => {
                    if let Some(backup) = outcome.backup {
                        tracing::info!(
                            "Rewired {} (backup: {})",
                            client.config_path.display(),
                            backup.display()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to rewire {}: {}", client.config_path.display(), e);
                }
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Config persistence
// ─────────────────────────────────────────────────────────────────────────────

pub fn persist_all(app: &AppState) -> Result<()> {
    let expanded_path = expand_path(app.config_path.to_string_lossy());

    // Create parent directory if needed
    if let Some(parent) = expanded_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    // Build the config
    let mut cfg = Config::default();
    for svc in &app.services {
        cfg.servers.insert(svc.name.clone(), svc.config.clone());
    }

    // Serialize based on extension
    let serialized = match expanded_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => serde_json::to_string_pretty(&cfg)?,
        "yaml" | "yml" => serde_yaml::to_string(&cfg)?,
        _ => toml::to_string_pretty(&cfg)?,
    };

    // Create backup if file exists
    if expanded_path.exists() {
        let backup_path = expanded_path.with_extension("bak");
        safe_copy(&expanded_path, &backup_path)?;
    }

    // Write the config
    std::fs::write(&expanded_path, serialized)
        .with_context(|| format!("failed to write {}", expanded_path.display()))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirm choice execution
// ─────────────────────────────────────────────────────────────────────────────

pub fn execute_confirm_choice(app: &mut AppState) -> Result<bool> {
    match app.confirm_choice {
        ConfirmChoice::SaveAll => {
            if !app.dry_run {
                // Save mux config
                persist_all(app)?;
                // Rewire selected clients
                rewire_selected_clients(app)?;
            }
            // Move to health check step
            app.wizard_step = WizardStep::HealthCheck;
            app.active_panel = Panel::ServiceList;
            app.health_choice = HealthCheckChoice::Ok;
            app.message = if app.dry_run {
                "STEP 4: Health Check (dry-run) - Verify config in your client, then confirm".into()
            } else {
                "STEP 4: Health Check - Verify config in your client, then confirm here".into()
            };
            Ok(false)
        }
        ConfirmChoice::SaveMuxOnly => {
            if !app.dry_run {
                persist_all(app)?;
            }
            // Move to health check step
            app.wizard_step = WizardStep::HealthCheck;
            app.active_panel = Panel::ServiceList;
            app.health_choice = HealthCheckChoice::Ok;
            app.message = if app.dry_run {
                "STEP 4: Health Check (dry-run) - Verify config in your client, then confirm".into()
            } else {
                "STEP 4: Health Check - Verify config in your client, then confirm here".into()
            };
            Ok(false)
        }
        ConfirmChoice::CopyToClipboard => {
            // Build config string for clipboard
            let cfg = build_config_for_export(app);
            if let Ok(text) = toml::to_string_pretty(&cfg) {
                // Try to copy to clipboard using pbcopy on macOS
                if let Ok(mut child) = std::process::Command::new("pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    let _ = child.wait();
                    app.message = "Configuration copied to clipboard!".into();
                } else {
                    app.message = "Failed to copy to clipboard (pbcopy not available)".into();
                }
            } else {
                app.message = "Failed to serialize configuration".into();
            }
            Ok(false)
        }
        ConfirmChoice::Back => {
            // Go back to step 2
            app.wizard_step = WizardStep::ClientSelection;
            app.active_panel = Panel::ServiceList;
            let selected_count = app.clients.iter().filter(|c| c.selected).count();
            let total_count = app.clients.len();
            app.message = format!(
                "STEP 2: {} of {} clients selected | Space: toggle | n: next step | p: previous",
                selected_count, total_count
            );
            Ok(false)
        }
        ConfirmChoice::Exit => Ok(true),
    }
}
