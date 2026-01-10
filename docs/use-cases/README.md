# Loctree Use Cases

Real-world validation of loctree on major open-source repositories.

## Source Repository Tests

| Repository | Stack | Files | FP Rate | Verdict | Details |
|------------|-------|-------|---------|---------|---------|
| rust-lang/rust | Rust | 35,387 | 0% | EXCEPTIONAL | [→](18_rust.md) |
| golang/go | Go | 17,182 | ~0% | PERFECT | [→](14_golang.md) |
| facebook/react | JSX/TSX | 3,951 | ~20% | B+ PASSED | [→](11_react.md) |
| sveltejs/svelte | TS/JS | 405 | 70%* | PASSED | [→](20_svelte.md) |
| python/cpython | Python | 842 | 100%* | N/A | [→](17_cpython.md) |
| denoland/deno | Rust | 611 | ~0% | EXCELLENT | [→](10_deno.md) |

*High FP expected for library/stdlib code - library mode auto-detection improves this

## Framework Tests

| Repository | Stack | Files | FP Rate | Verdict | Details |
|------------|-------|-------|---------|---------|---------|
| tauri-apps/tauri | Rust+TS | 385 | 0% | PERFECT | [→](21_tauri.md) |
| tiangolo/fastapi | Python | 1,184 | 0% | PERFECT | [→](12_fastapi.md) |
| vuejs/core | TS+Vue | ~500 | 0% | EXCELLENT | [→](22_vue.md) |
| sveltejs/kit | TS+Svelte | 1,117 | ~0% | EXCELLENT | [→](19_sveltekit.md) |

## Internal Use Cases

| Use Case | Description | Details |
|----------|-------------|---------|
| Circular Imports Fix | How to detect and fix circular dependencies | [→](01_circular_imports_fix.md) |
| Dead Exports Massacre | Cleaning up unused exports at scale | [→](02_dead_exports_massacre.md) |
| Dogfooding False Positives | Tracking and fixing loctree's own FP issues | [→](03_dogfooding_false_positive.md) |
| Event Flow Audit | Analyzing event-driven architectures | [→](04_event_flow_audit.md) |
| Rust Crate Imports | Handling Rust module systems | [→](05_rust_crate_imports.md) |
| Tauri Commands | Frontend-backend contract validation | [→](06_tauri_commands.md) |
| Vista Tauri Contract | Full Tauri app analysis case study | [→](07_vista_tauri_contract.md) |
| ripgrep + loct Synergy | Combining tools effectively | [→](08_rg_loct_synergy.md) |

## Performance Benchmarks

| Repository | Files | Time | Rate |
|------------|-------|------|------|
| rust-lang/rust | 35,387 | 45s | ~787 files/sec |
| golang/go | 17,182 | 107s | ~160 files/sec |
| facebook/react | 3,951 | 49s | ~81 files/sec |

## Accuracy Summary

- **0% FP**: Tauri, FastAPI, Go, Vue, SvelteKit, Deno
- **~20% FP**: React (WeakMap/WeakSet patterns, import aliasing)
- **High FP**: Library/stdlib code (use `--library-mode`)

## Test Methodology

```bash
git clone --depth 1 <repo>
cd <repo>
loct                              # Build snapshot
loct dead --confidence high       # Dead code analysis
loct twins                        # Duplicate detection
loct cycles                       # Circular imports
```

---

*Tested by M&K (c)2025 The LibraxisAI Team*
