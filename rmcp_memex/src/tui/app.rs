//! TUI Wizard Application Logic
//!
//! Main application state and step management for the configuration wizard.

use crate::ServerConfig;
use crate::tui::host_detection::{HostDetection, HostKind, detect_hosts, generate_snippet};
use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use std::io::{Stdout, stdout};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for running the wizard.
#[derive(Debug, Clone, Default)]
pub struct WizardConfig {
    pub config_path: Option<String>,
    pub dry_run: bool,
}

/// Wizard step enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    MemexSettings,
    HostSelection,
    SnippetPreview,
    HealthCheck,
    Summary,
}

impl WizardStep {
    pub fn title(&self) -> &'static str {
        match self {
            WizardStep::Welcome => "Welcome",
            WizardStep::MemexSettings => "Memex Settings",
            WizardStep::HostSelection => "Host Selection",
            WizardStep::SnippetPreview => "Config Preview",
            WizardStep::HealthCheck => "Health Check",
            WizardStep::Summary => "Summary",
        }
    }

    pub fn next(&self) -> Option<WizardStep> {
        match self {
            WizardStep::Welcome => Some(WizardStep::MemexSettings),
            WizardStep::MemexSettings => Some(WizardStep::HostSelection),
            WizardStep::HostSelection => Some(WizardStep::SnippetPreview),
            WizardStep::SnippetPreview => Some(WizardStep::HealthCheck),
            WizardStep::HealthCheck => Some(WizardStep::Summary),
            WizardStep::Summary => None,
        }
    }

    pub fn prev(&self) -> Option<WizardStep> {
        match self {
            WizardStep::Welcome => None,
            WizardStep::MemexSettings => Some(WizardStep::Welcome),
            WizardStep::HostSelection => Some(WizardStep::MemexSettings),
            WizardStep::SnippetPreview => Some(WizardStep::HostSelection),
            WizardStep::HealthCheck => Some(WizardStep::SnippetPreview),
            WizardStep::Summary => Some(WizardStep::HealthCheck),
        }
    }
}

/// Editable memex configuration.
#[derive(Debug, Clone)]
pub struct MemexCfg {
    pub db_path: String,
    pub cache_mb: usize,
    pub log_level: String,
    pub max_request_bytes: usize,
    pub mode: String,
}

impl Default for MemexCfg {
    fn default() -> Self {
        let cfg = ServerConfig::default();
        Self {
            db_path: cfg.db_path,
            cache_mb: cfg.cache_mb,
            log_level: "info".to_string(),
            max_request_bytes: cfg.max_request_bytes,
            mode: "full".to_string(),
        }
    }
}

/// Main application state.
pub struct App {
    pub step: WizardStep,
    pub memex_cfg: MemexCfg,
    pub hosts: Vec<HostDetection>,
    pub selected_hosts: Vec<usize>,
    pub dry_run: bool,
    pub messages: Vec<String>,
    pub focus: usize,
    pub binary_path: String,
    pub health_status: Option<String>,
    pub should_quit: bool,
    pub input_mode: bool,
    pub input_buffer: String,
    pub editing_field: Option<usize>,
}

impl App {
    pub fn new(config: WizardConfig) -> Self {
        let hosts = detect_hosts();
        let binary_path = which_rmcp_memex().unwrap_or_else(|| "rmcp_memex".to_string());

        Self {
            step: WizardStep::Welcome,
            memex_cfg: MemexCfg::default(),
            hosts,
            selected_hosts: Vec::new(),
            dry_run: config.dry_run,
            messages: Vec::new(),
            focus: 0,
            binary_path,
            health_status: None,
            should_quit: false,
            input_mode: false,
            input_buffer: String::new(),
            editing_field: None,
        }
    }

    pub fn next_step(&mut self) {
        if let Some(next) = self.step.next() {
            self.step = next;
            self.focus = 0;
        }
    }

    pub fn prev_step(&mut self) {
        if let Some(prev) = self.step.prev() {
            self.step = prev;
            self.focus = 0;
        }
    }

    pub fn toggle_host(&mut self, idx: usize) {
        if self.selected_hosts.contains(&idx) {
            self.selected_hosts.retain(|&i| i != idx);
        } else {
            self.selected_hosts.push(idx);
        }
    }

    pub fn get_selected_hosts(&self) -> Vec<&HostDetection> {
        self.selected_hosts
            .iter()
            .filter_map(|&i| self.hosts.get(i))
            .collect()
    }

    pub fn generate_snippets(&self) -> Vec<(HostKind, String)> {
        self.get_selected_hosts()
            .iter()
            .map(|h| {
                let snippet = generate_snippet(h.kind, &self.binary_path, &self.memex_cfg.db_path);
                (h.kind, snippet)
            })
            .collect()
    }

    pub fn run_health_check(&mut self) {
        self.health_status = Some("Checking...".to_string());

        // Try to run rmcp_memex --version
        match std::process::Command::new(&self.binary_path)
            .arg("--version")
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    self.health_status = Some(format!("✓ Binary OK: {}", version.trim()));
                } else {
                    self.health_status = Some("✗ Binary found but failed to run".to_string());
                }
            }
            Err(e) => {
                self.health_status = Some(format!("✗ Binary not found: {}", e));
            }
        }

        // Check db_path
        let expanded_path = shellexpand::tilde(&self.memex_cfg.db_path).to_string();
        let db_path = PathBuf::from(&expanded_path);
        if db_path.exists() {
            self.messages
                .push(format!("✓ DB path exists: {}", expanded_path));
        } else {
            self.messages
                .push(format!("○ DB path will be created: {}", expanded_path));
        }
    }

    pub fn write_configs(&mut self) -> Result<()> {
        if self.dry_run {
            self.messages.push("DRY RUN: No files written".to_string());
            return Ok(());
        }

        for &idx in &self.selected_hosts.clone() {
            if let Some(host) = self.hosts.get(idx) {
                let snippet =
                    generate_snippet(host.kind, &self.binary_path, &self.memex_cfg.db_path);

                // Create backup
                if host.exists {
                    let backup_path = format!("{}.bak.{}", host.path.display(), chrono_timestamp());
                    if let Err(e) = std::fs::copy(&host.path, &backup_path) {
                        self.messages.push(format!(
                            "Warning: backup failed for {}: {}",
                            host.kind.display_name(),
                            e
                        ));
                    } else {
                        self.messages
                            .push(format!("✓ Backup created: {}", backup_path));
                    }
                }

                // Write config (append or create)
                // For safety, we only show the snippet - actual writing would need more careful merging
                self.messages.push(format!(
                    "Config snippet for {} (manual merge recommended):\n{}",
                    host.kind.display_name(),
                    snippet
                ));
            }
        }

        Ok(())
    }

    fn settings_field_count(&self) -> usize {
        5 // db_path, cache_mb, log_level, max_request_bytes, mode
    }

    pub fn get_field_value(&self, field: usize) -> String {
        match field {
            0 => self.memex_cfg.db_path.clone(),
            1 => self.memex_cfg.cache_mb.to_string(),
            2 => self.memex_cfg.log_level.clone(),
            3 => self.memex_cfg.max_request_bytes.to_string(),
            4 => self.memex_cfg.mode.clone(),
            _ => String::new(),
        }
    }

    pub fn set_field_value(&mut self, field: usize, value: String) {
        match field {
            0 => self.memex_cfg.db_path = value,
            1 => {
                if let Ok(v) = value.parse() {
                    self.memex_cfg.cache_mb = v;
                }
            }
            2 => self.memex_cfg.log_level = value,
            3 => {
                if let Ok(v) = value.parse() {
                    self.memex_cfg.max_request_bytes = v;
                }
            }
            4 => self.memex_cfg.mode = value,
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        if self.input_mode {
            match key {
                KeyCode::Enter => {
                    if let Some(field) = self.editing_field {
                        self.set_field_value(field, self.input_buffer.clone());
                    }
                    self.input_mode = false;
                    self.editing_field = None;
                    self.input_buffer.clear();
                }
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.editing_field = None;
                    self.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.step != WizardStep::Welcome {
                    self.prev_step();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Enter | KeyCode::Tab => {
                match self.step {
                    WizardStep::MemexSettings => {
                        // Enter edit mode for current field
                        self.input_mode = true;
                        self.editing_field = Some(self.focus);
                        self.input_buffer = self.get_field_value(self.focus);
                    }
                    WizardStep::HostSelection => {
                        if self.focus < self.hosts.len() {
                            self.toggle_host(self.focus);
                        }
                    }
                    WizardStep::HealthCheck => {
                        self.run_health_check();
                    }
                    WizardStep::Summary => {
                        if let Err(e) = self.write_configs() {
                            self.messages.push(format!("Error: {}", e));
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Right | KeyCode::Char('n') => self.next_step(),
            KeyCode::Left | KeyCode::Char('p') => self.prev_step(),
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = match self.step {
                    WizardStep::MemexSettings => self.settings_field_count().saturating_sub(1),
                    WizardStep::HostSelection => self.hosts.len().saturating_sub(1),
                    _ => 0,
                };
                if self.focus < max {
                    self.focus += 1;
                }
            }
            KeyCode::Char(' ') => {
                if self.step == WizardStep::HostSelection && self.focus < self.hosts.len() {
                    self.toggle_host(self.focus);
                }
            }
            _ => {}
        }
    }
}

fn which_rmcp_memex() -> Option<String> {
    std::process::Command::new("which")
        .arg("rmcp_memex")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

/// Run the TUI wizard.
pub fn run_wizard(config: WizardConfig) -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new(config);

    let result = run_app(&mut terminal, &mut app);

    restore_terminal()?;
    result
}

fn run_app(terminal: &mut Tui, app: &mut App) -> Result<()> {
    use crate::tui::ui::render;

    loop {
        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
