# RMCP-MUX Refactor Proposal: Single Daemon, Multiple Servers

**Autor**: Klaudiusz
**Data**: 2025-12-24
**Status**: PROPOSAL - Do audytu przez Macieja

---

## Executive Summary

Obecna architektura rmcp-mux wymaga uruchomienia **oddzielnego procesu daemona dla każdego serwera MCP**. To prowadzi do:
- Chaosu w PID-ach
- Zawieszania terminali
- Konfliktów przy zamykaniu procesów
- Złożoności operacyjnej (5 serwerów = 5 procesów do zarządzania)

Proponuję refaktor do modelu **Single Daemon, Multiple Servers** - jeden proces zarządza wszystkimi serwerami z pliku konfiguracyjnego.

---

## Część 1: Diagnoza Obecnego Stanu

### 1.1 Jak działa teraz?

Obecny flow uruchomienia (z `mux-launcher.sh`):

```bash
# Dla każdego serwera w konfiguracji:
rmcp-mux --config ~/.rmcp_servers/config/mux.toml --service loctree &
rmcp-mux --config ~/.rmcp_servers/config/mux.toml --service rmcp-memex &
rmcp-mux --config ~/.rmcp_servers/config/mux.toml --service brave-search &
rmcp-mux --config ~/.rmcp_servers/config/mux.toml --service sequential-thinking &
rmcp-mux --config ~/.rmcp_servers/config/mux.toml --service youtube-transcript &
```

Każde wywołanie `rmcp-mux --service X`:
1. Parsuje cały `mux.toml`
2. Wyciąga TYLKO sekcję `[servers.X]`
3. Uruchamia JEDEN daemon z JEDNYM child process (serwer MCP)
4. Nasłuchuje na JEDNYM Unix socket

### 1.2 Architektura PRZED (Obecna)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        OBECNY STAN - 5 PROCESÓW                             │
└─────────────────────────────────────────────────────────────────────────────┘

mux-launcher.sh
      │
      ├──► rmcp-mux --service loctree              [PID 1701]
      │         │
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/loctree.sock
      │         │
      │         └── Child Process: loctree-mcp    [PID 1702]
      │
      ├──► rmcp-mux --service rmcp-memex           [PID 1704]
      │         │
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/rmcp-memex.sock
      │         │
      │         └── Child Process: rmcp_memex     [PID 1705]
      │
      ├──► rmcp-mux --service brave-search         [PID 1708]
      │         │
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/brave-search.sock
      │         │
      │         └── Child Process: npx @anthropic/... [PID 1709]
      │
      ├──► rmcp-mux --service sequential-thinking  [PID 1710]
      │         │
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/sequential-thinking.sock
      │         │
      │         └── Child Process: npx @anthropic/... [PID 1711]
      │
      └──► rmcp-mux --service youtube-transcript   [PID 1712]
                │
                ├── Unix Listener: ~/.rmcp_servers/sockets/youtube-transcript.sock
                │
                └── Child Process: npx @kazuph/... [PID 1713]


PROBLEMY:
─────────
1. 5 procesów rmcp-mux do zarządzania
2. 5 zestawów PID-ów do śledzenia
3. mux-launcher.sh musi iterować i tworzyć 5 procesów w tle
4. Każdy proces ma własny lifecycle (restart, shutdown)
5. Brak centralnego punktu kontroli
6. Problemy z synchronizacją shutdown (który proces zabić pierwszy?)
7. Każdy daemon ładuje całą konfigurację, używa 1/5
```

### 1.3 Problemy Operacyjne

#### Problem A: Chaos PID-ów

```bash
# Próba sprawdzenia statusu:
ps aux | grep rmcp-mux
# Wynik: 5+ linii, która jest która?

# Próba zabicia jednego serwera:
pkill -f "rmcp-mux.*loctree"
# Może zabić za dużo lub za mało
```

#### Problem B: Zawieszanie Terminali

Gdy `mux-launcher.sh` uruchamia procesy w tle:
1. Parent shell tworzy subshells
2. Każdy subshell uruchamia daemon
3. Stdin/stdout nie są prawidłowo odłączone
4. Terminal czeka na zakończenie wszystkich child processes
5. **HANG**

#### Problem C: Konflikt przy Shutdown

```bash
# mux-launcher.sh stop:
for pid in $(cat ~/.rmcp_servers/pids/*.pid); do
    kill $pid  # Który proces? Kolejność?
done
# Race conditions, orphaned children, zombie processes
```

#### Problem D: Brak Atomowego Restartu

Chcesz zrestartować wszystko?
```bash
mux-launcher.sh restart
# = stop wszystkich (może się nie udać)
# + start wszystkich (może się częściowo udać)
# = niespójny stan
```

---

## Część 2: Proponowane Rozwiązanie

### 2.1 Architektura PO (Docelowa)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        DOCELOWY STAN - 1 PROCES                             │
└─────────────────────────────────────────────────────────────────────────────┘

rmcp-mux --config ~/.rmcp_servers/config/mux.toml
      │
      │  [JEDEN PROCES - PID 1701]
      │
      ├── Tokio Runtime (multi-threaded)
      │
      ├── Server Manager: loctree
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/loctree.sock
      │         ├── Child Process: loctree-mcp
      │         ├── Client Pool (max 5)
      │         └── Heartbeat Monitor
      │
      ├── Server Manager: rmcp-memex
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/rmcp-memex.sock
      │         ├── Child Process: rmcp_memex
      │         ├── Client Pool (max 5)
      │         └── Heartbeat Monitor
      │
      ├── Server Manager: brave-search
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/brave-search.sock
      │         ├── Child Process: npx @anthropic/...
      │         ├── Client Pool (max 5)
      │         └── Heartbeat Monitor
      │
      ├── Server Manager: sequential-thinking
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/sequential-thinking.sock
      │         ├── Child Process: npx @anthropic/...
      │         ├── Client Pool (max 5)
      │         └── Heartbeat Monitor
      │
      ├── Server Manager: youtube-transcript
      │         ├── Unix Listener: ~/.rmcp_servers/sockets/youtube-transcript.sock
      │         ├── Child Process: npx @kazuph/...
      │         ├── Client Pool (max 5)
      │         └── Heartbeat Monitor
      │
      └── Central Controller
                ├── Graceful Shutdown Handler (Ctrl+C)
                ├── Status Aggregator (all servers)
                ├── Health Dashboard
                └── Single PID file: ~/.rmcp_servers/pids/mux.pid


KORZYŚCI:
─────────
1. JEDEN proces do zarządzania
2. JEDEN PID do zapamiętania
3. Atomowy start/stop/restart
4. Centralna kontrola wszystkich serwerów
5. Współdzielony Tokio runtime (efektywność)
6. Graceful shutdown - wszystkie children zamykane w kolejności
7. Unified logging i status
```

### 2.2 Jak to będzie działać?

#### Uruchomienie:

```bash
# Zamiast 5 komend:
rmcp-mux --config ~/.rmcp_servers/config/mux.toml

# Opcjonalnie z filtrem (uruchom tylko wybrane):
rmcp-mux --config mux.toml --only loctree,rmcp-memex

# Lub wykluczenie:
rmcp-mux --config mux.toml --except youtube-transcript
```

#### Shutdown:

```bash
# Jeden sygnał - wszystko się zamyka poprawnie:
kill $(cat ~/.rmcp_servers/pids/mux.pid)

# Lub Ctrl+C w terminalu foreground
```

#### Status:

```bash
# Jeden endpoint dla wszystkich serwerów:
rmcp-mux --status --config mux.toml

# Wynik:
# ┌─────────────────────┬────────┬─────────┬───────────┐
# │ Server              │ Status │ Clients │ Uptime    │
# ├─────────────────────┼────────┼─────────┼───────────┤
# │ loctree             │ OK     │ 2/5     │ 4h 23m    │
# │ rmcp-memex          │ OK     │ 1/5     │ 4h 23m    │
# │ brave-search        │ IDLE   │ 0/5     │ 4h 23m    │
# │ sequential-thinking │ OK     │ 1/5     │ 4h 23m    │
# │ youtube-transcript  │ LAZY   │ 0/5     │ not started│
# └─────────────────────┴────────┴─────────┴───────────┘
```

---

## Część 3: Zmiany w Kodzie

### 3.1 Obecna struktura kodu

```
rmcp-mux/src/
├── lib.rs              # MuxConfig, run_mux_server(), spawn_mux_server()
├── config.rs           # Config parsing, ResolvedParams
├── state.rs            # MuxState (per-server state)
├── runtime/
│   ├── mod.rs          # run_mux(), run_mux_internal()
│   ├── client.rs       # handle_client(), ID rewriting
│   ├── server.rs       # server_manager(), child process
│   └── proxy.rs        # STDIO <-> Socket bridge
└── bin/
    └── rmcp-mux.rs     # CLI: --service X (jeden serwer)
```

### 3.2 Co trzeba zmienić?

#### A) Nowy entry point w `lib.rs`:

```rust
// NOWA FUNKCJA - uruchamia WSZYSTKIE serwery z konfigu
pub async fn run_mux_multi(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;

    let shutdown = CancellationToken::new();
    let mut handles: Vec<MuxHandle> = Vec::new();

    // Spawn każdego serwera jako osobny task w TYM SAMYM procesie
    for (name, server_config) in &config.servers {
        let mux_config = server_config.to_mux_config(name);
        let handle = spawn_mux_server(mux_config).await?;
        handles.push(handle);
    }

    // Czekaj na shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Shutdown signal received");
        }
        _ = shutdown.cancelled() => {}
    }

    // Graceful shutdown wszystkich serwerów
    for handle in handles {
        handle.shutdown();
    }
    for handle in handles {
        handle.wait().await?;
    }

    Ok(())
}
```

#### B) Zmiana CLI w `bin/rmcp-mux.rs`:

```rust
// PRZED:
#[derive(Parser)]
struct Cli {
    #[arg(long)]
    config: PathBuf,

    #[arg(long)]
    service: String,  // WYMAGANE - jeden serwer
}

// PO:
#[derive(Parser)]
struct Cli {
    #[arg(long)]
    config: PathBuf,

    #[arg(long)]
    service: Option<String>,  // OPCJONALNE - dla backward compat

    #[arg(long)]
    only: Option<String>,  // "loctree,rmcp-memex" - filtr

    #[arg(long)]
    except: Option<String>,  // wykluczenia
}

// Logika:
// - Jeśli --service podane: stary tryb (jeden serwer)
// - Jeśli nie: nowy tryb (wszystkie serwery)
```

#### C) Struktura State dla multi-server:

```rust
// NOWY: Centralny state dla wszystkich serwerów
pub struct MultiMuxState {
    pub servers: HashMap<String, Arc<Mutex<MuxState>>>,
    pub shutdown: CancellationToken,
    pub start_time: Instant,
}

impl MultiMuxState {
    pub fn status(&self) -> MultiStatusSnapshot {
        // Agreguj status ze wszystkich serwerów
    }

    pub fn shutdown_all(&self) {
        self.shutdown.cancel();
    }
}
```

#### D) Nowy launcher (zastępuje mux-launcher.sh):

```bash
#!/bin/bash
# ~/.rmcp_servers/bin/mux-launcher.sh - NOWA WERSJA

CONFIG="$HOME/.rmcp_servers/config/mux.toml"
PID_FILE="$HOME/.rmcp_servers/pids/mux.pid"
LOG_FILE="$HOME/.rmcp_servers/logs/mux.log"

case "$1" in
    start)
        if [ -f "$PID_FILE" ] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
            echo "Already running (PID $(cat $PID_FILE))"
            exit 1
        fi

        # JEDEN proces - wszystkie serwery
        nohup rmcp-mux --config "$CONFIG" > "$LOG_FILE" 2>&1 &
        echo $! > "$PID_FILE"
        echo "Started (PID $!)"
        ;;

    stop)
        if [ -f "$PID_FILE" ]; then
            kill $(cat "$PID_FILE") 2>/dev/null
            rm -f "$PID_FILE"
            echo "Stopped"
        fi
        ;;

    restart)
        $0 stop
        sleep 1
        $0 start
        ;;

    status)
        rmcp-mux --status --config "$CONFIG"
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|status}"
        ;;
esac
```

---

## Część 4: Porównanie PRZED vs PO

### 4.1 Tabela Porównawcza

| Aspekt | PRZED (5 procesów) | PO (1 proces) |
|--------|-------------------|---------------|
| **Procesy systemowe** | 5 daemonów + 5 children = 10 | 1 daemon + 5 children = 6 |
| **PID management** | 5 plików .pid | 1 plik mux.pid |
| **Memory footprint** | 5× runtime overhead | 1× runtime (współdzielony) |
| **Startup time** | Sekwencyjny (5× spawn) | Równoległy (1× spawn, 5× task) |
| **Shutdown** | Chaotyczny (race conditions) | Atomowy (graceful) |
| **Logging** | 5 oddzielnych logów | 1 unified log |
| **Status check** | 5× health check | 1× aggregated status |
| **Restart pojedynczego** | Skomplikowane | `--restart-service X` |
| **Terminal hang** | TAK (subshell hell) | NIE (jeden proces) |
| **Ctrl+C behavior** | Nieprzewidywalne | Clean shutdown |

### 4.2 Diagram Przepływu - PRZED

```
User: "mux-launcher.sh start"
           │
           ▼
    ┌──────────────────┐
    │  mux-launcher.sh │
    │  (bash script)   │
    └────────┬─────────┘
             │
    ┌────────┴────────┐
    │ for server in   │
    │ loctree memex...│
    └────────┬────────┘
             │
    ┌────────┼────────┬────────┬────────┬────────┐
    ▼        ▼        ▼        ▼        ▼        ▼
  rmcp-mux rmcp-mux rmcp-mux rmcp-mux rmcp-mux
  --service --service --service --service --service
  loctree   memex    brave    seq-think youtube
    │        │        │        │        │
    ▼        ▼        ▼        ▼        ▼
  [daemon] [daemon] [daemon] [daemon] [daemon]
    │        │        │        │        │
    ▼        ▼        ▼        ▼        ▼
  [child]  [child]  [child]  [child]  [child]


═══════════════════════════════════════════════════
PROBLEM: 5 procesów w tle, każdy z własnym lifecycle
         Shell czeka na wszystkie → HANG
═══════════════════════════════════════════════════
```

### 4.3 Diagram Przepływu - PO

```
User: "mux-launcher.sh start"
           │
           ▼
    ┌──────────────────┐
    │  mux-launcher.sh │
    │  (bash script)   │
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────────────────────────┐
    │  rmcp-mux --config mux.toml          │
    │  (JEDEN proces)                      │
    └────────┬─────────────────────────────┘
             │
             ▼
    ┌──────────────────────────────────────┐
    │           Tokio Runtime              │
    │  ┌─────────────────────────────────┐ │
    │  │     Centralized Controller      │ │
    │  │  - Shutdown handler             │ │
    │  │  - Status aggregator            │ │
    │  │  - PID file writer              │ │
    │  └─────────────┬───────────────────┘ │
    │                │                     │
    │    ┌───────────┼───────────┐         │
    │    ▼           ▼           ▼         │
    │ [Task 1]   [Task 2]   [Task N]       │
    │ loctree    memex      youtube        │
    │    │           │           │         │
    │    ▼           ▼           ▼         │
    │ [child]    [child]    [child]        │
    └──────────────────────────────────────┘


═══════════════════════════════════════════════════
ROZWIĄZANIE: 1 proces, N tasków współdzielących runtime
             Ctrl+C → graceful shutdown wszystkiego
═══════════════════════════════════════════════════
```

---

## Część 5: Plan Implementacji

### Faza 1: Przygotowanie (bez breaking changes)

1. Dodaj `run_mux_multi()` do `lib.rs`
2. Dodaj `MultiMuxState` do `state.rs`
3. Dodaj testy dla multi-server mode
4. **Nie zmieniaj jeszcze CLI** - stary tryb dalej działa

### Faza 2: CLI Extension

1. Dodaj `--only` i `--except` flags
2. Zmień `--service` na opcjonalne
3. Domyślne zachowanie (bez `--service`) = wszystkie serwery
4. Backward compatibility: `--service X` dalej działa jak przed

### Faza 3: Nowy Launcher

1. Przepisz `mux-launcher.sh` na prostszą wersję
2. Jeden PID file zamiast wielu
3. Unified logging
4. Status command z tabelką

### Faza 4: Cleanup

1. Usuń stare PID files z `~/.rmcp_servers/pids/`
2. Zaktualizuj dokumentację
3. Dodaj migration guide dla użytkowników

---

## Część 6: Ryzyka i Mitygacje

### Ryzyko 1: Jeden crash = wszystko pada

**Mitygacja**: Tokio tasks są izolowane. Crash jednego child process nie zabija innych. `spawn_mux_server()` już to obsługuje - restart z exponential backoff.

### Ryzyko 2: Memory sharing conflicts

**Mitygacja**: Każdy serwer ma własny `MuxState` za `Arc<Mutex<>>`. Brak współdzielonych mutable state między serwerami.

### Ryzyko 3: Backward compatibility

**Mitygacja**: `--service X` dalej działa. Nowy tryb to opt-in przez brak `--service` flag.

### Ryzyko 4: Debugging trudniejszy

**Mitygacja**: Structured logging z `[server_name]` prefix. Status command pokazuje per-server metrics.

---

## Część 7: Pytania do Macieja

1. **Czy chcesz zachować backward compatibility z `--service X`?**
   - TAK = stary tryb jako fallback
   - NIE = uproszczenie kodu, tylko nowy tryb

2. **Czy `mux-launcher.sh` ma zostać czy zastąpić go bezpośrednim wywołaniem?**
   - Launcher = wygoda (start/stop/restart)
   - Bezpośrednio = mniej moving parts

3. **Czy status ma być jako CLI subcommand czy osobny binary?**
   - `rmcp-mux --status` = jeden binary
   - `rmcp-mux-status` = separation of concerns

4. **Czy robić to inkrementalnie (3 PR-y) czy jednym dużym refaktorem?**
   - Inkrementalnie = bezpieczniej, łatwiejszy review
   - Jednym = szybciej, mniej merge conflicts

---

## Podsumowanie

**CEL**: Jeden proces `rmcp-mux` zarządzający wszystkimi serwerami MCP z `mux.toml`.

**KORZYŚCI**:
- Zero chaosu w PID-ach
- Zero zawieszania terminali
- Atomowy start/stop/restart
- Centralna kontrola i monitoring
- Mniejszy footprint (współdzielony runtime)

**KOSZT**:
- ~200-300 linii nowego kodu
- ~2-4h implementacji
- Testy i dokumentacja

**RYZYKO**: Niskie - architektura `spawn_mux_server()` już istnieje w `lib.rs`, tylko trzeba ją użyć dla wszystkich serwerów zamiast jednego.
k
---

Created by M&K (c)2025 The LibraxisAI Team
