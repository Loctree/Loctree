# Loctree + rmcp_memex Integration Proposal

**Date:** December 2025  
**Status:** Draft  
**Authors:** rmcp_memex team  

---

## Executive Summary

This proposal outlines a semantic search integration between **loctree** (AI-oriented static code analyzer) and **rmcp_memex** (MCP-based RAG/memory server). The integration would enable AI agents to perform semantic queries over code analysis reports, dead code detection, and structural insights.

---

## Problem Statement

AI agents (Claude, GPT, Copilot) working with codebases need:
1. **Contextual understanding** of code structure beyond simple grep
2. **Semantic search** over analysis findings (dead code, duplicates, cycles)
3. **Persistent memory** of analysis results across sessions
4. **Natural language queries** like "what symbols are unused in the storage module?"

Currently, loctree generates excellent reports (SARIF, JSON, HTML), but they're not easily queryable by AI agents in natural language.

---

## Proposed Solution

### Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│    loctree      │────▶│   rmcp_memex     │◀────│   AI Agent      │
│  (analyzer)     │     │  (RAG + memory)  │     │ (Claude, etc.)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                        │
        ▼                        ▼
   analysis.json            LanceDB
   snapshot.json          (embeddings)
   report.sarif
```

### Data Flow

1. **loctree** runs `loct scan` / `loct report` → generates JSON/SARIF
2. **rmcp_memex** ingests reports via new `loctree_index` tool
3. **AI Agent** queries via `rag_search` / `memory_search` tools
4. Results include relevant code findings with semantic ranking

---

## New MCP Tools

### `loctree_index`

Index loctree analysis outputs into the vector store.

```json
{
  "name": "loctree_index",
  "description": "Index loctree analysis reports for semantic search",
  "inputSchema": {
    "type": "object",
    "properties": {
      "report_path": {
        "type": "string",
        "description": "Path to .loctree directory or specific JSON/SARIF file"
      },
      "namespace": {
        "type": "string",
        "default": "loctree",
        "description": "Namespace for organizing indexed data"
      },
      "project_id": {
        "type": "string",
        "description": "Project identifier (e.g., git repo name)"
      }
    },
    "required": ["report_path"]
  }
}
```

### `loctree_search`

Semantic search over indexed loctree findings.

```json
{
  "name": "loctree_search",
  "description": "Search loctree findings semantically",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Natural language query about code analysis"
      },
      "finding_type": {
        "type": "string",
        "enum": ["dead_code", "duplicates", "cycles", "all"],
        "default": "all"
      },
      "project_id": {
        "type": "string"
      },
      "k": {
        "type": "integer",
        "default": 10
      }
    },
    "required": ["query"]
  }
}
```

---

## Embedding Strategy

### What to Embed

| Source File | Embeddable Units | Metadata |
|-------------|------------------|----------|
| `analysis.json` | Dead symbols, duplicate clusters, barrel exports | severity, paths, symbol names |
| `snapshot.json` | File summaries, import graphs, export lists | LOC, language, is_test |
| `report.sarif` | Individual findings/warnings | ruleId, level, location |

### Chunking Strategy

```rust
struct LoctreeFinding {
    id: String,           // e.g., "dead:FastEmbedder@src/embeddings/mod.rs"
    finding_type: String, // "dead_code" | "duplicate" | "cycle" | "lint"
    text: String,         // Natural language description for embedding
    metadata: LoctreeMetadata,
}

struct LoctreeMetadata {
    project_id: String,
    git_commit: String,
    file_path: String,
    symbol_name: Option<String>,
    severity: String,
    rule_id: Option<String>,
}
```

### Example Embedded Text

For a dead symbol finding:
```
Dead code: symbol 'FastEmbedder' in src/embeddings/mod.rs is declared as public 
but appears unused. This struct provides text embedding functionality using 
the fastembed library. Consider removing if truly unused or adding pub(crate) 
visibility.
```

---

## Query Examples

| User Query | Expected Results |
|------------|------------------|
| "What code is unused in embeddings?" | FastEmbedder, MLXBridge findings |
| "Are there any circular dependencies?" | Cycle detection results |
| "Find duplicate function names" | Duplicate export clusters |
| "What did the last scan find?" | Summary of all findings |
| "Security issues in storage module" | Filtered SARIF results |

---

## Implementation Phases

### Phase 1: Basic Integration (MVP)
- [ ] Add `loctree_index` tool to rmcp_memex
- [ ] Parse `analysis.json` and `report.sarif`
- [ ] Store findings with namespace `loctree:{project_id}`
- [ ] Query via existing `rag_search` tool

### Phase 2: Enhanced Search
- [ ] Add `loctree_search` with finding_type filter
- [ ] Implement project_id scoping
- [ ] Add git commit tracking for versioned queries

### Phase 3: Real-time Integration
- [ ] Watch mode: auto-index on `loct scan`
- [ ] Incremental updates (diff-based indexing)
- [ ] Integration with loctree's `--json` output mode

### Phase 4: Advanced Features
- [ ] Cross-project search (find similar patterns)
- [ ] Trend analysis (track findings over commits)
- [ ] AI-generated fix suggestions based on findings

---

## Technical Considerations

### Dependencies
- rmcp_memex already has: LanceDB, fastembed, serde_json
- No new deps needed for basic integration

### Performance
- Typical loctree report: 50-100KB JSON → ~50-200 chunks
- Embedding time: <5 seconds per report
- Query latency: <100ms (LanceDB ANN search)

### Storage
- ~1MB per project in LanceDB (embeddings + metadata)
- Retention policy: keep last N commits or time-based

---

## Benefits for Loctree

1. **AI Agent Ecosystem**: rmcp_memex is MCP-compatible → works with Claude, Cursor, etc.
2. **Semantic Queries**: Beyond regex, natural language understanding
3. **Memory Persistence**: Findings available across AI sessions
4. **Low Integration Effort**: Just output JSON, rmcp_memex handles the rest

---

## Next Steps

1. **Review** this proposal with Loctree team
2. **Prototype** `loctree_index` with existing analysis.json
3. **Define** exact JSON schema contract
4. **Implement** Phase 1 MVP
5. **Test** with real-world projects

---

## Appendix: Sample Data Structures

### analysis.json (relevant sections)

```json
{
  "analysis": [{
    "aiViews": {
      "deadSymbols": [
        {
          "name": "FastEmbedder",
          "paths": ["src/embeddings/mod.rs"],
          "publicSurface": true
        }
      ],
      "ciSummary": {
        "duplicateClustersCount": 3,
        "topClusters": [
          {"symbolName": "new", "size": 3, "severity": "medium"}
        ]
      }
    }
  }]
}
```

### report.sarif (relevant sections)

```json
{
  "runs": [{
    "results": [
      {
        "ruleId": "duplicate-export",
        "level": "warning",
        "message": {"text": "Duplicate export 'storage'"},
        "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/lib.rs"}}}]
      }
    ]
  }]
}
```

---

## Contact

For questions or collaboration:
- Repository: https://github.com/Loctree/rmcp_memex
- Branch: `rebranding-and-improvements`
