# Configuration Guide

## Przegląd

rmcp-memex można skonfigurować na trzy sposoby (w kolejności priorytetu):
1. **Flagi CLI** - najwyższy priorytet
2. **Plik konfiguracyjny TOML** - średni priorytet
3. **Wartości domyślne** - najniższy priorytet

## Opcje CLI

```bash
rmcp_memex [OPTIONS] [COMMAND]
```

### Komendy

| Komenda | Opis |
|---------|------|
| `serve` | Uruchom serwer MCP (domyślna) |
| `wizard` | Interaktywny kreator konfiguracji |
| `index` | Batch indexing dokumentów |

### Globalne Opcje

| Flaga | Opis | Domyślnie |
|-------|------|-----------|
| `--config <PATH>` | Ścieżka do pliku konfiguracyjnego TOML | brak |
| `--mode <MODE>` | Tryb serwera: `memory` lub `full` | `full` |
| `--features <LIST>` | Lista funkcji (comma-separated) | `filesystem,memory,search` |
| `--cache-mb <SIZE>` | Rozmiar cache w MB | `4096` |
| `--db-path <PATH>` | Ścieżka do LanceDB | `~/.rmcp_servers/rmcp_memex/lancedb` |
| `--max-request-bytes <SIZE>` | Max rozmiar requestu | `5242880` (5MB) |
| `--log-level <LEVEL>` | Poziom logowania | `info` |
| `--allowed-paths <PATH>` | Dozwolone ścieżki (można powtórzyć) | `$HOME`, `cwd` |
| `--security-enabled` | Włącz namespace security | `false` |
| `--token-store-path <PATH>` | Ścieżka do token store | `~/.rmcp_servers/rmcp_memex/tokens.json` |

### Przykłady CLI

```bash
# Podstawowe uruchomienie
rmcp_memex serve

# Tryb memory-only (bez dostępu do filesystem)
rmcp_memex serve --mode memory

# Z własną konfiguracją
rmcp_memex serve --config ~/.rmcp_servers/config/rmcp-memex.toml

# Z security i custom paths
rmcp_memex serve \
  --security-enabled \
  --allowed-paths ~ \
  --allowed-paths /Volumes/Data \
  --log-level debug

# Batch indexing
rmcp_memex index ./documents --namespace docs --recursive --glob "*.md"
```

## Plik Konfiguracyjny (TOML)

### Lokalizacja

Domyślna lokalizacja: `~/.rmcp_servers/config/rmcp-memex.toml`

### Pełny Przykład

```toml
# Tryb serwera: "memory" lub "full"
mode = "full"

# Lista włączonych funkcji
features = "filesystem,memory,search"

# Rozmiar cache w MB
cache_mb = 4096

# Ścieżka do LanceDB vector store
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"

# Maksymalny rozmiar requestu JSON-RPC (bytes)
max_request_bytes = 5242880

# Poziom logowania: trace, debug, info, warn, error
log_level = "info"

# Whitelist dozwolonych ścieżek dla operacji na plikach
# Jeśli puste, domyślnie $HOME i current working directory
allowed_paths = [
    "~",
    "/Volumes/LibraxisShare/Klaudiusz",
    "/opt/shared/documents"
]

# Włącz namespace token-based access control
security_enabled = true

# Ścieżka do pliku z tokenami namespace'ów
token_store_path = "~/.rmcp_servers/rmcp_memex/tokens.json"
```

### Minimalna Konfiguracja

```toml
# Tylko niezbędne ustawienia
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"
security_enabled = true
```

## Tryby Serwera

### Full Mode (domyślny)

Wszystkie funkcje włączone:
- RAG indexing z plików
- Memory operations
- Semantic search

```bash
rmcp_memex serve --mode full
# lub
rmcp_memex serve  # domyślnie full
```

### Memory Mode

Tylko operacje pamięciowe, bez dostępu do filesystem:
- Memory upsert/get/search/delete
- Semantic search
- Brak rag_index (tylko rag_index_text)

```bash
rmcp_memex serve --mode memory
```

Użyj tego trybu gdy:
- Serwer nie powinien mieć dostępu do plików
- Tylko vector memory jest potrzebne
- Zwiększone bezpieczeństwo

## Zmienne Środowiskowe

| Zmienna | Opis |
|---------|------|
| `HOME` / `USERPROFILE` | Home directory (dla ~ expansion) |
| `LANCEDB_PATH` | Override ścieżki LanceDB |
| `SLED_PATH` | Override ścieżki sled K/V store |
| `FASTEMBED_CACHE_PATH` | Cache dla modeli FastEmbed |
| `HF_HUB_CACHE` | Cache dla modeli HuggingFace |

## Konfiguracja dla Claude/MCP

### ~/.claude.json

```json
{
  "mcpServers": {
    "rmcp-memex": {
      "command": "rmcp_memex",
      "args": ["serve", "--config", "~/.rmcp_servers/config/rmcp-memex.toml"]
    }
  }
}
```

### Z security enabled

```json
{
  "mcpServers": {
    "rmcp-memex": {
      "command": "rmcp_memex",
      "args": [
        "serve",
        "--security-enabled",
        "--allowed-paths", "~",
        "--allowed-paths", "/Volumes/Data"
      ]
    }
  }
}
```

## Batch Indexing

Komenda `index` pozwala na masowe indeksowanie dokumentów.

### Składnia

```bash
rmcp_memex index <PATH> [OPTIONS]
```

### Opcje

| Flaga | Opis |
|-------|------|
| `-n, --namespace <NAME>` | Namespace dla dokumentów (domyślnie: `rag`) |
| `-r, --recursive` | Rekursywnie przeglądaj podkatalogi |
| `-g, --glob <PATTERN>` | Filtruj pliki wzorcem glob |
| `--max-depth <N>` | Maksymalna głębokość (0 = bez limitu) |

### Przykłady

```bash
# Indeksuj pojedynczy plik
rmcp_memex index ./README.md

# Indeksuj folder rekursywnie
rmcp_memex index ./docs --recursive --namespace documentation

# Tylko pliki markdown
rmcp_memex index ./notes --recursive --glob "*.md" --namespace notes

# Z limitem głębokości
rmcp_memex index ./project --recursive --max-depth 3
```

## Wizard (Kreator Konfiguracji)

Interaktywny kreator do generowania konfiguracji.

```bash
rmcp_memex wizard

# Dry run - pokaż zmiany bez zapisywania
rmcp_memex wizard --dry-run
```

Wizard pomoże skonfigurować:
- Ścieżkę do LanceDB
- Dozwolone ścieżki
- Security settings
- Integrację z Claude

## Priorytet Konfiguracji

Gdy ta sama opcja jest ustawiona w wielu miejscach:

```
CLI flag > Config file > Default value
```

Przykład:
```bash
# Config file: log_level = "info"
# CLI: --log-level debug
# Wynik: debug (CLI wygrywa)
rmcp_memex serve --config config.toml --log-level debug
```

## Walidacja Konfiguracji

Serwer waliduje konfigurację przy starcie:

1. **Ścieżki** - sprawdza czy istnieją i są dostępne
2. **Allowed paths** - rozwiązuje ~ i sprawdza uprawnienia
3. **Token store** - tworzy plik jeśli nie istnieje (gdy security enabled)
4. **LanceDB** - inicjalizuje bazę jeśli nie istnieje

Błędy konfiguracji są raportowane przy starcie z jasnym komunikatem.

## Troubleshooting

### "Access denied: path outside allowed directories"

Dodaj ścieżkę do `allowed_paths`:
```toml
allowed_paths = [
    "~",
    "/path/to/your/directory"
]
```

### "Cannot resolve config path"

Sprawdź czy plik konfiguracyjny istnieje:
```bash
ls -la ~/.rmcp_servers/config/rmcp-memex.toml
```

### "Token store not found"

Przy pierwszym uruchomieniu z `--security-enabled`, token store jest tworzony automatycznie. Upewnij się że katalog nadrzędny istnieje:
```bash
mkdir -p ~/.rmcp_servers/rmcp_memex
```

### Logi debugowania

```bash
rmcp_memex serve --log-level trace
```

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
