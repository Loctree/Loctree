# Loctree Use Cases

Real-world validation of loctree on major open-source repositories.

## Source Repository Tests (Language/Framework Origins)

| Repository | Stack | Files | FP Rate | Verdict |
|------------|-------|-------|---------|---------|
| [rust-lang/rust](rust-lang-rust.md) | Rust | 35,387 | 0% | EXCEPTIONAL |
| [golang/go](golang-go.md) | Go | 17,182 | ~0% | PERFECT |
| [facebook/react](facebook-react.md) | JSX/TSX | 3,951 | ~20% | B+ PASSED |
| [sveltejs/svelte](sveltejs-svelte.md) | TS/JS | 405 | 70%* | PASSED |
| [python/cpython](python-cpython.md) | Python | 842 | 100%* | N/A |
| [denoland/deno](denoland-deno.md) | Rust | 611 | ~0% | EXCELLENT |

*High FP expected for library/stdlib code - library mode auto-detection improves this

## Framework Tests

| Repository | Stack | Files | FP Rate | Verdict |
|------------|-------|-------|---------|---------|
| [tauri-apps/tauri](tauri-apps-tauri.md) | Rust+TS | 385 | 0% | PERFECT |
| [tiangolo/fastapi](fastapi-fastapi.md) | Python | 1,184 | 0% | PERFECT |
| [vuejs/core](vuejs-core.md) | TS+Vue | ~500 | 0% | EXCELLENT |
| [sveltejs/kit](sveltejs-kit.md) | TS+Svelte | 1,117 | ~0% | EXCELLENT |

## Blocked Tests (UTF-8 Limitations)

| Repository | Issue | Workaround |
|------------|-------|------------|
| [nodejs/node](nodejs-node.md) | Binary ICU data files | Scan `lib/` only |
| [microsoft/TypeScript](microsoft-typescript.md) | Malformed test fixtures | Scan `src/` only |

## Key Achievements

### Performance
- **rust-lang/rust**: 35K files in 45 seconds (~787 files/sec)
- **golang/go**: 17K files in 107 seconds (~160 files/sec)
- **facebook/react**: 4K files in 49 seconds (~81 files/sec)

### Accuracy
- **0% FP** on: Tauri, FastAPI, Go, Vue, SvelteKit, Deno
- **~20% FP** on: React (WeakMap/WeakSet patterns, import aliasing, type-only exports)
- **Cycle Detection**: 91 intra-crate cycles found in Rust compiler (architectural, not bugs)

### Features Validated
- **Tauri Commands**: Perfect FE↔BE bridge tracking
- **FastAPI Routes**: 451 endpoints detected
- **Vue SFC**: Full `<script setup>` support
- **Virtual Modules**: `$app/*`, `$lib/*` resolution

## Known Limitations

### Library Mode Auto-Detection
For public API analysis (libraries, frameworks, stdlib):
- npm packages with `package.json` "exports" field: auto-detected
- Python stdlib (Lib/ directory): auto-detected
- Python `__all__` exports: properly tracked and excluded

**Remaining edge cases**:
- sveltejs/svelte: 70% FP on public exports (type-only re-exports)
- python/cpython: Manual `--library-mode` recommended for full stdlib analysis

### Edge Cases
- Svelte component method refs (GitButler regression)
- Rust associated/const functions (Zed)
- Python module attribute access patterns

### UTF-8 Handling
Repositories with intentionally malformed files (compiler test suites) need:
- `--skip-invalid-utf8` flag (planned)
- `.loctreeignore` support (planned)

## Note on Rust Circular Dependencies

Loctree detects intra-crate module cycles in Rust (91 in rust-lang/rust). These are **NOT bugs**:
- Rust allows modules within the same crate to mutually import
- Compiler resolves via lazy name resolution
- Different from JS/Python where circular imports cause runtime issues

## Test Methodology

Each repository tested with:
```bash
git clone --depth 1 <repo>
cd <repo>
rm -rf .loctree && loct           # Fresh snapshot
loct dead --confidence high       # Dead code analysis
loct twins                        # Duplicate detection
loct cycles                       # Circular imports
```

Sample verification: 10 random findings checked with `rg`.

---

*Tested by M&K (c)2025 The LibraxisAI Team*
