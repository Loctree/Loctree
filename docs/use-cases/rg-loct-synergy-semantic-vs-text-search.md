# Use Case: rg + loct Synergy

**Problem:** `rg "handler"` zwraca 7 matches, ale handler jest nieużywany. Dlaczego?

**Projekt:** Vista (Tauri: TS/React + Rust)
**Data:** 2025-12-02

## Konkretny przykład

```bash
$ rg "quick_search" --type ts --type rust -c
src-tauri/src/lib.rs:1
src-tauri/src/commands/system_menu.rs:2
src-tauri/src/app/tray.rs:5
src/app-shell/MainApplication.tsx:2
src/utils/tauriWrapper.ts:1
# Total: 11 matches!
```

Wygląda na używany. Ale sprawdźmy CO to za matche:

```bash
$ rg "quick_search" src/utils/tauriWrapper.ts
  'quick_search',   # ← type definition w KNOWN_COMMANDS[]

$ rg "quick_search" src/app-shell/MainApplication.tsx
  secureLogger.info?.('system_menu.quick_search');  # ← log string
```

**Zero `invoke('quick_search')` calls.**

## rg vs loct

```bash
# rg: GDZIE występuje string
$ rg "quick_search"
→ 11 matches (type defs, logs, menu IDs, Rust code)

# loct: CZY jest UŻYWANY w produkcji
$ loct commands | grep quick_search
→ Unused handlers (LOW confidence): quick_search (7 string literal matches)
```

## Pattern: co NIE jest invoke()

| Match type | Przykład | Czy używa handler? |
|------------|----------|-------------------|
| Type def | `'handler' as const` | ❌ |
| Log string | `logger.info('handler')` | ❌ |
| Menu ID | `MenuItem::with_id("handler")` | ❌ |
| Test | `expect(invoke('handler'))` | ❌ |
| **invoke()** | `invoke('handler', payload)` | ✅ |

## Workflow

```bash
loct commands                    # lista unused
rg "handler" --type ts           # sprawdź matche
# Jeśli wszystkie to type defs/logs/tests → safe to delete
```

---

*M&K (c)2025*
