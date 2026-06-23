# Agent Instructions — VaultLayer

VaultLayer is a public open-source project for indexing Markdown/Obsidian vaults into a rebuildable local-first query layer.

## Hard rules

- Do not put private vault content, generated indexes, embeddings, database files, caches, or sample personal notes in this repository.
- Runtime data must live outside the repository, e.g. `~/.local/share/vault-layer/`.
- Treat vault input paths as read-only unless a task explicitly enables writeback.
- Every retrieval result must preserve provenance: source path, heading/chunk id, content hash, modified timestamp, and excerpt.
- Prefer small vertical slices with tests and `make check`.

## Target architecture

- Rust workspace preferred for core/indexer/CLI.
- SQLite/libSQL/Turso are storage backends, not product identity.
- CLI and MCP are first-class consumers.
- Mega Vault Viewer should consume VaultLayer instead of owning indexing logic.

## Verification

Run before finishing repo work:

```bash
make check
```
