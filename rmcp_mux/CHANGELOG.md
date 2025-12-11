# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2025-12-04

### Added
- **Library-first architecture** – rmcp-mux is now an embeddable Rust library, not just a CLI tool.
- `MuxConfig` builder for programmatic configuration:
  ```rust
  let config = MuxConfig::new("/tmp/mcp.sock", "npx")
      .with_args(vec!["@mcp/server-memory".into()])
      .with_max_clients(10);
  ```
- `run_mux_server(config)` – blocking entry point for single mux server.
- `spawn_mux_server(config)` – non-blocking spawn returning `MuxHandle` for lifecycle control.
- `MuxHandle` with `shutdown()`, `wait()`, `is_running()` methods.
- `run_mux_with_shutdown(params, token)` – external `CancellationToken` support for custom shutdown logic.
- `check_health(socket_path)` – simple health check function.
- `CliOptions` trait for generic CLI parameter handling.
- `docs/integration.md` – comprehensive library integration guide.
- Feature flags: `cli` (wizard, scan, binaries) and `tray` (system tray icon).

### Changed
- **Package renamed** from `rmcp_mux` to `rmcp-mux` (crates.io convention).
- **Binary renamed** from `rmcp_mux` to `rmcp-mux`, proxy from `rmcp_mux_proxy` to `rmcp-mux-proxy`.
- **Library name** remains `rmcp_mux` (Rust identifier requirement) – use `use rmcp_mux::*` in code.
- Project structure reorganized:
  - `src/lib.rs` – new library entry point with public API.
  - `src/bin/rmcp_mux.rs` – CLI binary (requires `cli` feature).
  - `src/bin/rmcp_mux_proxy.rs` – proxy binary (requires `cli` feature).
- `runtime/mod.rs` split: `run_mux` now delegates to `run_mux_internal` with external shutdown support.
- `config.rs`: `resolve_params` now generic over `CliOptions` trait.
- Default features: `["cli", "tray"]` – use `default-features = false` for library-only.

### Migration Guide
**From CLI to Library:**
```rust
// Before: rmcp-mux --socket /tmp/mcp.sock --cmd npx -- @mcp/server
// After:
use rmcp_mux::{MuxConfig, run_mux_server};
let config = MuxConfig::new("/tmp/mcp.sock", "npx")
    .with_args(vec!["@mcp/server".into()]);
run_mux_server(config).await?;
```

**Multiple servers in one process:**
```rust
use rmcp_mux::{MuxConfig, spawn_mux_server};
let h1 = spawn_mux_server(MuxConfig::new("/tmp/a.sock", "server-a")).await?;
let h2 = spawn_mux_server(MuxConfig::new("/tmp/b.sock", "server-b")).await?;
// Both run in single process, sharing tokio runtime
```

## [0.2.1] - 2025-11-27

### Added
- **Three-step wizard flow** for comprehensive MCP configuration:
  - **Step 1: Server Detection** – Detects running MCP server processes via `ps`, loads existing config, allows selection with `Space`, shows health status indicators.
  - **Step 2: Client Detection** – Discovers MCP client applications (Codex, Cursor, VSCode, Claude, JetBrains), shows rewire status, allows selection for rewiring.
  - **Step 3: Confirmation** – Summary of selections with save options: Save All, Mux Only, Clipboard, Back, Exit.
- Clipboard support (`pbcopy` on macOS) for copying config without writing files.
- Client rewiring functionality – automatically updates client configs to use `rmcp_mux_proxy`.
- Health status indicators in wizard: green dot (healthy), red dot (unhealthy), gray circle (unknown).
- Source indicators in wizard: `[C]` for config-based servers, `[D]` for detected processes.

### Changed
- **Major refactoring** of `wizard.rs` (1829 LOC) into modular structure:
  - `wizard/types.rs` – Enums and structs (WizardStep, Field, Panel, ServiceEntry, ClientEntry, etc.)
  - `wizard/services.rs` – Service loading, MCP process detection, health checks
  - `wizard/clients.rs` – Client (host application) detection
  - `wizard/ui.rs` – All UI drawing functions (ratatui)
  - `wizard/keys.rs` – Key event handling
  - `wizard/persist.rs` – Config persistence and client rewiring
  - `wizard/mod.rs` – Entry point and re-exports
- **Major refactoring** of `runtime.rs` (1596 LOC) into modular structure:
  - `runtime/types.rs` – ServerEvent and constants (MAX_QUEUE, MAX_PENDING)
  - `runtime/client.rs` – Client connection handling
  - `runtime/server.rs` – MCP child process management with restart logic
  - `runtime/proxy.rs` – STDIO proxy for mux socket
  - `runtime/status.rs` – Status file writing
  - `runtime/mod.rs` – Main mux loop, health check, timeout reaper
  - `runtime/tests.rs` – All runtime tests (753 LOC)
- Improved wizard navigation: `n` for next step, `p` for previous step.
- Backup files (`.bak`) created for all modified configs.

### Fixed
- Redundant `scan_host_file` calls in client detection – now scans once and reuses result.

## [0.2.0] - 2025-11-24

### Added
- Optional tray icon (`--tray`) showing live server status, client and pending counts, and restart reasons. ([5eefde4](https://github.com/Loctree/rmcp-mux/commit/5eefde4))
- Config file support (JSON/YAML/TOML) with auto-detection and CLI overrides. ([5eefde4](https://github.com/Loctree/rmcp-mux/commit/5eefde4))
- `rmcp_mux_proxy` helper binary plus launchd template and installer tweaks for easier setup. ([04e5402](https://github.com/Loctree/rmcp-mux/commit/04e5402))
- GitHub Actions CI workflow for formatting, linting, testing, and coverage, including an async proxy forwarding test. ([ad2b9aa](https://github.com/Loctree/rmcp-mux/commit/ad2b9aa))
- Mux hooks, Semgrep rules, and expanded README documentation. ([e80083c](https://github.com/Loctree/rmcp-mux/commit/e80083c))
- `health` subcommand to resolve config and assert socket reachability, plus unit tests for healthy/missing sockets.

### Changed
- Refactored mux state management and tray functionality into dedicated `state` and `tray` modules, with tray dependencies gated behind an optional `tray` feature; CI updated to run with `--no-default-features`. ([0d60764](https://github.com/Loctree/rmcp-mux/commit/0d60764), [ad2b9aa](https://github.com/Loctree/rmcp-mux/commit/ad2b9aa))

## [0.1.5] - 2025-11-20

### Added
- JSON status snapshots (`--status-file` / `status_file`) including PID, queue depth, request limits, restart/backoff settings.
- Hardened runtime: lazy child start, request size guard, request timeouts, capped restart backoff, max restarts.
- Status writer task for tray/automation; MuxState now tracks queue depth and child PID.

### Changed
- Config/Wizard/Scan updated to surface new fields; defaults documented in README.
- Tests cover initialize cache, resets, status snapshots, and proxy; CI runs fmt/clippy/tests/tarpaulin with `--no-default-features` (tray off in CI).

## [0.1.0] - 2025-11-15

### Added
- Initial release of rmcp-mux.
- Single MCP server child process management.
- Unix socket listener for multiple clients.
- JSON-RPC ID rewriting per client.
- Initialize request caching and fan-out.
- Child process restart on failure.
- Basic CLI interface with `--socket`, `--cmd`, `--max-active-clients`, `--log-level`.
