# rmcp-mux Library Integration Guide

This guide explains how to embed `rmcp-mux` as a library in your Rust application to run MCP servers without spawning separate processes.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rmcp-mux = { version = "0.3", default-features = false }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

The `default-features = false` disables CLI-only dependencies (clap, ratatui, crossterm, tray-icon) for minimal footprint.

## Core Concepts

### MuxConfig

The `MuxConfig` struct configures a mux instance using the builder pattern:

```rust
use rmcp_mux::MuxConfig;
use std::time::Duration;

let config = MuxConfig::new("/tmp/my-service.sock", "npx")
    .with_args(vec!["-y".into(), "@modelcontextprotocol/server-memory".into()])
    .with_max_clients(10)
    .with_service_name("memory-service")
    .with_request_timeout(Duration::from_secs(60))
    .with_lazy_start(true);
```

### Configuration Options

| Method | Default | Description |
|--------|---------|-------------|
| `new(socket, cmd)` | required | Socket path and command to run |
| `with_args(vec)` | `[]` | Command arguments |
| `with_max_clients(n)` | `5` | Max concurrent active clients |
| `with_service_name(s)` | socket filename | Name for logging/status |
| `with_log_level(s)` | `"info"` | trace/debug/info/warn/error |
| `with_lazy_start(bool)` | `false` | Spawn server on first request |
| `with_max_request_bytes(n)` | `1048576` | Max request size (1MB) |
| `with_request_timeout(dur)` | `30s` | Request timeout |
| `with_restart_backoff(init, max)` | `1s, 30s` | Restart delay range |
| `with_max_restarts(n)` | `5` | Max restarts (0 = unlimited) |
| `with_status_file(path)` | none | JSON status snapshot path |
| `with_tray(bool)` | `false` | Enable tray icon (requires `tray` feature) |

## Usage Patterns

### Pattern 1: Single Blocking Server

Simplest usage - blocks until Ctrl+C:

```rust
use rmcp_mux::{MuxConfig, run_mux_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = MuxConfig::new("/tmp/mcp.sock", "my-mcp-server");
    run_mux_server(config).await
}
```

### Pattern 2: Multiple Servers in One Process

Run multiple MCP services sharing a single tokio runtime:

```rust
use rmcp_mux::{MuxConfig, spawn_mux_server, MuxHandle};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Define services
    let services = vec![
        ("memory", "/tmp/mcp-memory.sock", "npx", vec!["@mcp/server-memory"]),
        ("filesystem", "/tmp/mcp-fs.sock", "npx", vec!["@mcp/server-filesystem"]),
        ("brave", "/tmp/mcp-brave.sock", "npx", vec!["@mcp/server-brave-search"]),
    ];

    // Spawn all
    let mut handles: Vec<MuxHandle> = Vec::new();
    for (name, socket, cmd, args) in services {
        let config = MuxConfig::new(socket, cmd)
            .with_args(args.into_iter().map(String::from).collect())
            .with_service_name(name)
            .with_request_timeout(Duration::from_secs(60));

        handles.push(spawn_mux_server(config).await?);
        println!("Started {name} on {socket}");
    }

    // Wait for all (or Ctrl+C)
    for handle in handles {
        handle.wait().await?;
    }
    Ok(())
}
```

### Pattern 3: Programmatic Shutdown

Control server lifecycle from your application:

```rust
use rmcp_mux::{MuxConfig, spawn_mux_server};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let handle = spawn_mux_server(
        MuxConfig::new("/tmp/mcp.sock", "my-server")
    ).await?;

    // Do other work...
    sleep(Duration::from_secs(300)).await;

    // Graceful shutdown
    handle.shutdown();
    handle.wait().await?;

    Ok(())
}
```

### Pattern 4: External CancellationToken

Integrate with your own shutdown logic:

```rust
use rmcp_mux::{MuxConfig, run_mux_with_shutdown};
use tokio_util::sync::CancellationToken;
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let shutdown = CancellationToken::new();

    // Custom shutdown trigger
    let shutdown_trigger = shutdown.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        println!("Shutdown signal received");
        shutdown_trigger.cancel();
    });

    let config = MuxConfig::new("/tmp/mcp.sock", "my-server");
    run_mux_with_shutdown(config.into(), shutdown).await
}
```

### Pattern 5: Health Monitoring

Check if a mux socket is responsive:

```rust
use rmcp_mux::check_health;

async fn monitor_services() {
    let sockets = vec![
        "/tmp/mcp-memory.sock",
        "/tmp/mcp-fs.sock",
    ];

    for socket in sockets {
        match check_health(socket).await {
            Ok(_) => println!("{socket}: healthy"),
            Err(e) => println!("{socket}: unhealthy - {e}"),
        }
    }
}
```

### Pattern 6: Dynamic Service Loading

Load services from configuration:

```rust
use rmcp_mux::{MuxConfig, spawn_mux_server, MuxHandle};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ServiceDef {
    socket: String,
    command: String,
    args: Vec<String>,
    #[serde(default = "default_max_clients")]
    max_clients: usize,
}

fn default_max_clients() -> usize { 5 }

async fn load_services(config_path: &str) -> anyhow::Result<Vec<MuxHandle>> {
    let content = tokio::fs::read_to_string(config_path).await?;
    let services: HashMap<String, ServiceDef> = toml::from_str(&content)?;

    let mut handles = Vec::new();
    for (name, svc) in services {
        let config = MuxConfig::new(&svc.socket, &svc.command)
            .with_args(svc.args)
            .with_max_clients(svc.max_clients)
            .with_service_name(&name);

        handles.push(spawn_mux_server(config).await?);
    }
    Ok(handles)
}
```

## MuxHandle API

The `MuxHandle` returned by `spawn_mux_server` provides:

| Method | Description |
|--------|-------------|
| `shutdown()` | Request graceful shutdown (non-blocking) |
| `wait().await` | Wait for server to terminate |
| `is_running()` | Check if server is still active |

## Status Monitoring

Enable JSON status snapshots for external monitoring:

```rust
let config = MuxConfig::new("/tmp/mcp.sock", "server")
    .with_status_file("/var/run/mcp-status.json");
```

Status file format:

```json
{
  "service_name": "memory",
  "server_status": "Running",
  "restarts": 0,
  "connected_clients": 3,
  "active_clients": 1,
  "max_active_clients": 5,
  "pending_requests": 2,
  "cached_initialize": true,
  "queue_depth": 5,
  "child_pid": 12345
}
```

## Error Handling

All async functions return `anyhow::Result`. Common errors:

| Error | Cause | Solution |
|-------|-------|----------|
| "failed to bind socket" | Socket path in use | Remove stale socket or use different path |
| "failed to spawn child" | Command not found | Verify `cmd` is in PATH |
| "request timeout" | Server unresponsive | Increase `request_timeout` or check server |
| "max restarts exceeded" | Server keeps crashing | Check server logs, increase `max_restarts` |

## Feature Flags

| Feature | Default | Adds |
|---------|---------|------|
| `cli` | yes | CLI binary, wizard, scan (clap, ratatui, crossterm) |
| `tray` | yes | System tray icon (tray-icon, image) |

For library-only (minimal deps):

```toml
[dependencies]
rmcp-mux = { version = "0.3", default-features = false }
```

## Thread Safety

- `MuxConfig` is `Clone + Send + Sync`
- `MuxHandle` is `Send` but not `Clone` (unique ownership of shutdown token)
- Multiple mux instances can run concurrently in the same process
- Each mux manages its own child process and client connections

## Best Practices

1. **Use lazy_start for optional services** - Don't spawn servers until needed
2. **Set appropriate timeouts** - Match your use case (AI calls may need 60s+)
3. **Monitor with status_file** - Essential for production deployments
4. **Handle shutdown gracefully** - Always call `shutdown()` before dropping handles
5. **Use meaningful service names** - Helps with logging and debugging
6. **Set max_restarts = 0 for critical services** - Unlimited retries for must-have services

## Migration from CLI

If you were running `rmcp-mux` as a separate process:

**Before (CLI):**
```bash
rmcp-mux --socket /tmp/mcp.sock --cmd npx -- @mcp/server-memory
```

**After (Library):**
```rust
let config = MuxConfig::new("/tmp/mcp.sock", "npx")
    .with_args(vec!["-y".into(), "@mcp/server-memory".into()]);
run_mux_server(config).await?;
```

All CLI flags map directly to `MuxConfig` builder methods.

## Troubleshooting

### Socket already in use
```rust
// Remove stale socket before starting
let _ = std::fs::remove_file("/tmp/mcp.sock");
let config = MuxConfig::new("/tmp/mcp.sock", "server");
```

### Child process not starting
```rust
// Enable debug logging
let config = MuxConfig::new("/tmp/mcp.sock", "server")
    .with_log_level("debug");
```

### Requests timing out
```rust
// Increase timeout for slow operations
let config = MuxConfig::new("/tmp/mcp.sock", "server")
    .with_request_timeout(Duration::from_secs(120));
```

## Version Compatibility

| rmcp-mux | Rust | tokio | rmcp |
|----------|------|-------|------|
| 0.3.x | 1.70+ | 1.x | 0.9.x |
