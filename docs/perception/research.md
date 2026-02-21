# Global Direction Research: Context over Memory

- **Date:** 2026-02-17
- **Method:** synthesis from `web.run` page reads and Brave web search across primary sources (vendor docs, protocol specs, research papers).
- **Objective:** validate whether `context-over-memory` is aligned with broader agent engineering direction.

## Executive Summary
Global direction is converging on **context engineering** and **tool-mediated perception** rather than pure memory-first retrieval. The dominant pattern is:

1. Keep context windows curated and scoped.
2. Retrieve fresh context through tools/protocols (especially MCP).
3. Use memory as a secondary layer for continuity, not as source of truth for current state.

This supports loctree’s `repo-view -> focus -> slice -> impact -> find` workflow as a production-oriented default.

## Evidence

### 1) Vendor guidance now emphasizes context quality over raw context volume
- Anthropic’s agent guidance prioritizes simplicity, transparency, and strong tool interfaces over unnecessary complexity.
- Anthropic’s context-engineering guidance frames context as a managed state with finite attention budget.
- OpenAI’s agent guidance emphasizes tool-based context retrieval, guardrails, and workflow orchestration instead of monolithic prompts.

### 2) MCP is becoming standard context infrastructure
- Official MCP specification (2025-06-18 revision) formalizes tools/resources/prompts and transport/lifecycle behavior.
- OpenAI documents MCP usage across API, connectors, and Codex workflows.
- Google Cloud announced official MCP support for Google services and Cloud systems.
- Microsoft announced GA MCP integrations in Copilot Studio and Visual Studio experiences.

Inference from these sources: the ecosystem is standardizing on **protocolized context access** (discoverable, auditable, composable) rather than bespoke memory stacks.

### 3) Long-context research still supports selective context strategies
- *Lost in the Middle* shows degradation when relevant information sits in the middle of long inputs, supporting selective retrieval and ordering.
- ReAct and Toolformer-era evidence supports tool use for grounding and action over pure in-prompt recall.

Inference: larger windows help, but do not eliminate the need for careful context selection, compression, and provenance.

## Implications for loctree
`Context-over-memory` is not a niche stance; it is compatible with current industry movement:

- **Determinism:** tool call provenance is auditable.
- **Freshness:** context is generated from current repository state.
- **Debuggability:** failures can be traced to command outputs.
- **Cost control:** fewer irrelevant tokens than broad memory preload.

Repository-specific mapping and roadmap are documented separately:
- `docs/research/loctree-codebase-map-and-perception-first-vision-2026-02-17.md`

## Recommended Direction (next 2 cycles)
1. Keep perception-first guardrails as default for non-trivial edits.
2. Instrument KPIs from `docs/metrics/agent-context-kpis.md`.
3. Reference ADR/manifest in integration docs and examples.
4. Keep memory integrations available, but explicitly secondary to structural context.

## Sources
Primary:
- MCP spec and docs: https://modelcontextprotocol.io/specification/2025-06-18
- MCP GitHub repository: https://github.com/modelcontextprotocol/modelcontextprotocol
- Anthropic: Building Effective AI Agents: https://www.anthropic.com/research/building-effective-agents
- Anthropic: Effective context engineering for AI agents: https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- OpenAI practical guide: https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/
- OpenAI building agents track: https://developers.openai.com/tracks/building-agents/
- OpenAI MCP guide: https://platform.openai.com/docs/mcp
- OpenAI connectors + MCP: https://platform.openai.com/docs/guides/tools-connectors-mcp
- OpenAI Codex MCP docs: https://developers.openai.com/codex/mcp
- OpenAI cookbook (state/context pattern): https://cookbook.openai.com/examples/agents_sdk/context_personalization
- Google Cloud MCP support announcement: https://cloud.google.com/blog/products/ai-machine-learning/announcing-official-mcp-support-for-google-services
- Microsoft Copilot Studio MCP GA: https://www.microsoft.com/en-us/microsoft-copilot/blog/copilot-studio/model-context-protocol-mcp-is-now-generally-available-in-microsoft-copilot-studio/
- Microsoft Visual Studio MCP GA: https://devblogs.microsoft.com/visualstudio/mcp-is-now-generally-available-in-visual-studio/
- Lost in the Middle (ACL/TACL): https://aclanthology.org/2024.tacl-1.9/
- ReAct paper: https://arxiv.org/abs/2210.03629
- Toolformer paper: https://arxiv.org/abs/2302.04761

Supplementary signals (ecosystem/adoption tracking):
- Google adoption coverage: https://techcrunch.com/2025/04/09/google-says-itll-embrace-anthropics-standard-for-connecting-ai-models-to-data/
