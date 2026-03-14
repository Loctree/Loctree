# ADR: Context over Memory

- **Status:** Accepted
- **Date:** 2026-02-17
- **Owners:** loctree maintainers

## Context

Agentic workflows have been increasingly optimized around persistent memory and retrieval layers. In practice, we observed recurring issues:

- startup context bloat,
- stale retrieval against changing codebases,
- poor reproducibility across sessions,
- low explainability of "why this context was selected."

At the same time, loctree already provides deterministic structural perception (`repo-view`, `focus`, `slice`, `impact`, `find`) with current-state guarantees.

## Decision

We standardize on **context-over-memory** as the primary architecture for agent workflows:

1. Use graph-backed, on-demand perception as default context acquisition.
2. Use prepared context bundles for predictable recurrent workflows.
3. Keep long-term memory optional and subordinate to fresh structural data.

## Scope

This ADR applies to:

- CLI and MCP usage guidance,
- documentation and examples,
- internal integration recommendations for agent teams.

It does not force removal of existing memory integrations; it defines priority and default behavior.

## Decision Drivers

- Determinism
- Freshness
- Debuggability
- Lower latency
- Lower token overhead
- Safer refactors

## Consequences

### Positive

- Better first-pass accuracy in code modifications.
- Lower context-token overhead for multi-step tasks.
- Clear provenance of context ("which command produced this view?").
- Easier production incident analysis.

### Negative / Trade-offs

- Requires disciplined context mapping habits.
- Increases reliance on tool availability and quality.
- Some longitudinal use cases still benefit from memory layers.

## Alternatives considered

### A) Memory-first (vector retrieval as primary)

Rejected as default due to probabilistic recall and drift under active code change.

### B) Huge static preloaded prompts

Rejected due to latency, cost, and context pollution.

### C) Hybrid with memory as secondary (chosen)

Accepted: memory can enrich, but perception and scoped context remain source of truth.

## Guardrails

Before non-trivial edits, workflows should perform:

1. `repo-view`
2. `focus`
3. `slice`
4. `impact`
5. `find`

Use grep as local detail tool, not as primary mapping layer.

## Rollout

1. Publish manifesto and KPI definitions.
2. Add these docs to `docs/README.md`.
3. Update integration examples to reference this ADR.
4. Track KPI deltas for at least 2 release cycles.

## Success Criteria

See: [Agent Context KPIs](../metrics/agent-context-kpis.md).

This ADR is considered successful if reliability and context efficiency improve without reducing task throughput.
