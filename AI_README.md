# loctree – szybki przewodnik dla agentów (v0.4.3)

Najważniejsze (0.4.3):
- HTML raport przestał dublować kontrolki grafu: inline graph/toolbar są ukryte, zostaje tylko drawer na dole (koniec z podwójnym scrollem).
- Multi-root FE↔BE coverage jest scalane globalnie, więc `commands2`/coverage w JSON/CLI nie raportują sztucznych braków/nieużyć między src vs src-tauri.
- Duplikaty pomijają re-eksporty (barrels/index.ts) i `default` z plików deklaracji, redukując hałas z beczek TS.
- Stałe eventów są rozwiązywane między plikami (TS/JS/Rust, także importowane), więc ghost/orphan w pipeline/aiViews są bliżej prawdy.

## Instalacja / update (globalnie)
```bash
cargo install --force --path loctree_rs
# weryfikacja:
loctree --version   # oczekiwane: 0.4.3
```

## Tryby
- **Tree (domyślnie)** – drzewo plików z LOC highlight.
- **Analyzer (`-A`)** – importy/eksporty, duplikaty, re-eksporty, dynamiczne importy, pokrycie komend Tauri, graf.
- **Preset Tauri** – `--preset-tauri` ustawia rozszerzenia + preset ignore-symbols i auto-ignore artefaktów buildowych.

## Kluczowe flagi
### Tree
- `--summary[=N]` – podsumowanie + top duże pliki (domyślnie 5).
- `--loc <n>` – próg LOC dla highlight (domyślnie 1000).
- `-L, --max-depth <n>` – limit głębokości (0 = tylko dzieci).
- `-H, --show-hidden` – pokaż pliki ukryte.

### Analyzer (-A)
- `--ext <list>` – rozszerzenia (domyślnie: ts,tsx,js,jsx,mjs,cjs,rs,css,py).
- `--limit <N>` – top-N (duplikaty, dynamiczne importy), domyślnie 8.
- `--ai` – kompaktowy JSON dla agentów (top issues + bridges, bez sekcji per-file; dobre do krótkich kontekstów).
- `--top-dead-symbols <N>` / `--skip-dead-symbols` – kontrola listy dead symbols (domyślnie 20 lub całkowicie pomiń).
- `--ignore-symbols <list>` / `--ignore-symbols-preset common|tauri` – filtr szumu (np. main/run/setup/__all__/test_*).
- `--focus <glob>` / `--exclude-report <glob>` – filtrowanie widoku duplikatów (analiza pełna).
- `--py-root <path>` – dodatkowe rooty Pythona (powtarzalne); pyproject jest nadal wykrywany.
- `--html-report <file>` – zapis HTML; `--graph` dokłada interaktywny graf (Cytoscape lokalnie).
- `--serve` – wymaga `--html-report`; uruchamia lokalny serwer do otwierania plików w edytorze/OS (`--editor-cmd` do szablonu, default: VS Code -> open/xdg-open).
- `--max-graph-nodes/--max-graph-edges` – limity bezpieczeństwa (gdy przekroczone, graf jest pomijany z ostrzeżeniem).

### Wspólne
- `-I, --ignore <path>` – ignoruj ścieżkę (powtarzalne).
- `-g, --gitignore` – respektuj .gitignore.
- `--color[=auto|always|never]` – kolory (domyślnie auto).
- `--json` – JSON na stdout (tree/analyzer); `--jsonl` (analyzer) – jeden JSON na linię per root.

## Raport HTML (zakładki + drawer)
- Sekcje: Overview (AI Insights), Duplicates, Dynamic imports, Tauri coverage, Graph (kotwica).
- Graf i kontrolki są w dolnym drawerze (toggle). Toolbar: filtr tekstowy, min-degree, labels on/off, fit/reset/fullscreen/dark, PNG/JSON, panel komponentów (wyspy) z highlight/dim/copy/export.
- Tooltipy są przypięte do kursora; filtrowanie po ścieżkach i rozmiarze komponentów.

## JSON (schema 1.2.0)
- `files[*].imports` mają `resolutionKind` (local|stdlib|dynamic|unknown) oraz `isTypeChecking`.
- `aiViews.commands2` – FE↔BE komendy (status: ok/missing_handler/unused_handler + alias impl); w `--ai` dostępne w skróconej liście `bridges`.
- `symbols/clusters` – grupy duplikatów z canonical, score, reasons.
- `dynamicImports` – statyczne + dynamiczne (importlib/__import__/f-strings).
- `graphs` – osobno, gdy `--graph` i limity pozwalają.

## Przykłady (Monika/agent)
- Pełny scan FE+BE z raportem (mniej szumu dzięki auto-ignore w preset-tauri):
```bash
loctree -A --preset-tauri src src-tauri/src --graph \
  --exclude-report "**/__tests__/**" \
  --json-out .ai-agents/loctree/reports/loctree.json \
  --html-report .ai-agents/loctree/reports/loctree.html \
  --serve --verbose
```
- Szybki JSON tylko dla FE:
```bash
loctree -A src --ext ts,tsx --gitignore --limit 5 --json > /tmp/loctree.json
```
- Python-only z dodatkowymi rootami:
```bash
loctree -A backend --ext py --py-root backend/src --gitignore --graph \
  --html-report /tmp/loctree-py.html
```

## Notatki operacyjne
- Graph może być pominięty przy dużych kodach – sprawdź ostrzeżenie w HTML/CLI i ewentualnie podbij limity.
- `--serve` wymaga, by proces loctree pozostał uruchomiony (nie zabijaj sesji).
- W ciemnym motywie raportu graf dostosowuje kolory (tryb dark w toolbarze draweru).
- Pomoc “per-mode” dostępna przez `loctree --help` (podział na Tree/Analyzer/Common).

## Troubleshooting
- “Root ... is not a directory” – podaj ścieżki względem bieżącego cwd lub użyj bezwzględnych.
- Brak grafu – sprawdź limity (`--max-graph-nodes/edges`), ewentualnie uruchom z mniejszym zakresem (`--focus` lub węższe rooty).
- Tauri pokrycie wygląda na szum – w Viście część komend idzie przez wraper safeInvoke; patrz `commands2` w JSON, aliasy impl są raportowane. 
