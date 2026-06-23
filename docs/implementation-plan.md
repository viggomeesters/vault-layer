# VaultLayer MVP Implementation Plan

This public repo intentionally starts with a thin scaffold. The durable execution plan lives in Viggo's Agent Workflow Lite vault state.

MVP target:

1. Rust workspace + CLI skeleton.
2. Runtime data directory outside repo.
3. Markdown scanner with incremental content hashes.
4. SQLite/libSQL schema for notes, chunks, links, and provenance.
5. FTS search.
6. Embedding provider abstraction and vector storage spike.
7. MCP server with cited `vault_search`, `vault_get_note`, and `vault_related` tools.
