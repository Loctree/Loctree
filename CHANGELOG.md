# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.2.7] - 2025-11-22

### Added
- `--graph` opcjonalnie dokleja interaktywny graf importów/re-eksportów do raportu HTML (Cytoscape.js z CDN).
- `--ignore-symbols-preset <name>` (na razie `common` → `main,run,setup,test_*`) oraz wsparcie prefiksów `foo*` w `--ignore-symbols`.

### Changed
- Help/README/Monika guide uzupełnione o nowe flagi; analiza duplikatów uwzględnia wzorce prefiksowe.

## [0.2.6] - 2025-11-22

### Added
- Flaga `--ignore-symbols` dla analizera – pozwala pominąć wskazane symbole (np. `main,run`) przy wykrywaniu duplikatów eksportów.

### Changed
- Dokumentacja i help zaktualizowane o nową flagę.

## [0.2.5] - 2025-11-22

### Added
- Import/export analyzer obejmuje Pythona: `import`/`from`/`__all__`, wykrywa dynamiczne `importlib.import_module` oraz `__import__`, raportuje re-export przez `from x import *`.
- Domyślne rozszerzenia analizera zawierają teraz `py`.

### Changed
- README i przewodnik Moniki uzupełnione o wsparcie Pythona.

## [0.2.4] - 2025-11-22

### Added
- Optional `--serve` mini HTTP server: HTML raport zawiera klikalne linki `file:line` otwierające się w edytorze/OS (`code -g` domyślnie, konfigurowalne `--editor-cmd`). Bezpieczne: ścieżki kanonikalizowane i ograniczone do podanych rootów.
- Raporty i JSON zawierają lokalizacje wywołań/handlerów komend Tauri, co przyspiesza diagnozę FE↔BE.

### Changed
- `--serve`/`--editor-cmd` opisane w pomocy/README; auto-open raportu w przeglądarce pozostaje.

## [0.2.3] - 2025-11-22

### Added
- Analyzer raportuje pokrycie komend Tauri: wywołania FE (`safeInvoke`/`invokeSnake`) vs. handlery z `#[tauri::command]` w Rust; pokazuje brakujące i nieużywane handlery również w raporcie HTML/JSON/CLI.

### Changed
- Hardening auto-open HTML (kanonikalizacja ścieżki, brak kontroli znaków sterujących).
- Ujednolicone zależności: `regex = 1.12` w manifest.
- Ukryte pliki rozpoznawane wyłącznie po kropce (bez specjal-case `.DS_Store`).

## [0.2.2] - 2025-11-22

### Added
- Analyzer now understands CSS `@import` and Rust `use`/`pub use`/public items; default analyzer extensions expanded to include `rs` and `css`.
- HTML report auto-open remains; help/README updated to note new language coverage.

### Changed
- Hidden-file detection no longer special-cases `.DS_Store`; relies on leading dot + `--show-hidden`.

## [0.2.0] - 2025-11-21

### Added
- Unified CLI features and JSON output across all runtimes (Node.js, Python, Rust): extension filters, ignore patterns, gitignore support, max depth, color modes, JSON output, and summary reporting (commit [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).
- Installation scripts for fast setup: `install.sh`, `install_node.sh`, and `install_py.sh` (commit [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4)).
- `--show-hidden` (`-H`) option to include dotfiles and other hidden entries in output in Rust and Python CLIs (commit [`12310b4`](https://github.com/LibraxisAI/loctree/commit/12310b4)).

### Changed
- Standardized the project name from `loc-tree` to `loctree` across runtimes, binaries, installers, and documentation; improved CLI UX and argument parsing, and enhanced error messages (commit [`e31d3a4`](https://github.com/LibraxisAI/loctree/commit/e31d3a4)).
- Usage/help output refined and examples clarified across Rust, Node, and Python CLIs (commit [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4) and [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).

### Documentation
- Expanded and clarified README with installation instructions, usage details, examples, and project structure overview (commits [`e31d3a4`](https://github.com/LibraxisAI/loctree/commit/e31d3a4), [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4), [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).

### Other
- Initial project setup (commit [`2031f80`](https://github.com/LibraxisAI/loctree/commit/2031f80)).

---

Release notes are generated from the last 5 commits on the default branch (`main`).
