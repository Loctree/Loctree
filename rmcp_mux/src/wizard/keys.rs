//! Key handling logic for the wizard TUI.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::clients::detect_clients;
use super::persist::{execute_confirm_choice, persist_all};
use super::services::{check_health, default_server_config, form_from_service, service_from_form};
use super::types::{
    AppState, ConfirmChoice, Field, HealthCheckChoice, HealthStatus, Panel, ServiceEntry,
    ServiceSource, WizardStep, next_confirm_choice, next_field, previous_confirm_choice,
    previous_field,
};

// ─────────────────────────────────────────────────────────────────────────────
// Main key handler
// ─────────────────────────────────────────────────────────────────────────────

pub fn handle_key(app: &mut AppState, key: KeyEvent) -> Result<bool> {
    // Handle confirm dialog separately
    if app.active_panel == Panel::ConfirmDialog {
        return handle_confirm_dialog_key(app, key);
    }

    let is_ctrl_s = key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'));
    let is_plain_s = matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        && app.editing.is_none()
        && app.active_panel != Panel::Editor;
    let is_save = is_ctrl_s || is_plain_s;

    match key.code {
        // Quit (only when not editing)
        KeyCode::Char('q') if app.editing.is_none() => {
            return Ok(true);
        }

        // Save -> open confirm dialog
        _ if is_save => {
            // First sync form to service
            sync_form_to_service(app);
            app.active_panel = Panel::ConfirmDialog;
            app.confirm_choice = ConfirmChoice::SaveAll;
            app.message = "Use arrows to select, Enter to confirm".into();
        }

        // Tab to switch panels
        KeyCode::Tab => {
            if app.editing.is_none() {
                app.active_panel = match app.active_panel {
                    Panel::ServiceList => Panel::Editor,
                    Panel::Editor => Panel::ServiceList,
                    Panel::ConfirmDialog => Panel::ConfirmDialog,
                };
                update_message(app);
            }
        }

        KeyCode::BackTab => {
            if app.editing.is_none() {
                app.active_panel = match app.active_panel {
                    Panel::ServiceList => Panel::Editor,
                    Panel::Editor => Panel::ServiceList,
                    Panel::ConfirmDialog => Panel::ConfirmDialog,
                };
                update_message(app);
            }
        }

        // Navigation
        KeyCode::Up => {
            if app.editing.is_none() {
                match (app.wizard_step, app.active_panel) {
                    (WizardStep::ServerSelection, Panel::ServiceList) => {
                        if app.selected_service > 0 {
                            sync_form_to_service(app);
                            app.selected_service -= 1;
                            load_service_to_form(app);
                        }
                    }
                    (WizardStep::ServerSelection, Panel::Editor) => {
                        app.current_field = previous_field(app.current_field);
                    }
                    (WizardStep::ClientSelection, Panel::ServiceList) => {
                        if app.selected_client > 0 {
                            app.selected_client -= 1;
                        }
                    }
                    (WizardStep::Confirmation, _) => {
                        // Navigate through save options
                        app.confirm_choice = previous_confirm_choice(app.confirm_choice);
                    }
                    (WizardStep::HealthCheck, _) => {
                        // Toggle between Ok and TryAgain
                        app.health_choice = match app.health_choice {
                            HealthCheckChoice::Ok => HealthCheckChoice::TryAgain,
                            HealthCheckChoice::TryAgain => HealthCheckChoice::Ok,
                        };
                    }
                    _ => {}
                }
            }
        }

        KeyCode::Down => {
            if app.editing.is_none() {
                match (app.wizard_step, app.active_panel) {
                    (WizardStep::ServerSelection, Panel::ServiceList) => {
                        if app.selected_service < app.services.len().saturating_sub(1) {
                            sync_form_to_service(app);
                            app.selected_service += 1;
                            load_service_to_form(app);
                        }
                    }
                    (WizardStep::ServerSelection, Panel::Editor) => {
                        app.current_field = next_field(app.current_field);
                    }
                    (WizardStep::ClientSelection, Panel::ServiceList) => {
                        if app.selected_client < app.clients.len().saturating_sub(1) {
                            app.selected_client += 1;
                        }
                    }
                    (WizardStep::Confirmation, _) => {
                        // Navigate through save options
                        app.confirm_choice = next_confirm_choice(app.confirm_choice);
                    }
                    (WizardStep::HealthCheck, _) => {
                        // Toggle between Ok and TryAgain
                        app.health_choice = match app.health_choice {
                            HealthCheckChoice::Ok => HealthCheckChoice::TryAgain,
                            HealthCheckChoice::TryAgain => HealthCheckChoice::Ok,
                        };
                    }
                    _ => {}
                }
            }
        }

        // Enter
        KeyCode::Enter => {
            match (app.wizard_step, app.active_panel) {
                (WizardStep::ServerSelection, Panel::ServiceList) => {
                    // Switch to editor panel
                    app.active_panel = Panel::Editor;
                    update_message(app);
                }
                (WizardStep::ServerSelection, Panel::Editor) => {
                    if app.current_field == Field::Tray {
                        app.form.tray = !app.form.tray;
                        app.form.dirty = true;
                    } else {
                        app.editing = Some(app.current_field);
                        app.message = "Editing... Esc to finish".into();
                    }
                }
                (WizardStep::ClientSelection, Panel::ServiceList) => {
                    // Toggle client selection on Enter as well
                    if app.selected_client < app.clients.len() {
                        app.clients[app.selected_client].selected =
                            !app.clients[app.selected_client].selected;
                        update_step2_message(app);
                    }
                }
                (WizardStep::Confirmation, _) => {
                    // Execute the selected action
                    return execute_confirm_choice(app);
                }
                (WizardStep::HealthCheck, _) => {
                    // Execute health check choice
                    return execute_health_check_choice(app);
                }
                _ => {}
            }
        }

        // Space toggles selection (in ServiceList for both steps) or tray (in Editor)
        KeyCode::Char(' ') => {
            match (app.wizard_step, app.active_panel) {
                (WizardStep::ServerSelection, Panel::ServiceList) => {
                    // Toggle selection for current server
                    if app.selected_service < app.services.len() {
                        app.services[app.selected_service].selected =
                            !app.services[app.selected_service].selected;
                        let selected_count = app.services.iter().filter(|s| s.selected).count();
                        app.message = format!(
                            "STEP 1: {} servers selected | Space: toggle | Tab: edit | n: next step",
                            selected_count
                        );
                    }
                }
                (WizardStep::ServerSelection, Panel::Editor) => {
                    if app.current_field == Field::Tray {
                        app.form.tray = !app.form.tray;
                        app.form.dirty = true;
                    }
                }
                (WizardStep::ClientSelection, Panel::ServiceList) => {
                    // Toggle selection for current client
                    if app.selected_client < app.clients.len() {
                        app.clients[app.selected_client].selected =
                            !app.clients[app.selected_client].selected;
                        update_step2_message(app);
                    }
                }
                _ => {}
            }
        }

        // Escape
        KeyCode::Esc => {
            if app.editing.is_some() {
                app.editing = None;
                update_message(app);
            }
        }

        // Next step with 'n' key
        KeyCode::Char('n') if app.editing.is_none() => {
            match app.wizard_step {
                WizardStep::ServerSelection => {
                    // Check if any servers are selected
                    let selected_count = app.services.iter().filter(|s| s.selected).count();
                    if selected_count == 0 {
                        app.message =
                            "Please select at least one server (use Space to toggle)".into();
                    } else {
                        // Move to step 2: Client Selection
                        sync_form_to_service(app);
                        app.wizard_step = WizardStep::ClientSelection;
                        app.clients = detect_clients();
                        app.selected_client = 0;
                        app.active_panel = Panel::ServiceList;
                        let client_count = app.clients.len();
                        app.message = format!(
                            "STEP 2: Client Detection - {} clients found | Space: toggle | n: next step | p: previous",
                            client_count
                        );
                    }
                }
                WizardStep::ClientSelection => {
                    // Move to step 3: Confirmation
                    app.wizard_step = WizardStep::Confirmation;
                    app.active_panel = Panel::ConfirmDialog;
                    app.confirm_choice = ConfirmChoice::SaveAll;
                    app.message = "STEP 3: Confirm - Select action and press Enter".into();
                }
                WizardStep::Confirmation => {
                    // Already at confirmation, do nothing
                }
                WizardStep::HealthCheck => {
                    // Already at last step, do nothing
                }
            }
        }

        // Previous step with 'p' key
        KeyCode::Char('p') if app.editing.is_none() => {
            match app.wizard_step {
                WizardStep::ServerSelection => {
                    // Already at first step, do nothing
                }
                WizardStep::ClientSelection => {
                    // Go back to step 1
                    app.wizard_step = WizardStep::ServerSelection;
                    app.active_panel = Panel::ServiceList;
                    let selected_count = app.services.iter().filter(|s| s.selected).count();
                    app.message = format!(
                        "STEP 1: Server Detection - {} servers selected | Space: toggle | n: next step",
                        selected_count
                    );
                }
                WizardStep::Confirmation => {
                    // Go back to step 2
                    app.wizard_step = WizardStep::ClientSelection;
                    app.active_panel = Panel::ServiceList;
                    let client_count = app.clients.len();
                    app.message = format!(
                        "STEP 2: Client Detection - {} clients found | Space: toggle | n: next step | p: previous",
                        client_count
                    );
                }
                WizardStep::HealthCheck => {
                    // Go back to step 3
                    app.wizard_step = WizardStep::Confirmation;
                    app.active_panel = Panel::ConfirmDialog;
                    app.confirm_choice = ConfirmChoice::SaveAll;
                    app.message = "STEP 3: Confirm - Select action and press Enter".into();
                }
            }
        }

        // Add new service with 'a' key
        KeyCode::Char('a')
            if app.editing.is_none()
                && app.active_panel == Panel::ServiceList
                && app.wizard_step == WizardStep::ServerSelection =>
        {
            let new_name = format!("new-service-{}", app.services.len() + 1);
            app.services.push(ServiceEntry {
                name: new_name,
                config: default_server_config(),
                health: HealthStatus::Unknown,
                dirty: true,
                source: ServiceSource::Config,
                pid: None,
                selected: true,
            });
            app.selected_service = app.services.len() - 1;
            load_service_to_form(app);
            app.message = "New service added. Edit in the right panel.".into();
        }

        // Refresh health with 'r' key (must be before general Char(c) handler)
        KeyCode::Char('r') if app.editing.is_none() => {
            for svc in &mut app.services {
                svc.health = check_health(&svc.config);
            }
            app.message = "Health checks refreshed".into();
        }

        // Backspace in edit mode
        KeyCode::Backspace => {
            if let Some(field) = app.editing {
                mutate_field(&mut app.form, field, |s| {
                    s.pop();
                });
            }
        }

        // Character input in edit mode (must be last among Char handlers)
        KeyCode::Char(c) => {
            if let Some(field) = app.editing {
                mutate_field(&mut app.form, field, |s| s.push(c));
            }
        }

        _ => {}
    }

    Ok(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirm dialog key handler
// ─────────────────────────────────────────────────────────────────────────────

fn handle_confirm_dialog_key(app: &mut AppState, key: KeyEvent) -> Result<bool> {
    // Order of choices for navigation: SaveAll, SaveMuxOnly, CopyToClipboard, Back, Exit
    let choices = [
        ConfirmChoice::SaveAll,
        ConfirmChoice::SaveMuxOnly,
        ConfirmChoice::CopyToClipboard,
        ConfirmChoice::Back,
        ConfirmChoice::Exit,
    ];
    let current_idx = choices
        .iter()
        .position(|c| *c == app.confirm_choice)
        .unwrap_or(0);

    match key.code {
        KeyCode::Left => {
            let new_idx = if current_idx == 0 {
                choices.len() - 1
            } else {
                current_idx - 1
            };
            app.confirm_choice = choices[new_idx];
        }
        KeyCode::Right => {
            let new_idx = (current_idx + 1) % choices.len();
            app.confirm_choice = choices[new_idx];
        }
        KeyCode::Enter => match app.confirm_choice {
            ConfirmChoice::SaveAll => {
                if !app.dry_run {
                    persist_all(app)?;
                    // TODO: Also rewire selected clients
                }
                app.message = if app.dry_run {
                    "Dry run: config would be saved. Exiting...".into()
                } else {
                    "Configuration saved!".into()
                };
                return Ok(true);
            }
            ConfirmChoice::SaveMuxOnly => {
                if !app.dry_run {
                    persist_all(app)?;
                }
                app.message = if app.dry_run {
                    "Dry run: mux config would be saved. Exiting...".into()
                } else {
                    "Mux configuration saved!".into()
                };
                return Ok(true);
            }
            ConfirmChoice::CopyToClipboard => {
                // TODO: Copy config to clipboard
                app.message = "Config copied to clipboard (not yet implemented)".into();
            }
            ConfirmChoice::Back => {
                app.active_panel = Panel::Editor;
                update_message(app);
            }
            ConfirmChoice::Exit => {
                return Ok(true);
            }
        },
        KeyCode::Esc => {
            app.active_panel = Panel::Editor;
            update_message(app);
        }
        _ => {}
    }
    Ok(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Health check choice handler
// ─────────────────────────────────────────────────────────────────────────────

fn execute_health_check_choice(app: &mut AppState) -> Result<bool> {
    match app.health_choice {
        HealthCheckChoice::Ok => {
            // Configuration verified, exit wizard
            app.message = "Configuration verified successfully! Exiting...".into();
            Ok(true)
        }
        HealthCheckChoice::TryAgain => {
            // Re-run detection and go back to step 1
            use super::services::load_all_services;

            // Reload services from config and detect running processes
            if let Ok(services) = load_all_services(&app.config_path) {
                app.services = services;
            }

            // Run health checks on all services
            for svc in &mut app.services {
                svc.health = check_health(&svc.config);
            }

            // Clear clients (will be re-detected in step 2)
            app.clients.clear();
            app.selected_client = 0;

            // Reset to step 1
            app.wizard_step = WizardStep::ServerSelection;
            app.selected_service = 0;
            app.active_panel = Panel::ServiceList;
            app.health_choice = HealthCheckChoice::Ok;

            if !app.services.is_empty() {
                load_service_to_form(app);
            }

            let selected_count = app.services.iter().filter(|s| s.selected).count();
            app.message = format!(
                "STEP 1: Server Detection (retry) - {} servers | Space: toggle | n: next step",
                selected_count
            );

            Ok(false)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

pub fn sync_form_to_service(app: &mut AppState) {
    if app.form.dirty {
        let idx = app.selected_service;
        if idx < app.services.len() {
            app.services[idx].name = app.form.service_name.clone();
            app.services[idx].config = service_from_form(&app.form);
            app.services[idx].dirty = true;
        }
        app.form.dirty = false;
    }
}

pub fn load_service_to_form(app: &mut AppState) {
    let idx = app.selected_service;
    if idx < app.services.len() {
        app.form = form_from_service(&app.services[idx]);
    }
}

pub fn update_message(app: &mut AppState) {
    app.message = match app.active_panel {
        Panel::ServiceList => "Tab: switch | Up/Down: select | Enter: edit | n: new | r: refresh health | s: save | q: quit".into(),
        Panel::Editor => "Tab: switch | Up/Down: navigate | Enter: edit field | Space: toggle tray | Esc: stop edit | s: save".into(),
        Panel::ConfirmDialog => "Left/Right: select | Enter: confirm | Esc: cancel".into(),
    };
}

fn mutate_field<F: FnOnce(&mut String)>(form: &mut super::types::FormState, field: Field, f: F) {
    let target = match field {
        Field::ServiceName => &mut form.service_name,
        Field::Socket => &mut form.socket,
        Field::Cmd => &mut form.cmd,
        Field::Args => &mut form.args,
        Field::MaxClients => &mut form.max_clients,
        Field::LogLevel => &mut form.log_level,
        Field::Tray => return,
    };
    f(target);
    form.dirty = true;
}

fn update_step2_message(app: &mut AppState) {
    let selected_count = app.clients.iter().filter(|c| c.selected).count();
    let total_count = app.clients.len();
    app.message = format!(
        "STEP 2: {} of {} clients selected | Space: toggle | n: next step | p: previous",
        selected_count, total_count
    );
}
