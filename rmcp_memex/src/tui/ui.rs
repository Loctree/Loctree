//! TUI Rendering Module
//!
//! Renders the wizard UI using ratatui widgets.

use crate::tui::app::{App, WizardStep};
use ratatui::prelude::*;
use ratatui::widgets::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer/help
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_main(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let step_num = match app.step {
        WizardStep::Welcome => 1,
        WizardStep::MemexSettings => 2,
        WizardStep::HostSelection => 3,
        WizardStep::SnippetPreview => 4,
        WizardStep::HealthCheck => 5,
        WizardStep::Summary => 6,
    };

    let title = format!(
        " rmcp_memex wizard v{} - Step {}/6: {} ",
        VERSION,
        step_num,
        app.step.title()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold());

    frame.render_widget(block, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = if app.input_mode {
        " [Enter] Save | [Esc] Cancel | Type to edit "
    } else {
        match app.step {
            WizardStep::Welcome => " [→/n] Next | [q] Quit ",
            WizardStep::MemexSettings => {
                " [↑↓] Navigate | [Enter] Edit | [→/n] Next | [←/p] Back | [q] Quit "
            }
            WizardStep::HostSelection => {
                " [↑↓] Navigate | [Space/Enter] Toggle | [→/n] Next | [←/p] Back | [q] Quit "
            }
            WizardStep::SnippetPreview => " [→/n] Next | [←/p] Back | [q] Quit ",
            WizardStep::HealthCheck => " [Enter] Run Check | [→/n] Next | [←/p] Back | [q] Quit ",
            WizardStep::Summary => " [Enter] Write Configs | [←/p] Back | [q] Quit ",
        }
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

    frame.render_widget(help, area);
}

fn render_main(frame: &mut Frame, area: Rect, app: &App) {
    match app.step {
        WizardStep::Welcome => render_welcome(frame, area, app),
        WizardStep::MemexSettings => render_settings(frame, area, app),
        WizardStep::HostSelection => render_host_selection(frame, area, app),
        WizardStep::SnippetPreview => render_snippet_preview(frame, area, app),
        WizardStep::HealthCheck => render_health_check(frame, area, app),
        WizardStep::Summary => render_summary(frame, area, app),
    }
}

fn render_welcome(frame: &mut Frame, area: Rect, app: &App) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to rmcp_memex Configuration Wizard",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from("This wizard will help you:"),
        Line::from(""),
        Line::from("  1. Configure rmcp_memex settings (database path, cache, etc.)"),
        Line::from("  2. Detect and configure MCP host integrations"),
        Line::from("  3. Generate configuration snippets for your hosts"),
        Line::from("  4. Verify your setup with a health check"),
        Line::from(""),
        Line::from(Span::styled(
            format!("Binary: {}", app.binary_path),
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
        Line::from(format!(
            "Detected {} MCP host configuration(s)",
            app.hosts.iter().filter(|h| h.exists).count()
        )),
        Line::from(""),
        if app.dry_run {
            Line::from(Span::styled(
                "DRY RUN MODE - No files will be written",
                Style::default().fg(Color::Yellow).bold(),
            ))
        } else {
            Line::from("")
        },
        Line::from(""),
        Line::from(Span::styled(
            "Press → or 'n' to continue...",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    frame.render_widget(paragraph, area);
}

fn render_settings(frame: &mut Frame, area: Rect, app: &App) {
    let fields = [
        (
            "Database Path",
            &app.memex_cfg.db_path,
            "LanceDB storage location",
        ),
        (
            "Cache Size (MB)",
            &app.memex_cfg.cache_mb.to_string(),
            "In-memory cache size",
        ),
        (
            "Log Level",
            &app.memex_cfg.log_level,
            "trace/debug/info/warn/error",
        ),
        (
            "Max Request (bytes)",
            &app.memex_cfg.max_request_bytes.to_string(),
            "JSON-RPC size limit",
        ),
        ("Mode", &app.memex_cfg.mode, "full or memory"),
    ];

    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value, hint))| {
            let is_focused = i == app.focus;
            let is_editing = app.input_mode && app.editing_field == Some(i);

            let display_value = if is_editing {
                format!("{}▏", app.input_buffer)
            } else {
                (*value).clone()
            };

            let style = if is_editing {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else if is_focused {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };

            let prefix = if is_focused { "▶ " } else { "  " };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{:<20}", label), style),
                Span::styled(format!("{:<40}", display_value), style),
                Span::styled(format!(" ({})", hint), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Memex Configuration "),
    );

    frame.render_widget(list, area);
}

fn render_host_selection(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let is_focused = i == app.focus;
            let is_selected = app.selected_hosts.contains(&i);

            let checkbox = if is_selected { "[✓]" } else { "[ ]" };
            let status = host.status_icon();

            let style = if is_focused {
                Style::default().fg(Color::Cyan).bold()
            } else if is_selected {
                Style::default().fg(Color::Green)
            } else if !host.exists {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let prefix = if is_focused { "▶ " } else { "  " };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{} ", checkbox), style),
                Span::styled(format!("{} ", status), style),
                Span::styled(format!("{:<20}", host.kind.display_name()), style),
                Span::styled(
                    format!(" {} ", host.status_text()),
                    if host.has_rmcp_memex {
                        Style::default().fg(Color::Green)
                    } else if host.exists {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Select MCP Hosts to Configure "),
    );

    frame.render_widget(list, area);
}

fn render_snippet_preview(frame: &mut Frame, area: Rect, app: &App) {
    let snippets = app.generate_snippets();

    let mut lines = vec![
        Line::from(Span::styled(
            "Configuration snippets for selected hosts:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    if snippets.is_empty() {
        lines.push(Line::from(Span::styled(
            "No hosts selected. Go back and select at least one host.",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        for (kind, snippet) in snippets {
            lines.push(Line::from(Span::styled(
                format!("--- {} ---", kind.display_name()),
                Style::default().fg(Color::Green).bold(),
            )));
            for line in snippet.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::White),
                )));
            }
            lines.push(Line::from(""));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Config Snippets Preview "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_health_check(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![
        Line::from(Span::styled(
            "Health Check",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
    ];

    if let Some(status) = &app.health_status {
        lines.push(Line::from(Span::styled(
            format!("Binary: {}", status),
            if status.starts_with('✓') {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            },
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Press [Enter] to run health check",
            Style::default().fg(Color::Yellow),
        )));
    }

    lines.push(Line::from(""));

    for msg in &app.messages {
        let style = if msg.starts_with('✓') {
            Style::default().fg(Color::Green)
        } else if msg.starts_with('✗') {
            Style::default().fg(Color::Red)
        } else if msg.starts_with('○') {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(msg.clone(), style)));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Health Check "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_summary(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![
        Line::from(Span::styled(
            "Configuration Summary",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("Memex Settings:", Style::default().bold())),
        Line::from(format!("  Database: {}", app.memex_cfg.db_path)),
        Line::from(format!("  Cache: {} MB", app.memex_cfg.cache_mb)),
        Line::from(format!("  Log Level: {}", app.memex_cfg.log_level)),
        Line::from(format!("  Mode: {}", app.memex_cfg.mode)),
        Line::from(""),
        Line::from(Span::styled("Selected Hosts:", Style::default().bold())),
    ];

    let selected = app.get_selected_hosts();
    if selected.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none selected)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for host in selected {
            lines.push(Line::from(format!("  • {}", host.kind.display_name())));
        }
    }

    lines.push(Line::from(""));

    if app.dry_run {
        lines.push(Line::from(Span::styled(
            "DRY RUN MODE - Press [Enter] to preview changes",
            Style::default().fg(Color::Yellow).bold(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Press [Enter] to write configuration files",
            Style::default().fg(Color::Green).bold(),
        )));
    }

    lines.push(Line::from(""));

    // Show any messages from write operation
    for msg in &app.messages {
        let style = if msg.starts_with('✓') {
            Style::default().fg(Color::Green)
        } else if msg.contains("Warning") || msg.contains("DRY RUN") {
            Style::default().fg(Color::Yellow)
        } else if msg.contains("Error") {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(msg.clone(), style)));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Summary "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
