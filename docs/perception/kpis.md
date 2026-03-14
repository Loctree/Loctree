# Agent Context KPIs

## Purpose
Define measurable outcomes for the `context-over-memory` architecture and track whether reliability improves without sacrificing throughput.

## Measurement Window
- Track per release and rolling 30-day windows.
- Compare against a baseline collected before ADR `2026-02-17-context-over-memory`.
- Evaluate for at least 2 release cycles.

## Primary KPIs

### 1) First-Pass Correct Change Rate (FPCR)
Percentage of agent-authored changes accepted without rework requests.

Formula:
`FPCR = accepted_without_rework / total_agent_changes`

Target trend:
- Up and to the right.
- No release should regress more than 5% from previous stable baseline.

### 2) Context Token Cost per Completed Task (CTC)
Average context tokens consumed to complete one successful task.

Formula:
`CTC = total_context_tokens / completed_tasks`

Target trend:
- Decrease while maintaining or improving FPCR.

### 3) Context Provenance Coverage (CPC)
Share of non-trivial tasks where context sources are explicitly traceable to commands.

Formula:
`CPC = tasks_with_command_provenance / non_trivial_tasks`

Minimum threshold:
- 90%+

### 4) Freshness Violation Rate (FVR)
Rate of incidents where stale context caused wrong edits or wrong recommendations.

Formula:
`FVR = stale_context_incidents / total_agent_tasks`

Target trend:
- Near zero.

### 5) Mean Context Bootstrap Time (MCBT)
Median time from task start to first actionable edit.

Formula:
`MCBT = median(t_first_edit - t_task_start)`

Target trend:
- Flat or down, despite stricter mapping guardrails.

### 6) Post-Review Rework Rate (PRR)
Rate of tasks requiring follow-up fixes after initial review.

Formula:
`PRR = tasks_with_post_review_rework / reviewed_tasks`

Target trend:
- Down.

## Secondary KPIs
- `Tool Coverage Rate`: percentage of non-trivial tasks that used the full mapping chain (`repo-view`, `focus`, `slice`, `impact`, `find`).
- `Refactor Safety Incident Rate`: breakages introduced in medium/large refactors.
- `Throughput`: completed tasks per engineer-day (must not materially decline).

## Instrumentation Guidance
- Log task metadata: task size, touched files, commands used, token usage, outcome.
- Store a compact execution trace for reproducibility.
- Tag incidents with root cause category: stale context, missing context, tool failure, logic error.

## Success Criteria
The ADR is successful when:
1. FPCR improves.
2. CTC decreases or stays flat with better FPCR.
3. FVR stays near zero.
4. Throughput does not regress materially.
