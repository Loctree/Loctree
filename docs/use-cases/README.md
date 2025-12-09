# Loctree Use Cases

Real-world validation of loctree on major open-source repositories.

## Source Repository Tests (Language/Framework Origins)

| Repository | Stack | Files | FP Rate | Verdict |
|------------|-------|-------|---------|---------|
| [rust-lang/rust](rust-lang-rust.md) | Rust | 35,387 | 0% | EXCEPTIONAL |
| [golang/go](golang-go.md) | Go | 17,182 | ~0% | PERFECT |
| [facebook/react](facebook-react.md) | JSX/TSX | 3,951 | 40% | B+ PASSED |
| [sveltejs/svelte](sveltejs-svelte.md) | TS/JS | 405 | 70%* | PASSED |
| [python/cpython](python-cpython.md) | Python | 842 | 100%* | N/A |
| [denoland/deno](denoland-deno.md) | Rust | 611 | ~0% | EXCELLENT |

*High FP expected for library/stdlib code - need `--library-mode`

## Framework Tests

| Repository | Stack | Files | FP Rate | Verdict |
|------------|-------|-------|---------|---------|
| [tauri-apps/tauri](tauri-apps-tauri.md) | Rust+TS | 385 | 0% | PERFECT |
| [tiangolo/fastapi](fastapi-fastapi.md) | Python | 1,184 | 0% | PERFECT |
| [vuejs/core](vuejs-core.md) | TS+Vue | ~500 | 0% | EXCELLENT |
| [sveltejs/kit](sveltejs-kit.md) | TS+Svelte | 1,117 | ~0% | EXCELLENT |

## Key Achievements

### Performance
- **rust-lang/rust**: 35K files in 45 seconds (787 files/sec)
- **golang/go**: 17K files in 107 seconds (160 files/sec)

### Accuracy
- **0% FP** on: Tauri, FastAPI, Go, Vue, SvelteKit, Deno
- **Cycle Detection**: 91 real cycles found in Rust compiler

### Features Validated
- **Tauri Commands**: Perfect FE↔BE bridge tracking
- **FastAPI Routes**: 451 endpoints detected
- **Vue SFC**: Full `<script setup>` support
- **Virtual Modules**: `$app/*`, `$lib/*` resolution

## Known Limitations

### Library Mode Needed
For public API analysis (libraries, frameworks, stdlib):
- sveltejs/svelte: 70% FP on public exports
- python/cpython: 100% FP on `__all__` exports

**Recommendation**: Use `--library-mode` (planned feature) or focus on internal modules.

### Edge Cases
- Svelte component method refs (GitButler regression)
- Rust associated/const functions (Zed)
- Python module attribute access patterns

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
