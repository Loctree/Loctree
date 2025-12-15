# Loctree v0.6.19+ Session Notes — 2024-12-14

**Branch:** `fix/improve-detection-accuracy`
**Commits:** 10+ (this session continued work from previous sessions)
**Author:** Claude + Monika

---

## TL;DR

Dzisiejsza sesja dodała 10 nowych funkcjonalności:

| # | Feature | Komenda/Flaga | Status |
|---|---------|---------------|--------|
| 1 | Directory slice | `loct focus <dir>` | NEW |
| 2 | Rescan before slice | `loct slice --rescan` | NEW |
| 3 | Unified tag search | `loct tagmap <keyword>` | NEW |
| 4 | Import frequency analysis | `loct hotspots` | NEW |
| 5 | Python package roots | `--py-root <path>` | NEW |
| 6 | Test file filtering | `loct twins --include-tests` | NEW |
| 7 | Crowd context types | `context_type` field | ENHANCED |
| 8 | CSS layout analysis | `loct layoutmap` | NEW |
| 9 | Extended help | `--help-full` | EXISTS |
| 10 | Shell completions | `completions/` | NEW |

---

## 1. `loct focus <dir>` — Directory-level Slice

### Problem
`loct slice` działa na pojedynczych plikach. Gdy agent chce zrozumieć cały feature/moduł (np. `src/features/patients/`), musiał ręcznie slice'ować każdy plik.

### Rozwiązanie
```bash
loct focus src/features/patients/
loct focus src/features/patients/ --consumers
loct focus src/features/patients/ --json
```

### Output
```
Focus: src/features/patients/

Core (12 files, 2,340 LOC):
  src/features/patients/index.ts
  src/features/patients/PatientsList.tsx
  src/features/patients/PatientsHub.tsx
  ... (9 more)

Internal edges: 18 imports within directory

External Deps (8 files, 890 LOC):
  [d1] src/components/Button.tsx
  [d1] src/hooks/usePatient.ts
  ...

Consumers (3 files, 450 LOC):
  src/App.tsx
  src/routes/index.tsx
  ...

Total: 23 files, 3,680 LOC (18 internal + 11 external edges)
```

### Pliki
- `loctree_rs/src/focuser.rs` (NEW)
- `loctree_rs/src/cli/command.rs` — `FocusOptions`
- `loctree_rs/src/cli/parser.rs` — `parse_focus_command()`
- `loctree_rs/src/cli/dispatch/handlers/analysis.rs` — `handle_focus_command()`

### Use cases
- Agent chce zrozumieć feature przed refaktorem
- Analiza coupling między modułami
- Znajdowanie "samotnych wysp" w kodzie

---

## 2. `loct slice --rescan` — Uncommitted Files Support

### Problem
`loct slice` używa cached snapshot. Nowo utworzone pliki (jeszcze nie w git) nie są widoczne.

### Rozwiązanie
```bash
loct slice src/new-feature.ts --rescan
```

Flaga `--rescan` wymusza pełny rescan przed slice'em.

### Pliki
- `loctree_rs/src/cli/command.rs` — `SliceOptions.rescan`
- `loctree_rs/src/cli/parser.rs`
- `loctree_rs/src/cli/dispatch/handlers/slice.rs`

### Use cases
- Agent właśnie utworzył nowy plik i chce go slice'ować
- Praca na WIP branch z wieloma niezcommitowanymi zmianami

---

## 3. `loct tagmap <keyword>` — Unified Tag Search

### Problem
Agent szuka wszystkiego związanego z "message" — musi odpalać osobno `find`, `crowd`, `dead`, `slice`.

### Rozwiązanie
```bash
loct tagmap message
loct tagmap message --json
```

### Output
Kombinuje wyniki z:
- **Find**: pliki/symbole pasujące do keyword
- **Crowd**: funkcjonalne duplikaty wokół keyword
- **Dead**: martwy kod związany z keyword
- **Slice suggestions**: pliki do głębszej analizy

### Pliki
- `loctree_rs/src/cli/command.rs` — `TagmapOptions`
- `loctree_rs/src/cli/parser.rs` — `parse_tagmap_command()`
- `loctree_rs/src/cli/dispatch/handlers/analysis.rs` — `handle_tagmap_command()`

### Use cases
- "Pokaż mi wszystko o messages w tym projekcie"
- Szybki overview przed refaktorem konkretnej domeny

---

## 4. `loct hotspots` — Import Frequency Heatmap

### Problem
Które pliki są "core" (importowane przez wszystko) vs "peripheral" (liście)?

### Rozwiązanie
```bash
loct hotspots                    # top 50 most-imported
loct hotspots --leaves           # entry points / dead code candidates
loct hotspots --coupling         # show in-degree AND out-degree
loct hotspots --min 5            # only files with 5+ importers
loct hotspots --json
```

### Output
```
Import Hotspots (1,234 files analyzed)

CORE (10+ importers):
  [ 45] src/utils/api.ts
  [ 32] src/components/Button.tsx
  [ 28] src/hooks/useAuth.ts

SHARED (3-9 importers):
  [  7] src/types/common.ts
  [  5] src/utils/format.ts

PERIPHERAL (1-2 importers):
  [  2] src/features/settings/SettingsPage.tsx

LEAF (0 importers):
  src/pages/admin/Debug.tsx
  src/utils/deprecated.ts
```

### Kategorie
| Kategoria | In-degree | Interpretacja |
|-----------|-----------|---------------|
| CORE | 10+ | Fundamentalne pliki, ostrożnie z refaktorem |
| SHARED | 3-9 | Współdzielone utilities |
| PERIPHERAL | 1-2 | Feature-specific |
| LEAF | 0 | Entry points lub martwy kod |

### Pliki
- `loctree_rs/src/cli/command.rs` — `HotspotsOptions`
- `loctree_rs/src/cli/parser.rs` — `parse_hotspots_command()`
- `loctree_rs/src/cli/dispatch/handlers/analysis.rs` — `handle_hotspots_command()`

### Use cases
- Identyfikacja core files przed dużym refaktorem
- Znajdowanie kandydatów na usunięcie (LEAF z 0 importers)
- Analiza coupling w projekcie

---

## 5. `--py-root <path>` — Python Package Roots

### Problem
Python projekty z niestandardową strukturą (monorepo, `packages/mylib/`) mają problemy z rozwiązywaniem importów.

### Rozwiązanie
```bash
loct --py-root packages/mylib dead
loct --py-root Lib --py-root src scan
```

Flaga globalna, można użyć wielokrotnie.

### Pliki
- `loctree_rs/src/cli/command.rs` — `GlobalOptions.py_roots`
- `loctree_rs/src/cli/parser.rs` — obsługa `--py-root`

### Use cases
- CPython stdlib (`--py-root Lib`)
- Monorepo z wieloma Python packages
- Niestandardowe struktury katalogów

---

## 6. `loct twins --include-tests` — Test File Filtering

### Problem
`loct twins` pokazuje "duplikaty" które są w rzeczywistości testami (np. `Message` w `types.ts` i `Message.test.ts`).

### Rozwiązanie
```bash
loct twins                    # domyślnie EXCLUDE test files
loct twins --include-tests    # include test files
```

### Heurystyki wykrywania testów
- Ścieżka zawiera: `test`, `tests`, `__tests__`, `spec`, `__mocks__`
- Nazwa pliku: `*.test.*`, `*.spec.*`, `*_test.*`, `*_spec.*`
- Katalogi: `fixtures`, `__fixtures__`

### Pliki
- `loctree_rs/src/cli/command.rs` — `TwinsOptions.include_tests`
- `loctree_rs/src/analyzer/twins.rs` — `is_test_file()`, zmodyfikowane `build_symbol_registry()`, `find_dead_parrots()`, `detect_exact_twins()`

### Use cases
- Czystsze wyniki twins bez false positives z testów
- Opcjonalnie włączanie testów gdy szukamy duplikacji w samych testach

---

## 7. Crowd `context_type` — UI Context Heuristics

### Problem
`loct crowd` znajduje grupy podobnych plików, ale agent nie wie czy to komponenty UI, hooks, czy utilities.

### Rozwiązanie
Automatyczna detekcja typu kontekstu:

```json
{
  "pattern": "message",
  "context_type": "state",
  "members": [...],
  "score": 6.5
}
```

### Typy kontekstu
| Type | Opis | Heurystyki |
|------|------|------------|
| `rail` | Navigation (sidebars, drawers) | nav, sidebar, drawer, menu, rail |
| `panel` | Content panels | panel, card, section, content |
| `modal` | Overlays | modal, dialog, popup, toast, overlay |
| `form` | User input | form, input, field, select, picker |
| `list` | Data display | list, table, grid, data |
| `state` | State management | hook, store, context, state, provider |
| `api` | Data fetching | api, service, client, fetch |
| `util` | Utilities | util, helper, lib |
| `other` | Unclassified | default |

### Pliki
- `loctree_rs/src/analyzer/crowd/types.rs` — `ContextType` enum
- `loctree_rs/src/analyzer/crowd/mod.rs` — `infer_context_type()`
- `loctree_rs/src/analyzer/crowd/output.rs` — wyświetlanie

### Use cases
- Agent wie że crowd "message" to `state` → szuka hooks/stores
- Lepsza kategoryzacja w raportach
- Filtrowanie crowds po typie

---

## 8. `loct layoutmap` — CSS Layout Analysis

### Problem
Agent nie widzi "warstw" UI — które elementy są sticky, jakie są z-index, gdzie są grid/flex containers.

### Rozwiązanie
```bash
loct layoutmap                           # pełna analiza
loct layoutmap --zindex-only             # tylko z-index
loct layoutmap --sticky-only             # tylko sticky/fixed
loct layoutmap --grid-only               # tylko grid/flex
loct layoutmap --min-zindex 100          # filtr wysokich z-index
loct layoutmap --exclude .obsidian       # wykluczanie ścieżek
loct layoutmap --json
```

### Output
```
Z-INDEX LAYERS (sorted by z-index):
  z-index:   9999  body::before  (styles/main.css:60)
  z-index:    100  .nav  (styles/nav.css:11)
  z-index:     50  .float-zone  (styles/layout.css:432)

STICKY/FIXED ELEMENTS:
  .nav  fixed  (styles/nav.css:7)
  .settings-bar sticky  (styles/settings.css:389)

CSS GRID CONTAINERS:
  .app-container  (styles/layout.css:13)
  .dashboard-grid  (styles/dashboard.css:8)

FLEX CONTAINERS:
  .card-content  (styles/card.css:3)
  .nav-items  (styles/nav.css:25)

Total: 16 z-index, 9 sticky/fixed, 12 grid, 45 flex
```

### Skanowane pliki
- CSS/SCSS/SASS/LESS
- CSS-in-JS (styled-components, emotion) w JS/TS/JSX/TSX

### Domyślnie ignorowane
- `node_modules`, `.git`, `dist/`, `build/`, `target/`, `.next/`, `coverage/`

### Pliki
- `loctree_rs/src/layoutmap.rs` (NEW ~350 LOC)
- `loctree_rs/src/cli/command.rs` — `LayoutmapOptions`
- `loctree_rs/src/cli/parser.rs` — `parse_layoutmap_command()`
- `loctree_rs/src/cli/dispatch/handlers/analysis.rs` — `handle_layoutmap_command()`

### Use cases
- Debugowanie z-index conflicts
- Mapowanie sticky/fixed elementów dla scroll behavior
- Understanding layout architecture
- Audit przed redesignem

---

## 9. `--help-full` — Extended Help

### Status
Już istniało — wyświetla pełny legacy help z wszystkimi flagami.

```bash
loct --help-full
```

---

## 10. Shell Completions (bash/zsh)

### Problem
Brak tab-completion dla `loct` komend i flag.

### Rozwiązanie
Nowy katalog `loctree_rs/completions/`:

```
completions/
├── loct.bash    # Bash completion
└── _loct        # Zsh completion
```

### Instalacja

**Bash:**
```bash
# Opcja 1: source w .bashrc
echo 'source /path/to/loct.bash' >> ~/.bashrc

# Opcja 2: system-wide
sudo cp loct.bash /etc/bash_completion.d/loct
```

**Zsh:**
```bash
# Dodaj do fpath
mkdir -p ~/.zsh/completions
cp _loct ~/.zsh/completions/
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
```

### Pokrycie
- Wszystkie subcommands (slice, find, dead, twins, crowd, focus, hotspots, layoutmap, etc.)
- Wszystkie flagi per-command
- File/directory completion gdzie appropriate

---

## Podsumowanie zmian w plikach

### Nowe pliki
| Plik | LOC | Opis |
|------|-----|------|
| `src/focuser.rs` | ~350 | Directory-level holographic focus |
| `src/layoutmap.rs` | ~350 | CSS layout analysis |
| `completions/loct.bash` | ~130 | Bash completion |
| `completions/_loct` | ~200 | Zsh completion |

### Zmodyfikowane pliki
| Plik | Zmiany |
|------|--------|
| `src/cli/command.rs` | +FocusOptions, +TagmapOptions, +HotspotsOptions, +LayoutmapOptions, help texts |
| `src/cli/parser.rs` | +parse_focus/tagmap/hotspots/layoutmap_command(), --py-root, --rescan |
| `src/cli/dispatch/mod.rs` | routing dla nowych komend |
| `src/cli/dispatch/handlers/analysis.rs` | handlery dla nowych komend |
| `src/analyzer/twins.rs` | --include-tests support |
| `src/analyzer/crowd/types.rs` | +ContextType enum |
| `src/analyzer/crowd/mod.rs` | +infer_context_type() |
| `src/lib.rs` | +pub mod focuser, layoutmap |

---

## Testowanie

### Quick smoke test
```bash
# Build
cargo build --release -p loctree

# Test nowych komend
loct focus src/cli/
loct tagmap message
loct hotspots --limit 10
loct layoutmap --zindex-only
loct twins --include-tests
loct crowd --json | jq '.[0].context_type'
```

### Na zewnętrznym projekcie
```bash
cd ~/Git/vista
loct layoutmap --exclude .obsidian --exclude prototype
loct hotspots --leaves
loct focus src/features/ai-suite/
```

---

## Breaking Changes

**Brak.** Wszystkie zmiany są addytywne.

---

## Znane limitacje

1. **layoutmap @layer selectors** — CSS Layers (`@layer components { ... }`) pokazują `@layer components` jako selector zamiast rzeczywistego selectora wewnątrz. To limitacja prostego parsera regex.

2. **focus depth** — `--depth` flag jest zaimplementowany ale nie w pełni przetestowany dla głębokich hierarchii.

3. **tagmap performance** — Na bardzo dużych projektach może być wolny bo wykonuje multiple passes (find + crowd + dead).

---

## Next steps (propozycje)

1. **layoutmap CSS variables** — Wykrywanie `--z-index-modal: 100` custom properties
2. **focus --diff** — Porównanie focus między branchami
3. **hotspots --graph** — Wizualizacja dependency graph dla hotspots
4. **completions fish** — Fish shell completion

---

*Dokument wygenerowany: 2024-12-14*
*Sesja prowadzona przez: Claude (Opus) + Monika*
