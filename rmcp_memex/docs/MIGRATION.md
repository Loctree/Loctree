# Schema Migration Guide

This document describes how to handle schema changes in rmcp_memex storage layers.

## Schema Versioning

The current schema version is defined in `src/storage/mod.rs`:

```rust
pub const SCHEMA_VERSION: u32 = 1;
```

Increment this version whenever you make breaking changes to:
- LanceDB table structure (columns, embeddings dimension)
- Sled key-value schema
- ChromaDocument fields

## Current Schema (v1)

### LanceDB Table: `mcp_documents`

| Column | Type | Description |
|--------|------|-------------|
| `id` | String | Unique document identifier |
| `namespace` | String | Namespace for isolation |
| `embedding` | FixedSizeList[Float32, 384] | fastembed vector (384 dims) |
| `metadata` | String (JSON) | Serialized metadata object |
| `document` | String | Original text content |

### Sled Keys

| Key Pattern | Value | Description |
|-------------|-------|-------------|
| `{namespace}:{id}` | JSON bytes | Document lookup cache |

## Migration Procedures

### Before Any Migration

1. **Backup your data**:
   ```bash
   cp -r ~/.rmcp_servers/rmcp_memex/lancedb ~/.rmcp_servers/rmcp_memex/lancedb.backup
   cp -r ~/.rmcp_servers/sled ~/.rmcp_servers/sled.backup
   ```

2. **Stop all rmcp_memex instances**

### Migration Strategies

#### Strategy A: Re-index (Recommended for small datasets)

Best for datasets < 100K documents or when embeddings model changes.

```bash
# 1. Backup
cp -r ~/.rmcp_servers/rmcp_memex/lancedb ~/.rmcp_servers/rmcp_memex/lancedb.backup

# 2. Delete old data
rm -rf ~/.rmcp_servers/rmcp_memex/lancedb

# 3. Re-index your documents
# (Use your indexing scripts or MCP tools)
```

#### Strategy B: In-place Migration (Advanced)

For large datasets where re-indexing is expensive.

```rust
// Example migration script (pseudocode)
use rmcp_memex::storage::StorageManager;

async fn migrate_v1_to_v2(storage: &StorageManager) -> Result<()> {
    // 1. Read all documents from old schema
    // 2. Transform to new schema
    // 3. Write to new table
    // 4. Swap tables atomically
    Ok(())
}
```

### Version-Specific Migrations

#### v1 → v2 (Future)

*Not yet defined. This section will be updated when v2 schema is introduced.*

## Adding a Migration

When changing the schema:

1. Increment `SCHEMA_VERSION` in `src/storage/mod.rs`
2. Document the new schema in this file
3. Add migration procedure from previous version
4. Add a test that creates old-schema data and verifies migration
5. Update CHANGELOG.md

## Testing Migrations

```bash
# Run migration tests
cargo test migration

# Manual verification
cargo run -- --db-path /tmp/test_migration_db
```

## Rollback

If migration fails:

```bash
# Restore from backup
rm -rf ~/.rmcp_servers/rmcp_memex/lancedb
cp -r ~/.rmcp_servers/rmcp_memex/lancedb.backup ~/.rmcp_servers/rmcp_memex/lancedb
```

## FAQ

**Q: Do I need to migrate if I only update rmcp_memex?**

A: Only if the SCHEMA_VERSION changes. Check the CHANGELOG for "breaking: schema change" entries.

**Q: Can I run different schema versions side by side?**

A: Not recommended. Use separate `--db-path` for each version during testing.

**Q: How do I check my current schema version?**

A: Currently there's no runtime check. The schema version is implied by the rmcp_memex version you're running. Future versions may store schema version in the database.
