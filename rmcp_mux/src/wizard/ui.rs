//! UI drawing functions for the wizard TUI.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::scan::HostKind;

use super::types::{
    AppState, ConfirmChoice, Field, HealthCheckChoice, HealthStatus, Panel, ServiceSource,
    WizardStep,
};

// ─────────────────────────────────────────────────────────────────────────────
// Main draw function
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_ui(f: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Main area (two columns)
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    // Title with step indicator
    let step_info = match app.wizard_step {
        WizardStep::ServerSelection => "Step 1/4: Server Detection",
        WizardStep::ClientSelection => "Step 2/4: Client Detection",
        WizardStep::Confirmation => "Step 3/4: Confirmation",
        WizardStep::HealthCheck => "Step 4/4: Health Check",
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "rmcp_mux wizard",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" — "),
        Span::styled(step_info, Style::default().fg(Color::Cyan)),
    ]));
    f.render_widget(title, chunks[0]);

    // Main area: two columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Left: list
            Constraint::Percentage(65), // Right: editor/details
        ])
        .split(chunks[1]);

    // Draw appropriate content based on wizard step
    match app.wizard_step {
        WizardStep::ServerSelection => {
            draw_service_list(f, app, main_chunks[0]);
            draw_editor(f, app, main_chunks[1]);
        }
        WizardStep::ClientSelection => {
            draw_client_list(f, app, main_chunks[0]);
            draw_client_details(f, app, main_chunks[1]);
        }
        WizardStep::Confirmation => {
            draw_summary(f, app, main_chunks[0]);
            draw_save_options(f, app, main_chunks[1]);
        }
        WizardStep::HealthCheck => {
            draw_health_check_info(f, app, main_chunks[0]);
            draw_health_check_options(f, app, main_chunks[1]);
        }
    }

    // Status bar
    let mut footer_spans = vec![Span::raw(&app.message)];
    if app.dry_run {
        footer_spans.push(Span::styled(
            " | DRY-RUN",
            Style::default().fg(Color::Yellow),
        ));
    }
    let status = Paragraph::new(Line::from(footer_spans))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[2]);

    // Draw confirm dialog if active
    if app.active_panel == Panel::ConfirmDialog {
        draw_confirm_dialog(f, app);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service list (Step 1)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_service_list(f: &mut Frame, app: &AppState, area: Rect) {
    let is_active =
        app.active_panel == Panel::ServiceList && app.wizard_step == WizardStep::ServerSelection;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Count services by source and selection for title
    let selected_count = app.services.iter().filter(|s| s.selected).count();
    let total_count = app.services.len();
    let title = format!("STEP 1: Servers [{}/{}]", selected_count, total_count);

    let items: Vec<ListItem> = app
        .services
        .iter()
        .enumerate()
        .map(|(i, svc)| {
            // Selection checkbox
            let checkbox = if svc.selected {
                Span::styled("[x] ", Style::default().fg(Color::Green))
            } else {
                Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
            };

            // Source indicator: config file vs detected process
            let source_indicator = match svc.source {
                ServiceSource::Config => Span::styled("[C]", Style::default().fg(Color::Blue)),
                ServiceSource::Detected => Span::styled("[D]", Style::default().fg(Color::Magenta)),
            };

            // Health indicator
            let health_indicator = match svc.health {
                HealthStatus::Healthy => Span::styled(" ● ", Style::default().fg(Color::Green)),
                HealthStatus::Unhealthy => Span::styled(" ● ", Style::default().fg(Color::Red)),
                HealthStatus::Unknown => Span::styled(" ○ ", Style::default().fg(Color::DarkGray)),
            };

            let name_style = if i == app.selected_service {
                if is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                }
            } else {
                Style::default()
            };

            let dirty_marker = if svc.dirty {
                Span::styled(" *", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            };

            // Show PID for detected processes
            let pid_info = match (svc.source, svc.pid) {
                (ServiceSource::Detected, Some(pid)) => {
                    Span::styled(format!(" ({})", pid), Style::default().fg(Color::DarkGray))
                }
                _ => Span::raw(""),
            };

            ListItem::new(Line::from(vec![
                checkbox,
                source_indicator,
                health_indicator,
                Span::styled(&svc.name, name_style),
                dirty_marker,
                pid_info,
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    f.render_widget(list, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor panel (Step 1)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_editor(f: &mut Frame, app: &AppState, area: Rect) {
    let is_active = app.active_panel == Panel::Editor;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let fields = vec![
        (Field::ServiceName, "Service name", &app.form.service_name),
        (Field::Socket, "Socket", &app.form.socket),
        (Field::Cmd, "Command", &app.form.cmd),
        (Field::Args, "Args", &app.form.args),
        (Field::MaxClients, "Max clients", &app.form.max_clients),
        (Field::LogLevel, "Log level", &app.form.log_level),
    ];

    let mut lines: Vec<Line> = fields
        .into_iter()
        .map(|(field, label, value)| {
            let label_style = Style::default().fg(Color::Cyan);
            let val_style = if Some(field) == app.editing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if field == app.current_field && is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Line::from(vec![
                Span::styled(format!("{label:<14}"), label_style),
                Span::styled(value.clone(), val_style),
            ])
        })
        .collect();

    // Tray field
    let tray_label = if app.form.tray { "true" } else { "false" };
    let tray_style = if Some(Field::Tray) == app.editing {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if app.current_field == Field::Tray && is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled("Tray enabled  ", Style::default().fg(Color::Cyan)),
        Span::styled(tray_label, tray_style),
    ]));

    let editor = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title("Editor"),
    );

    f.render_widget(editor, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Client list (Step 2)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_client_list(f: &mut Frame, app: &AppState, area: Rect) {
    let is_active =
        app.active_panel == Panel::ServiceList && app.wizard_step == WizardStep::ClientSelection;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_count = app.clients.iter().filter(|c| c.selected).count();
    let total_count = app.clients.len();
    let title = format!("STEP 2: Clients [{}/{}]", selected_count, total_count);

    let items: Vec<ListItem> = app
        .clients
        .iter()
        .enumerate()
        .map(|(i, client)| {
            // Selection checkbox
            let checkbox = if client.selected {
                Span::styled("[x] ", Style::default().fg(Color::Green))
            } else {
                Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
            };

            // Host kind indicator
            let kind_label = match client.kind {
                HostKind::Codex => Span::styled("Codex", Style::default().fg(Color::Blue)),
                HostKind::Cursor => Span::styled("Cursor", Style::default().fg(Color::Magenta)),
                HostKind::VSCode => Span::styled("VSCode", Style::default().fg(Color::Cyan)),
                HostKind::Claude => Span::styled("Claude", Style::default().fg(Color::Yellow)),
                HostKind::JetBrains => Span::styled("JetBrains", Style::default().fg(Color::Green)),
                HostKind::Unknown => Span::styled("Unknown", Style::default().fg(Color::DarkGray)),
            };

            // Rewired status indicator
            let status = if !client.config_exists {
                Span::styled(" [no config]", Style::default().fg(Color::Red))
            } else if client.already_rewired {
                Span::styled(" [rewired]", Style::default().fg(Color::Green))
            } else {
                Span::styled(" [not rewired]", Style::default().fg(Color::Yellow))
            };

            let name_style = if i == app.selected_client {
                if is_active {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }
            } else {
                Style::default()
            };

            // Service count
            let svc_count = Span::styled(
                format!(" ({} svcs)", client.services.len()),
                Style::default().fg(Color::DarkGray),
            );

            ListItem::new(Line::from(vec![
                checkbox,
                Span::styled("", name_style), // Apply style context
                kind_label,
                status,
                svc_count,
            ]))
        })
        .collect();

    if items.is_empty() {
        let empty_msg = Paragraph::new(
            "No MCP clients detected.\nSupported: Codex, Cursor, VSCode, Claude, JetBrains",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .wrap(Wrap { trim: true });
        f.render_widget(empty_msg, area);
        return;
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    f.render_widget(list, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Client details panel (Step 2)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_client_details(f: &mut Frame, app: &AppState, area: Rect) {
    let is_active =
        app.active_panel == Panel::Editor && app.wizard_step == WizardStep::ClientSelection;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Client Configuration Details",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.clients.is_empty() {
        lines.push(Line::from("No clients detected."));
        lines.push(Line::from(""));
        lines.push(Line::from("The wizard searches for MCP client configs in:"));
        lines.push(Line::from("  • ~/.codex/config.toml (Codex)"));
        lines.push(Line::from("  • ~/Library/.../Cursor/settings.json"));
        lines.push(Line::from("  • ~/Library/.../Code/settings.json (VSCode)"));
        lines.push(Line::from("  • ~/.config/Claude/claude_config.json"));
        lines.push(Line::from("  • ~/Library/.../JetBrains/LLM/mcp.json"));
    } else if app.selected_client < app.clients.len() {
        let client = &app.clients[app.selected_client];

        lines.push(Line::from(vec![
            Span::styled("Host:     ", Style::default().fg(Color::Cyan)),
            Span::raw(client.kind.as_label()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Config:   ", Style::default().fg(Color::Cyan)),
            Span::raw(client.config_path.display().to_string()),
        ]));

        // Config existence status
        if !client.config_exists {
            lines.push(Line::from(vec![
                Span::styled("Status:   ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "No MCP config file (app installed)",
                    Style::default().fg(Color::Red),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "A new MCP config will be created for this client.",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Status:   ", Style::default().fg(Color::Cyan)),
                if client.already_rewired {
                    Span::styled(
                        "Already rewired to rmcp_mux",
                        Style::default().fg(Color::Green),
                    )
                } else {
                    Span::styled("Not yet rewired", Style::default().fg(Color::Yellow))
                },
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Services in this client:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        if client.services.is_empty() {
            if client.config_exists {
                lines.push(Line::from("  (no MCP services defined yet)"));
            } else {
                lines.push(Line::from("  (config will be created with mux services)"));
            }
        } else {
            for svc in &client.services {
                lines.push(Line::from(format!("  • {}", svc)));
            }
        }

        lines.push(Line::from(""));
        if client.selected {
            if client.config_exists {
                lines.push(Line::from(Span::styled(
                    "This client will be rewired to use rmcp_mux proxy.",
                    Style::default().fg(Color::Green),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "A new MCP config will be created for this client.",
                    Style::default().fg(Color::Green),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "This client will NOT be modified.",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("Details"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(details, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary panel (Step 3)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_summary(f: &mut Frame, app: &AppState, area: Rect) {
    let border_style = Style::default().fg(Color::Cyan);

    let selected_servers: Vec<&str> = app
        .services
        .iter()
        .filter(|s| s.selected)
        .map(|s| s.name.as_str())
        .collect();

    let selected_clients: Vec<&str> = app
        .clients
        .iter()
        .filter(|c| c.selected)
        .map(|c| c.kind.as_label())
        .collect();

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Configuration Summary",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Selected Servers ({})", selected_servers.len()),
            Style::default().fg(Color::Cyan),
        )),
    ];

    for name in &selected_servers {
        lines.push(Line::from(format!("  [x] {}", name)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Selected Clients ({})", selected_clients.len()),
        Style::default().fg(Color::Cyan),
    )));

    for name in &selected_clients {
        lines.push(Line::from(format!("  [x] {}", name)));
    }

    let summary = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("STEP 3: Summary"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(summary, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Save options panel (Step 3)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_save_options(f: &mut Frame, app: &AppState, area: Rect) {
    let border_style = Style::default().fg(Color::Cyan);

    let choices = [
        (
            ConfirmChoice::SaveAll,
            "Save All",
            "Save mux config AND rewire selected clients",
        ),
        (
            ConfirmChoice::SaveMuxOnly,
            "Mux Only",
            "Save mux config only (no client rewiring)",
        ),
        (
            ConfirmChoice::CopyToClipboard,
            "Clipboard",
            "Copy config to clipboard",
        ),
        (ConfirmChoice::Back, "Back", "Return to previous step"),
        (ConfirmChoice::Exit, "Exit", "Exit without saving"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Save Options",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Use Up/Down to select, Enter to confirm:"),
        Line::from(""),
    ];

    for (choice, label, description) in choices {
        let is_selected = choice == app.confirm_choice;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("[{}]", label), style),
            Span::raw(" - "),
            Span::styled(description, Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines.push(Line::from(""));
    if app.dry_run {
        lines.push(Line::from(Span::styled(
            "DRY-RUN MODE: No files will be modified",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Backups will be created for all modified files (.bak)",
            Style::default().fg(Color::Green),
        )));
    }

    let options = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("Actions"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(options, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Health check panel (Step 4)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_health_check_info(f: &mut Frame, app: &AppState, area: Rect) {
    let border_style = Style::default().fg(Color::Cyan);

    let selected_servers: Vec<&str> = app
        .services
        .iter()
        .filter(|s| s.selected)
        .map(|s| s.name.as_str())
        .collect();

    let selected_clients: Vec<&str> = app
        .clients
        .iter()
        .filter(|c| c.selected)
        .map(|c| c.kind.as_label())
        .collect();

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Configuration Saved!",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Now verify the configuration works:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("1. Go to your MCP client application"),
        Line::from("2. Check if the MCP servers are working"),
        Line::from("3. Return here and confirm the result"),
        Line::from(""),
        Line::from(Span::styled(
            format!("Configured Servers ({})", selected_servers.len()),
            Style::default().fg(Color::Cyan),
        )),
    ];

    for name in &selected_servers {
        lines.push(Line::from(format!("  [x] {}", name)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Rewired Clients ({})", selected_clients.len()),
        Style::default().fg(Color::Cyan),
    )));

    for name in &selected_clients {
        lines.push(Line::from(format!("  [x] {}", name)));
    }

    let info = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("STEP 4: Health Check"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(info, area);
}

pub fn draw_health_check_options(f: &mut Frame, app: &AppState, area: Rect) {
    let border_style = Style::default().fg(Color::Cyan);

    let choices = [
        (
            HealthCheckChoice::Ok,
            "OK",
            "Configuration verified - exit wizard",
        ),
        (
            HealthCheckChoice::TryAgain,
            "Try Again",
            "Re-run detection and reconfigure",
        ),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Verification",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Did the configuration work correctly?"),
        Line::from(""),
        Line::from("Use Up/Down to select, Enter to confirm:"),
        Line::from(""),
    ];

    for (choice, label, description) in choices {
        let is_selected = choice == app.health_choice;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("[{}]", label), style),
            Span::raw(" - "),
            Span::styled(description, Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tip: Keep this terminal open while testing",
        Style::default().fg(Color::DarkGray),
    )));

    let options = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("Actions"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(options, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirm dialog (overlay)
// ─────────────────────────────────────────────────────────────────────────────

pub fn draw_confirm_dialog(f: &mut Frame, app: &AppState) {
    let area = f.area();
    let dialog_width = 40;
    let dialog_height = 7;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear the background
    f.render_widget(Clear, dialog_area);

    let choices = [
        (ConfirmChoice::SaveAll, "SAVE ALL"),
        (ConfirmChoice::SaveMuxOnly, "MUX ONLY"),
        (ConfirmChoice::CopyToClipboard, "CLIPBOARD"),
        (ConfirmChoice::Back, "BACK"),
        (ConfirmChoice::Exit, "EXIT"),
    ];

    let choice_spans: Vec<Span> = choices
        .iter()
        .map(|(choice, label)| {
            if *choice == app.confirm_choice {
                Span::styled(
                    format!(" [{label}] "),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(format!("  {label}  "), Style::default().fg(Color::White))
            }
        })
        .collect();

    let content = vec![
        Line::from(""),
        Line::from("Save configuration?"),
        Line::from(""),
        Line::from(choice_spans),
    ];

    let dialog = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title("Confirm"),
        )
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(dialog, dialog_area);
}
