# Security - Namespace Access Control

## Problem

W środowisku multi-agent, gdzie wiele AI agentów może korzystać z tego samego serwera rmcp-memex, potrzebna jest izolacja danych między agentami. Bez mechanizmu kontroli dostępu:
- Agent A może odczytać dane Agenta B
- Brak możliwości ochrony wrażliwych namespace'ów
- Trudność w audycie dostępu do danych

Dodatkowo, ograniczenie dostępu do plików tylko do `$HOME` i `cwd` było zbyt restrykcyjne - blokowało legitymowe użycie zewnętrznych wolumenów (np. `/Volumes/ExternalDrive`).

## Rozwiązanie

Zaimplementowano dwupoziomowy system bezpieczeństwa:

### 1. Konfigurowalna Whitelist Ścieżek

Zamiast hardcoded ograniczenia do `$HOME` i `cwd`, wprowadzono konfigurowalną listę dozwolonych ścieżek.

```toml
# ~/.rmcp_servers/config/rmcp-memex.toml
allowed_paths = [
    "~",                              # Home directory
    "/Volumes/LibraxisShare/data",    # External volume
    "/opt/shared/documents"           # Shared directory
]
```

**Zachowanie:**
- Jeśli `allowed_paths` jest puste → domyślnie `$HOME` + `cwd` (backward compatible)
- Jeśli `allowed_paths` jest ustawione → tylko te ścieżki są dozwolone
- Wspiera `~` expansion do home directory
- Walidacja przez canonicalization (rozwiązuje symlinki)

### 2. Namespace Access Tokens

Token-based access control dla namespace'ów. Chronione namespace'y wymagają tokena do odczytu/zapisu.

```
┌─────────────────────────────────────────────────────────────┐
│                    Namespace Security                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Public Namespace          Protected Namespace               │
│  ┌─────────────┐          ┌─────────────────────┐           │
│  │  "default"  │          │    "pamietnik"      │           │
│  │             │          │                     │           │
│  │  No token   │          │  Token: rmx_7f3a9b  │           │
│  │  required   │          │  required           │           │
│  └─────────────┘          └─────────────────────┘           │
│                                                              │
│  Agent A: ✓ access        Agent A: ✗ no token               │
│  Agent B: ✓ access        Agent B: ✓ has token              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Użycie

### Włączenie Security

```bash
# CLI
rmcp_memex serve --security-enabled

# Lub w config.toml
security_enabled = true
token_store_path = "~/.rmcp_servers/rmcp_memex/tokens.json"
```

### Tworzenie Tokena dla Namespace

```json
// MCP Request
{
  "method": "tools/call",
  "params": {
    "name": "namespace_create_token",
    "arguments": {
      "namespace": "pamietnik",
      "description": "Personal diary namespace"
    }
  }
}

// Response
{
  "content": [{
    "type": "text",
    "text": "Token created for namespace 'pamietnik': rmx_a1b2c3d4e5f6..."
  }]
}
```

**WAŻNE:** Token jest zwracany tylko raz przy tworzeniu. Zapisz go w bezpiecznym miejscu!

### Dostęp do Chronionego Namespace

```json
// Bez tokena - BŁĄD
{
  "method": "tools/call",
  "params": {
    "name": "memory_search",
    "arguments": {
      "namespace": "pamietnik",
      "query": "moje wspomnienia"
    }
  }
}
// Error: "Access denied: namespace 'pamietnik' requires a valid token"

// Z tokenem - OK
{
  "method": "tools/call",
  "params": {
    "name": "memory_search",
    "arguments": {
      "namespace": "pamietnik",
      "query": "moje wspomnienia",
      "token": "rmx_a1b2c3d4e5f6..."
    }
  }
}
// Success: returns search results
```

### Odwołanie Tokena

```json
{
  "method": "tools/call",
  "params": {
    "name": "namespace_revoke_token",
    "arguments": {
      "namespace": "pamietnik"
    }
  }
}
```

Po odwołaniu tokena, namespace staje się ponownie publiczny.

### Lista Chronionych Namespace'ów

```json
{
  "method": "tools/call",
  "params": {
    "name": "namespace_list_protected",
    "arguments": {}
  }
}

// Response
{
  "content": [{
    "type": "text",
    "text": "[\"pamietnik\", \"projekty\", \"finanse\"]"
  }]
}
```

### Status Security

```json
{
  "method": "tools/call",
  "params": {
    "name": "namespace_security_status",
    "arguments": {}
  }
}

// Response (enabled)
{
  "content": [{
    "type": "text",
    "text": "{\"enabled\":true,\"token_store_path\":\"~/.rmcp_servers/rmcp_memex/tokens.json\"}"
  }]
}

// Response (disabled)
{
  "content": [{
    "type": "text",
    "text": "{\"enabled\":false,\"message\":\"Namespace security is disabled. All namespaces are publicly accessible.\"}"
  }]
}
```

## Format Tokena

Tokeny mają format: `rmx_<32 znaki hex>`

```
rmx_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
│   └─────────────────────────────────┘
│              32 znaki hex (128 bit)
└── prefix "rmx_" (rmcp-memex)
```

Tokeny są generowane kryptograficznie bezpiecznie (`rand::thread_rng()`).

## Token Store

Tokeny są przechowywane w pliku JSON:

```json
// ~/.rmcp_servers/rmcp_memex/tokens.json
{
  "pamietnik": {
    "token_hash": "5e884898da28047d...",
    "created_at": "2024-12-22T10:30:00Z",
    "description": "Personal diary namespace"
  },
  "projekty": {
    "token_hash": "d033e22ae348aeb5...",
    "created_at": "2024-12-22T11:00:00Z",
    "description": null
  }
}
```

**Bezpieczeństwo:**
- Przechowywany jest tylko **hash** tokena (SHA-256), nie sam token
- Token plaintext jest zwracany tylko raz przy tworzeniu
- Weryfikacja przez porównanie hashy

## Path Validation

Funkcja `validate_path()` chroni przed path traversal attacks:

```rust
// Blokowane wzorce
"../../../etc/asdpasswd"     // Path traversal
sd"/etc/passwd"             // Outside allowed paths
"~/../../root/.ssh"       // Traversal after expansion

// Dozwolone (jeśli w allowed_paths)
"~/Documents/notes.md"    // Under home
"/Volumes/Data/file.txt"  // Configured external volume
```

**Walidacja:**
1. Sprawdzenie czy ścieżka nie jest pusta
2. Expansion `~` do home directory
3. Sprawdzenie wzorca `..` (path traversal)
4. Canonicalization (rozwiązanie symlinków)
5. Sprawdzenie czy canonical path jest pod dozwoloną ścieżką

## Implementacja

### Pliki źródłowe

| Plik | Opis |
|------|------|
| `rmcp-memex/src/security/mod.rs` | `NamespaceAccessManager`, `TokenStore`, token generation/verification |
| `rmcp-memex/src/handlers/mod.rs` | `validate_path()`, integration z access manager |
| `rmcp-memex/src/lib.rs` | `NamespaceSecurityConfig`, re-exports |
| `rmcp-memex/src/bin/rmcp_memex.rs` | CLI flags `--security-enabled`, `--token-store-path` |

### Kluczowe struktury

```rust
/// Konfiguracja security
pub struct NamespaceSecurityConfig {
    pub enabled: bool,
    pub token_store_path: Option<String>,
}

/// Manager dostępu do namespace'ów
pub struct NamespaceAccessManager {
    enabled: bool,
    store: Option<Arc<Mutex<TokenStore>>>,
}

/// Przechowywanie tokenów
pub struct TokenStore {
    path: PathBuf,
    tokens: HashMap<String, TokenEntry>,
}

/// Wpis tokena
pub struct TokenEntry {
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}
```

### Testy

```bash
cd rmcp-memex && cargo test security
```

Testy pokrywają:
- `test_token_generation` - generowanie tokenów w poprawnym formacie
- `test_access_manager_disabled` - zachowanie gdy security wyłączone
- `test_token_store_create_and_verify` - tworzenie i weryfikacja tokenów
- `test_access_manager_enabled` - pełny flow z włączonym security

## Best Practices

### Dla administratorów

1. **Włącz security w produkcji** - `--security-enabled`
2. **Ogranicz allowed_paths** - tylko niezbędne ścieżki
3. **Backup token store** - tokeny są nieodwracalne
4. **Rotacja tokenów** - okresowo revoke + create nowe

### Dla agentów AI

1. **Przechowuj tokeny bezpiecznie** - env vars lub secure storage
2. **Nie loguj tokenów** - unikaj wyświetlania w logach
3. **Używaj dedykowanych namespace'ów** - izolacja danych
4. **Sprawdzaj security_status** - upewnij się że security jest włączone

## Przyszłe rozszerzenia

### Faza 3: Szyfrowanie Namespace'ów (planowane)

```toml
# Przyszła konfiguracja
[namespaces.pamietnik]
encrypted = true
key_derivation = "argon2id"
```

- Dane w LanceDB szyfrowane kluczem namespace'u
- Bez klucza = dane bezużyteczne
- Dla naprawdę wrażliwych danych

---

Created by M&K (c)2025 The LibraxisAI Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
