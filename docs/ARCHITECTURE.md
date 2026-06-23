# Architecture

# VaultLayer Architecture

## Principle

The Markdown vault remains the source of truth. VaultLayer builds a rebuildable shadow database for fast retrieval, agent context, and viewer read models.

## Layers

```text
Markdown/Obsidian vault
  -> parser/chunker/link extractor
  -> local shadow database
  -> hybrid retrieval engine
  -> CLI / MCP / HTTP / viewer adapter
```

## Runtime data location

Index data is stored outside the Git repository. Default Linux/WSL location:

```text
~/.local/share/vault-layer/<vault-id>/
```

A future config may support custom paths, but the repo itself must stay free of private data and generated DB files.

## Retrieval model

Hybrid retrieval combines:

1. exact path/title lookup
2. FTS/BM25
3. WikiLink/backlink graph
4. metadata filters
5. vector similarity through SQLite/libSQL/Turso-compatible vector columns
6. reranking and citation packing

## Initial schemas

Planned core tables:

- `vaults`
- `notes`
- `sections`
- `chunks`
- `links`
- `frontmatter`
- `tags`
- `embeddings`
- `index_runs`
- `provenance`

## Safety

- Read-only vault scan by default.
- No writeback in MVP.
- Every result includes path, heading, chunk id, content hash, and excerpt.
- Private sample vault data is forbidden in the repo.

## CLI skeleton

The first CLI surface reserves `init`, `index`, `search`, `context`, and `serve`. `init` already reports the external runtime state directory and keeps writeback disabled. Later tasks fill the scanner, store, retrieval, vector, and MCP behavior behind this command surface.

## Scanner records

The scanner produces public-safe records rather than storing private fixture content in the repo:

- `NoteRecord`: path, title, modified timestamp, content hash, frontmatter pairs.
- `SectionRecord`: deterministic chunk id, heading path, level, text, content hash.
- `LinkRecord`: source note id, WikiLink target, raw link text.

Stable IDs are derived from vault id, relative path, heading, and content hash so indexes can be rebuilt and stale embeddings detected.

## SQLite/libSQL shadow store

The first store writes a real SQLite database through the system `sqlite3` CLI to keep the Rust MVP dependency-light. The schema is embedded from `crates/vault-layer-core/src/schema.sql` and remains libSQL/Turso-compatible where possible. Runtime DB files are written under the resolved state directory, never under the repository and never under the source vault.

## Viewer adapter

Mega Vault Viewer consumes VaultLayer read models. It should not own separate parsing/indexing logic beyond UI-local cache. See `docs/viewer-adapter.md`.
