# MCP Stack Quick Start

## 1. Clone and install

```bash
# Clone to any directory you like
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite

# Install MCP binaries (rmcp-mux, rmcp-mux-proxy, loctree-mcp, rmcp_memex)
make mcp-install
```

Alternatively, for the full suite including `loct` and `loctree` CLI:
```bash
make install-all
```

## 2. Setup directories

```bash
make mux-setup
```

Creates:
```
~/.rmcp_servers/
├── config/mux.toml      # <-- EDIT THIS
├── logs/
├── pids/
└── sockets/
```

## 3. Configure mux.toml

Copy example config (run from repo root):
```bash
cp docs/dev/.TL_DR/config-files/mux.toml ~/.rmcp_servers/config/mux.toml
```

**IMPORTANT**: Edit `~/.rmcp_servers/config/mux.toml` and replace all occurrences of:
- `/Users/YOURUSER/` → your actual home path (e.g., `/Users/monika/`)
- `YOUR_BRAVE_API_KEY_HERE` → your Brave Search API key
- `YOUR_GITHUB_TOKEN_HERE` → your GitHub personal access token

## 4. Configure Claude Code

Add to `~/.claude.json` (merge into existing `mcpServers` section):
```bash
cat docs/dev/.TL_DR/config-files/.claude/.claude.json
```

## 5. Start rmcp-mux

```bash
make mux-start
```

This starts a **single rmcp-mux process** that manages ALL servers from `mux.toml`.

## 6. Verify

```bash
# Quick check via Makefile
make mux-status

# Or query the daemon directly for detailed info
rmcp-mux daemon-status
```

Expected output from `daemon-status`:
```
rmcp-mux v0.3.4 | uptime: 5s
────────────────────────────────────────────────────────────────────────
Server                State    Clients  Pending   Restarts  Heartbeat
────────────────────────────────────────────────────────────────────────
brave-search         ✓ START       0/3        0          0          -
loctree                ✓ UP        0/5        0          0          -
rmcp-memex             ✓ UP        0/5        0          0          -
youtube-transcript   ✓ START       0/3        0          0          -
────────────────────────────────────────────────────────────────────────
Total: 4 servers (4 running, 0 errors)
```

Legend: `UP` = running, `START` = lazy (starts on first connection), `FAIL` = error

## Commands

| Command | Description |
|---------|-------------|
| `make mux-start` | Start rmcp-mux (manages all servers) |
| `make mux-stop` | Stop rmcp-mux |
| `make mux-restart` | Restart rmcp-mux |
| `make mux-status` | Show status of all servers |
| `make mux-kill` | Force kill rmcp-mux |
| `make mux-tui` | Launch TUI interface |
| `make mux-restart-service SERVICE=name` | Restart a single service |
| `rmcp-mux daemon-status` | Query running daemon status |
| `rmcp-mux daemon-status --json` | Get status as JSON |
| `make mux-logs` | Tail mux.log (live) |
| `make mcp-health` | Health check sockets |

## TUI Interface

Launch the multi-server TUI dashboard:
```bash
make mux-tui
```

Keybindings:
- `j/k` or arrows - Navigate servers
- `r` - Restart selected server
- `s` - Stop selected server
- `S` - Start selected server
- `q` - Quit

## CLI Flags

Direct `rmcp-mux` usage:

```bash
# Start all servers from config (default)
rmcp-mux --config ~/.rmcp_servers/config/mux.toml

# Start only specific servers
rmcp-mux --config mux.toml --only loctree,rmcp-memex

# Start all except some servers
rmcp-mux --config mux.toml --except youtube-transcript

# Show status of all configured servers
rmcp-mux --show-status --config mux.toml

# Restart a specific service
rmcp-mux --restart-service memex --config mux.toml
```

## Running Multiple Instances of the Same Server

When you need multiple instances of the same MCP server (e.g., two rmcp-memex with different databases), **each instance MUST have a unique socket path**.

### Example: Two rmcp-memex instances

In `mux.toml`:
```toml
[servers.rmcp-memex]
socket = "~/.rmcp_servers/sockets/rmcp-memex.sock"  # Unique socket!
cmd = "/Users/silver/.cargo/bin/rmcp_memex"
args = ["serve", "--db-path", "~/.rmcp_servers/rmcp_memex/lancedb"]

[servers.rmcp-memex-ollama]
socket = "~/.rmcp_servers/sockets/rmcp-memex-ollama.sock"  # Different socket!
cmd = "/Users/silver/.cargo/bin/rmcp_memex"
args = ["serve", "--db-path", "/path/to/other/lancedb"]
```

In `~/.claude.json`:
```json
{
  "mcpServers": {
    "rmcp-memex": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "/Users/silver/.rmcp_servers/sockets/rmcp-memex.sock"]
    },
    "rmcp-memex-ollama": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "/Users/silver/.rmcp_servers/sockets/rmcp-memex-ollama.sock"]
    }
  }
}
```

### Key Rules

1. **Socket path must be unique per instance** - two servers cannot share a socket
2. **Server name in mux.toml must be unique** - this becomes the service name
3. **MCP server name in claude.json can be anything** - this is what Claude sees
4. **Same binary, different args** - the `cmd` can be identical, only args differ
5. **SLED_PATH required when sharing LanceDB** - see below

### Sharing LanceDB Between Instances

If multiple rmcp-memex instances point to the **same** LanceDB path (e.g., for different Claude sessions accessing shared knowledge), each instance needs its own sled K/V cache:

```toml
[servers.memex-session-1]
socket = "~/.rmcp_servers/sockets/memex-1.sock"
cmd = "rmcp_memex"
args = ["serve", "--db-path", "/shared/lancedb"]
env = { SLED_PATH = "~/.rmcp_servers/sled/memex-1" }  # Unique sled!

[servers.memex-session-2]
socket = "~/.rmcp_servers/sockets/memex-2.sock"
cmd = "rmcp_memex"
args = ["serve", "--db-path", "/shared/lancedb"]  # Same LanceDB OK
env = { SLED_PATH = "~/.rmcp_servers/sled/memex-2" }  # Different sled!
```

**Why?** Sled is an embedded K/V store that requires exclusive file locking. Without unique `SLED_PATH`, you'll get:
```
Error: could not acquire lock on ".../.sled/db": Resource temporarily unavailable
```

### Common Mistake

```toml
# WRONG - same socket for different instances!
[servers.memex-1]
socket = "~/.rmcp_servers/sockets/memex.sock"  # Same socket
cmd = "rmcp_memex"
args = ["serve", "--db-path", "/db1"]

[servers.memex-2]
socket = "~/.rmcp_servers/sockets/memex.sock"  # Conflict!
cmd = "rmcp_memex"
args = ["serve", "--db-path", "/db2"]
```

## Troubleshooting

### Server won't start
```bash
# Check logs
tail -100 ~/.rmcp_servers/logs/mux.log

# Common issue: wrong path in mux.toml
# Fix: use FULL paths, not ~/
```

### Sled lock conflict (multiple memex instances)
```
Error: could not acquire lock on ".../.sled/db": Resource temporarily unavailable
```
**Fix**: Add unique `SLED_PATH` env var for each instance sharing the same LanceDB.
See "Sharing LanceDB Between Instances" section above.

### Socket exists but server dead
```bash
make mux-kill   # removes pids and sockets
make mux-start
```

### Claude Code can't connect
1. Check server is running: `make mux-status`
2. Check socket exists: `ls ~/.rmcp_servers/sockets/`
3. Restart Claude Code

## Architecture

```
Claude Code
    │
    ▼ (spawns)
rmcp-mux proxy --socket ~/.rmcp_servers/sockets/loctree.sock
    │
    ▼ (Unix socket)
rmcp-mux daemon (SINGLE PROCESS - manages ALL servers)
    │
    ├── loctree-mcp (child process)
    ├── rmcp_memex (child process)
    ├── brave-search (child process)
    ├── sequential-thinking (child process)
    └── youtube-transcript (child process)
```

Benefits:
- **Single process** manages all MCP servers
- One PID to track, one log to tail
- Atomic start/stop/restart
- Centralized TUI dashboard
- Shared Tokio runtime (memory efficient)
- Fast reconnection (no cold start)
