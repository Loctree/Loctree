# rmcp-mux Development Guidelines

This document is intended for an industry specialist who may not be a day‑to‑day programmer but is comfortable with
command‑line tools and configuration files. It explains how to build, configure, test, and extend the `rmcp-mux` project
**based on the Rust source code itself**.

The project is a **Rust** workspace built with **Cargo** (Rust’s package manager and build tool). It provides:

- a main binary called `rmcp-mux` – a robust MCP mux that manages a single MCP server process and many clients over a
  Unix socket; and
- a helper binary `rmcp_mux_proxy` – a lightweight proxy that connects standard input/output (STDIN/STDOUT) to the mux’s
  Unix socket.

---

## 1. Build & Configuration Instructions

### 1.1. Prerequisites

From the code, we can see this is a standard Rust project (`Cargo.toml` with `edition = "2021"`). To work with it you
need:

1. **Rust toolchain** (compiler + Cargo)
    - Recommended installation method: use the official [rustup](https://rustup.rs/) installer.
    - On macOS or Linux this usually looks like:

      ```bash
      curl https://sh.rustup.rs -sSf | sh
      ```

    - After installation, open a new terminal and verify:

      ```bash
      rustc --version
      cargo --version
      ```

    - You should see version numbers printed (exact numbers don’t matter, but Rust 1.70+ is a good baseline for a
      2021‑edition project).

2. **A Unix‑like environment**
    - The code uses Unix domain sockets (`tokio::net::UnixListener`, `UnixStream`) and paths like `/tmp/...` or `~/...`.
      This is typical for macOS and Linux.
    - Windows is not explicitly supported in the code; if you need Windows support, expect extra work.

3. **Network and process permissions**
    - To run MCP servers (for example via `npx` or another CLI), you need permission to run child processes.
    - To create Unix sockets, your user must be able to create files in the chosen directory (e.g., `/tmp/mcp-sockets`).

### 1.2. Project layout (code‑based view)

Key paths mentioned here are taken directly from the Rust source tree:

- `Cargo.toml` – package definition and dependencies.
- `src/main.rs` – entry point for the `rmcp-mux` CLI.
- `src/config.rs` – configuration types and logic (`Config`, `ServerConfig`, `ResolvedParams`, `load_config`,
  `resolve_params`).
- `src/runtime.rs` – core runtime: Unix socket listener, client handling, server process management, timeouts, health
  checks, status writer.
- `src/state.rs` – shared state (`MuxState`, `StatusSnapshot`, `ServerStatus`) and helper functions.
- `src/scan.rs` – host configuration scanning and rewiring commands (for various tools/hosts).
- `src/tray.rs` – tray‑icon support (compiled only when the `tray` feature is enabled).
- `src/wizard.rs` – interactive terminal wizard (built with the `ratatui` and `crossterm` crates).
- `src/bin/rmcp_mux_proxy.rs` – the proxy binary.

You do **not** need to understand every module to build the project, but it is useful to know these names when reading
error messages or tests.

### 1.3. Building the binaries

All builds are driven through Cargo. From the project root (`/Users/maciejgad/.rmcp_servers/rmcp-mux` in the current
setup):

#### 1.3.1. Debug build (fast, for development)

```bash
cargo build
```

This compiles:

- `rmcp-mux` – main mux binary
- `rmcp_mux_proxy` – proxy binary (because there is a file under `src/bin/`)

The resulting binaries will typically be placed under `target/debug/`:

- `target/debug/rmcp-mux`
- `target/debug/rmcp_mux_proxy`

You can then run them directly, for example:

```bash
target/debug/rmcp-mux --help
target/debug/rmcp_mux_proxy --help
```

#### 1.3.2. Release build (optimized, for production use)

```bash
cargo build --release
```

This produces optimized binaries under `target/release/`:

- `target/release/rmcp-mux`
- `target/release/rmcp_mux_proxy`

Release builds take longer to compile but run faster and are suited to long‑running deployments.

#### 1.3.3. Optional tray feature

The `Cargo.toml` file defines a feature section:

```toml
[features]
default = ["tray"]
tray = ["tray-icon", "image"]
```

- By default, the **`tray` feature is enabled**, meaning the project links the tray‑icon and image libraries and enables
  tray‑related code (`src/tray.rs`, and certain branches in `src/runtime.rs`).
- If you encounter build issues related to GUI/tray libraries, or if you don’t need a tray icon (for example on a
  headless server), you can **disable default features**:

  ```bash
  cargo build --no-default-features
  ```

  or, for a release build without tray:

  ```bash
  cargo build --release --no-default-features
  ```

In the current codebase, only one explicit feature is defined (`tray`), so disabling default features primarily affects
tray support.

### 1.4. Running `rmcp-mux`

The CLI is defined in `src/main.rs` using the `clap` library. At a high level there are two modes:

1. **Subcommand mode** – you call one of the explicit subcommands like `wizard`, `scan`, `rewire`, `proxy`, `status`, or
   `health`.
2. **Mux mode (no subcommand)** – you run the main mux server itself.

You can always see the current CLI options by running:

```bash
target/debug/rmcp-mux --help
```

#### 1.4.1. Running the mux using only CLI flags

If you do **not** use an external configuration file, the mux expects at least:

- a Unix socket path (`--socket`); and
- a command to run your MCP server (`--cmd`), followed by **the arguments for that command**.

From `src/main.rs` and `src/config.rs` we can see the relevant CLI fields:

- `--socket <PATH>` – path to the Unix socket where clients will connect.
- `--cmd <STRING>` – command used to start the MCP server (e.g. `npx`).
- `-- ...` – everything after `--` is passed as `args` to the MCP server command.

Example (using a hypothetical npm‑based MCP server):

```bash
target/debug/rmcp-mux \
  --socket /tmp/memory.sock \
  --cmd npx \
  -- @mcp/server-memory --some-server-flag
```

Explanation:

- `--socket /tmp/memory.sock` tells the mux where to create its Unix socket.
- `--cmd npx` indicates that the mux starts the server by running the `npx` command.
- The `--` (double dash) marks the end of mux options; everything after it (`@mcp/server-memory --some-server-flag`) is
  passed directly as arguments to `npx`.

Additional optional flags (see `struct Cli` in `src/main.rs`):

- `--max-active-clients <N>` – maximum number of concurrent clients (default `5`).
- `--lazy-start <true|false>` – if `true`, only start the child MCP server when the first client connects.
- `--max-request-bytes <N>` – maximum request size; default is `1_048_576` bytes (1 MiB).
- `--request-timeout-ms <N>` – per‑request timeout in milliseconds; default is `30000` (30 seconds).
- `--restart-backoff-ms <N>` / `--restart-backoff-max-ms <N>` – initial and maximum backoff between restarts.
- `--max-restarts <N>` – how many times to restart the server before giving up (default `5`).
- `--log-level <LEVEL>` – logging level; defaults to `info`. `runtime.rs` uses this to configure `tracing_subscriber`’s
  max log level.
- `--tray` / `--service-name` – tray‑related options (only useful when the `tray` feature is compiled in).
- `--status-file <PATH>` – optional file to which JSON status snapshots will be written.

#### 1.4.2. Running the mux using a config file

The configuration module (`src/config.rs`) defines:

- `Config` – top‑level structure with a map of services: `servers: HashMap<String, ServerConfig>`.
- `ServerConfig` – per‑service settings such as `socket`, `cmd`, `args`, `max_active_clients`, `tray`, `service_name`,
  `log_level`, and the various timeout/backoff fields.
- `load_config(path: &Path)` – reads and parses the config file.
- `resolve_params(cli: &Cli, config: Option<&Config>)` – merges CLI arguments and config data into a `ResolvedParams`
  struct.

Supported config formats are detected **by file extension** in `load_config`:

- `.json` – parsed as JSON
- `.yaml` / `.yml` – parsed as YAML
- `.toml` – parsed as TOML

Anything else defaults to JSON parsing. If the path does not exist, `load_config` returns `Ok(None)` (see tests in
`runtime.rs`).

When you use `--config`, the code requires **also** providing a service key via `--service` (enforced in
`resolve_params`):

```bash
target/debug/rmcp-mux \
  --config ~/.codex/mcp.json \
  --service memory
```

Internally, `resolve_params`:

1. Reads the config file into a `Config` if it exists.
2. Looks up the entry `servers["memory"]` and clones it.
3. Merges CLI flags and config values, giving CLI precedence when both are present.
4. Validates that it has a socket path and a command, otherwise it returns an error.

**Important rule from the code:**

- If a config file is present (`Some(Config)`), but no `--service` was provided, the program returns an error:
  `"--service is required when using --config"`.

##### 1.4.2.1. Minimal JSON example (derived from the types)

A minimal JSON config for a single server named `memory` could look like this:

```json
{
  "servers": {
    "memory": {
      "socket": "/tmp/memory.sock",
      "cmd": "npx",
      "args": [
        "@mcp/server-memory"
      ]
    }
  }
}
```

You would then run:

```bash
target/debug/rmcp-mux --config ~/.codex/mcp.json --service memory
```

Here, `socket` and `cmd` are required for this service to be usable; other fields (`max_active_clients`, `tray`,
`log_level`, etc.) are optional and fall back to defaults if omitted.

#### 1.4.3. Path expansion (`~/...`)

The `expand_path` function in `src/config.rs` implements a simple `~/` expansion:

- if a path string starts with `~/`, it replaces `~` with the value of the `HOME` environment variable;
- otherwise, it uses the path as‑is.

This behavior is used for config paths, socket paths, and status file paths that come from the configuration. It does *
*not** implement full shell‑style expansion (no `$VAR` support), only the `~/` pattern.

### 1.5. Helper binary: `rmcp_mux_proxy`

The file `src/bin/rmcp_mux_proxy.rs` defines a separate binary which:

- connects to a Unix socket (`UnixStream::connect(socket)`) and
- forwards everything from STDIN to the socket and everything from the socket to STDOUT.

This is useful when a tool expects an MCP server on STDIO, while `rmcp-mux` exposes a Unix socket.

Example usage (after building):

```bash
target/debug/rmcp_mux_proxy --socket /tmp/memory.sock
```

In many host configurations, this proxy path can be what you set as the "command" for an MCP server.

---

## 2. Testing Information

The project is already heavily tested. The tests are written directly inside modules using `#[cfg(test)]` and Rust’s
built‑in test framework (`cargo test`). There are also async tests using the `tokio::test` macro.

From `Cargo.toml` we can see one dev‑dependency:

```toml
[dev-dependencies]
tempfile = "3"
```

This is used inside tests (for example in `runtime.rs` and `scan.rs`) to create temporary directories and files.

### 2.1. Running the full test suite

From the project root:

```bash
cargo test
```

This will:

- build the project in **test** mode, and
- run all unit tests defined inside the Rust modules and binaries.

On the current codebase, `cargo test` runs tests in (at least):

- `src/runtime.rs` (async tests for mux logic and configuration loading)
- `src/scan.rs` (tests for config scanning, manifest building, rewiring)
- `src/state.rs` (a simple state test – see below)
- `src/tray.rs` (tray UI/helpers, when the `tray` feature is compiled)
- `src/bin/rmcp_mux_proxy.rs` (proxy behavior)

The test command output (from an actual run) shows dozens of tests passing, which confirms that the test harness is
working correctly.

#### Notes for non‑programmer specialists

- `cargo test` both compiles **and** runs tests; you do not need to run `cargo build` first.
- If tests fail, Cargo will print error messages and a backtrace. These can be shared with developers for
  troubleshooting.
- Some tests are asynchronous (`#[tokio::test]`); Cargo and Tokio handle async runtime setup automatically.

### 2.2. Example test added for demonstration

To demonstrate how to add a simple test, the following unit test was added to `src/state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_request_id_increments_sequentially() {
        let mut state = MuxState::new(
            5,
            "test-service".into(),
            1_048_576,
            Duration::from_secs(30),
            Duration::from_millis(1_000),
            Duration::from_millis(30_000),
            5,
            0,
            None,
        );

        let first = state.next_request_id();
        let second = state.next_request_id();

        assert_eq!(first + 1, second);
    }
}
```

What it does:

- Constructs an in‑memory `MuxState` using the public `MuxState::new` constructor.
- Calls `next_request_id()` twice.
- Asserts that the second ID is exactly one greater than the first, confirming sequential numbering.

This test is **purely in memory** – it does not touch the filesystem, spawn subprocesses, or use the network. This makes
it safe and stable as a demonstration.

You can run only this test (instead of the whole suite) with:

```bash
cargo test next_request_id_increments_sequentially
```

Cargo matches test names by substring, so this will pick up the `state::tests::next_request_id_increments_sequentially`
test.

### 2.3. Running specific test modules or functions

Rust’s test harness (through Cargo) lets you run a subset of tests by pattern. Some practical examples, all based on
functions present in the code:

- Run only state tests:

  ```bash
  cargo test state::tests
  ```

- Run only scan‑related tests:

  ```bash
  cargo test scan::tests
  ```

- Run only the proxy test (`proxy_forwards_bytes` in `src/bin/rmcp_mux_proxy.rs`):

  ```bash
  cargo test proxy_forwards_bytes
  ```

### 2.4. Adding new tests – practical guidelines

#### 2.4.1. Unit tests inside modules

Most existing tests in this project are **unit tests declared inside the module they test**, behind a `#[cfg(test)]`
guard. The general pattern looks like this (as seen in `runtime.rs`, `scan.rs`, `state.rs`, and `rmcp_mux_proxy.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_behavior_is_correct() {
        // Arrange: build input data
        // Act: call the function under test
        // Assert: use assert_eq!, assert!(...), etc.
    }
}
```

Key points:

- `#[cfg(test)]` ensures the `tests` module is **only compiled when running tests**, not in production builds.
- `use super::*;` imports items from the parent module (functions, structs, etc.) so tests can call them directly.
- The `#[test]` attribute marks regular, synchronous tests.
- For async tests, `#[tokio::test]` is used instead (as in multiple functions in `runtime.rs`).

##### When to add unit tests

Based on current code patterns, good candidates include:

- Pure functions that transform data (`snapshot_for_state` in `src/state.rs`).
- Error handling behavior (`error_response` formats errors in a standard JSON‑RPC style).
- Configuration merging and defaults (`resolve_params` in `src/config.rs` – already well‑covered by existing tests in
  `runtime.rs`).

#### 2.4.2. Integration tests in a separate `tests/` directory

The current codebase uses module‑internal tests only; there is no `tests/` directory yet. Rust, however, supports *
*integration tests** placed under a top‑level `tests/` directory.

To add one:

1. Create a directory `tests/` at the project root (alongside `src/`).
2. Add a file like `tests/smoke.rs` with contents:

   ```rust
   #[test]
   fn smoke_test_binary_compiles_and_runs_help() {
       // This is a placeholder to show where integration tests would go.
       // A realistic integration test might spawn the binary via std::process::Command
       // and assert on its exit code or output.
       assert!(true);
   }
   ```

3. Run:

   ```bash
   cargo test
   ```

In practice, for more realistic integration tests you would:

- use `std::process::Command` to invoke `target/debug/rmcp-mux --help`, and
- assert that the command exits successfully and prints the expected help text.

But note that this requires the binary to be buildable on the local platform; for continuous integration this is normal,
but on constrained systems it might require adjustments.

### 2.5. Test configuration and environment

Some tests in `runtime.rs` and `scan.rs` interact with the filesystem (using `tempfile::tempdir` and `std::fs`). From
the code we can infer:

- tests create temporary directories and files under the system temp directory (via `tempfile` and `env::temp_dir()`);
- they clean up automatically when the test process exits (or explicitly via `remove_file`).

General recommendations:

- Run tests as a regular user (not as root), so that any accidental writes stay within your user’s temp directories.
- Ensure the system’s temporary directory (`/tmp` on most Unix systems) is writable.

---

## 3. Additional Development & Debugging Information

### 3.1. Code style and conventions

From the codebase we can observe several conventions:

1. **Rust 2021 edition**
    - Modern Rust idioms are used: `Result<T, anyhow::Error>`, `async`/`await`, `tokio` runtime, and `clap` for CLI
      parsing.

2. **Error handling with `anyhow`**
    - Functions that can fail generally return `anyhow::Result<T>`.
    - The `anyhow!` macro is used to create user‑friendly error messages.
    - The `Context` trait (`with_context(|| ...)`) is used when reading/parsing config files to add path information to
      errors.

3. **Logging with `tracing`**
    - The mux initializes logging in `main` via
      `tracing_subscriber::fmt().with_max_level(level).with_target(false).init();`.
    - Log messages use structured fields (for example in `runtime.rs`):

      ```rust
      tracing::info!(
          service = params.service_name.as_str(),
          socket = %params.socket.display(),
          cmd = %params.cmd,
          max_clients = params.max_clients,
          tray = params.tray_enabled,
          "mux starting"
      );
      ```

    - When debugging issues, increasing `--log-level` to `debug` can provide more insight.

4. **Async concurrency with Tokio**
    - The main async runtime uses `#[tokio::main]` in `src/main.rs` and `src/bin/rmcp_mux_proxy.rs`.
    - Internally, components use:
        - `tokio::net::UnixListener` / `UnixStream` for sockets.
        - `tokio::sync::mpsc` channels for internal messaging.
        - `tokio::sync::watch` channels for publishing status updates.
        - `tokio::sync::Semaphore` to limit active client count.
        - `tokio::time::sleep` and `Duration` values for backoff and timeouts.

5. **State management**
    - Shared state is encapsulated in `MuxState` (`src/state.rs`), wrapped in `Arc<Mutex<MuxState>>` when shared across
      async tasks.
    - Status snapshots are represented by `StatusSnapshot` and marshalled as JSON via `serde`.

6. **Configuration merging**
    - CLI values override config values; config values override built‑in defaults.
    - Defaults for key parameters (from `resolve_params`):
        - `max_active_clients`: CLI default is `5`.
        - `lazy_start`: default `false`.
        - `max_request_bytes`: default `1_048_576` (1 MiB).
        - `request_timeout_ms`: default `30_000` (30 seconds).
        - `restart_backoff_ms`: default `1_000` (1 second).
        - `restart_backoff_max_ms`: default `30_000` (30 seconds).
        - `max_restarts`: default `5`.

7. **ID management**
    - `MuxState::next_request_id` increments a `next_global_id` counter.
    - `state::set_id` updates JSON‑RPC messages by setting the `"id"` field in a `serde_json::Value` object, used in
      tests and runtime.

### 3.2. Debugging and observability

#### 3.2.1. Health checks

The `health` subcommand in `src/main.rs` uses the `health_check` function from `runtime.rs`:

```bash
target/debug/rmcp-mux health --socket /tmp/memory.sock --cmd npx -- @mcp/server-memory
```

In practice, this subcommand:

- resolves configuration (similarly to regular mux startup), and
- attempts to connect to the specified Unix socket.

If successful, `run_health` prints:

```text
OK: connected to <socket-path>
```

If the socket is missing or not listening, it returns an error. There are tests in `runtime.rs` verifying both success
and failure cases.

#### 3.2.2. Status file

If `ResolvedParams.status_file` is set (via `--status-file` or config), the runtime uses a
`watch::Sender<StatusSnapshot>` and the `spawn_status_writer` helper to keep a JSON file up to date with status
snapshots.

This is useful for external monitoring tools that can:

- read the JSON file periodically,
- extract state such as `server_status`, `connected_clients`, `active_clients`, and
- show or alert based on changes.

The format is driven by the `StatusSnapshot` struct in `src/state.rs`:

- fields include `service_name`, `server_status`, `restarts`, `connected_clients`, `active_clients`, `pending_requests`,
  `cached_initialize`, `initializing`, `last_reset`, `queue_depth`, `child_pid`, `max_request_bytes`, and
  restart/backoff parameters.

#### 3.2.3. Tray icon

When the `tray` feature is enabled and the `--tray` flag is set, the runtime interacts with `src/tray.rs` (via
`find_tray_icon` and `spawn_tray`). Tests in `tray.rs` confirm that:

- a default icon can be loaded without panicking; and
- status/lines rendering works.

If you do not need a tray icon (for example on a server), you can:

- build without tray support using `--no-default-features`, and/or
- avoid setting `--tray` at runtime.

### 3.3. Workflow tips for non‑programmer specialists

1. **Start with test runs before making changes**
    - Run `cargo test` to ensure the baseline is green.
    - After adjusting configuration or making code changes with a developer, run `cargo test` again.

2. **Use log levels for diagnosis**
    - If something behaves strangely, rerun the mux with `--log-level debug` to get more detailed logs.
    - Capture logs from the terminal when reporting bugs.

3. **Use the health check and status file instead of attaching debuggers**
    - The `health` subcommand and JSON status file are designed for operational visibility without low‑level tools.

4. **Prefer configuration changes over code changes**
    - Many behaviors (timeouts, max clients, logging) can be adjusted via config or CLI flags without changing Rust
      code.
    - If you need a behavior not exposed in config (for example, a new host scanning rule in `scan.rs`), coordinate with
      a Rust developer.

5. **When in doubt, capture three things**
    - The exact `rmcp-mux` command you ran.
    - The configuration snippet for the relevant service.
    - The terminal output (logs and, if available, the status JSON file contents).

---

## 4. Summary for new contributors

- Use **Rust + Cargo** to build: `cargo build` for development, `cargo build --release` for production.
- Run **tests regularly** with `cargo test`; tests are fast and already cover much of the core logic.
- When adding code, follow the existing patterns:
    - `anyhow` for errors
    - `tracing` for logging
    - `tokio` for async runtime
    - `serde`/`serde_json`/`serde_yaml`/`toml` for configuration and JSON handling
- Extend tests in the same style as existing ones – place unit tests in `#[cfg(test)] mod tests` within the module, and
  consider integration tests under a top‑level `tests/` directory if needed.
- Prefer configuration tweaks (via `Config` / `ServerConfig` and CLI flags) over invasive code changes when adjusting
  operational behavior.

---

## 5. Files created under `.ai-agents/`

The following file was created as part of this guidance work:

- `.ai-agents/guidelines.md` – this document.

No other new files were added under `.ai-agents/`.
