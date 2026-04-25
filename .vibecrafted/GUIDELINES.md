## VibeCrafted Guidelines

This repository keeps the canonical project contract in `AGENTS.md`.
Read that first, then use these repo-specific defaults:

- Map before you edit: `repo-view -> focus -> slice -> impact -> find -> follow`.
- Rust quality gate: `make precheck` for a fast pass, `make check` for the full repo gate, and `make test` before release-facing changes.
- Treat `loctree_rs/src/types.rs`, `loctree_rs/src/snapshot.rs`, and `reports/src/types.rs` as blast-radius hubs.
- Distribution truth lives under `distribution/`; do not invent release notes or install paths outside that spine.
- `loct` is the canonical CLI. `loctree` remains a compatibility alias and should stay quiet unless behavior truly changes.
