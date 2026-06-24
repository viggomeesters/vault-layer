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

## Human relevance

`notes`, `sections`, and `provenance` include `human_relevance_score` so consumers
can choose between agent/system context and human-facing views. The score is
source-derived from explicit frontmatter when present, otherwise conservative
defaults keep system/agent plumbing lower than normal notes.

## Safety

- Read-only vault scan by default.
- No writeback in MVP.
- Every result includes path, heading, chunk id, content hash, and excerpt.
- Private sample vault data is forbidden in the repo.

## CLI and backend contract

The first CLI surface reserves `init`, `index`, `search`, `context`, `serve`,
and `backend-info`. `init` reports the external runtime state directory,
writeback state, backend, index write mode, and vector mode.

Backends are explicit:

- `sqlite`: implemented local shadow DB writes and portable JSON-vector smoke search.
- `turso-libsql`: configured target when `TURSO_DATABASE_URL` is set. It reports
  native libSQL vector target mode, but index writes are blocked until an
  explicit remote-sync command exists.

This keeps WSL/private-vault indexing local and safe while preserving the Turso
schema target.

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


## DuckDB projection store

DuckDB is the recommended local backend. It is used as a rebuildable projection
over Markdown: notes, sections, frontmatter, tags, links, provenance, index runs,
and embeddings live outside the vault under the state directory. This matches the
core product shape: users keep editing `.md`; VaultLayer provides fast local
operations and retrieval over the derived database.
