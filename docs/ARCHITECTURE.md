# Architecture

## Principle

The Markdown vault remains the source of truth. VaultLayer builds rebuildable runtime projections for fast retrieval, agent context, analytics, and viewer read models.

## Layers

```text
Markdown/Obsidian vault
  -> parser/chunker/link extractor
  -> local retrieval database (SQLite + FTS5)
  -> optional analytics sidecar (DuckDB)
  -> CLI / MCP / HTTP / viewer adapter
```

## Runtime data location

Index data is stored outside the Git repository and outside the source vault. Default Linux/WSL location:

```text
~/.local/share/vault-layer/<vault-id>/
```

Generated files such as `.db`, `.duckdb`, `.libsql`, `.parquet`, `.arrow`, caches, and embeddings must never be committed.

## Retrieval model

Hybrid retrieval combines:

1. exact path/title lookup;
2. SQLite FTS5 BM25 search over sections/chunks;
3. WikiLink/backlink graph;
4. metadata filters;
5. sqlite-vec native vector search when packaging is proven, with deterministic JSON cosine fallback;
6. reranking and citation packing.

Embedding rows are keyed by `(chunk_id, model)` and record dimensions per model, so future local semantic embeddings can be compared against `deterministic-v0` without overwriting smoke vectors.

## Backend split

VaultLayer is not identified with one storage vendor. Backends are adapters over the same Markdown source and provenance contract:

| Backend | Role | Default? | Runtime artifact |
|---|---|---:|---|
| SQLite + FTS5 | Primary local retrieval store | yes | `vault-layer.db` |
| DuckDB | Analytics/export/reporting sidecar | no, explicit | `vault-layer.duckdb` |
| local libSQL | Open-source Turso-compatible local adapter | no, explicit | `vault-layer.libsql` |
| hosted Turso/libSQL | Explicit cloud sync/export target | no, explicit credentials + command | remote database |

### SQLite + FTS5 primary retrieval store

The primary store writes a real SQLite database through the system `sqlite3` CLI. It creates relational tables plus `sections_fts` for BM25-ranked full-text search over chunks. Runtime DB files are written under the resolved state directory, never under the repository and never under the source vault.

The sqlite-vec native vector path is the intended next local vector implementation. Deterministic JSON cosine remains a fallback until sqlite-vec packaging is proven across WSL/macOS/Windows and the Rust MSRV.

### DuckDB analytics sidecar

DuckDB remains available via explicit backend selection:

```bash
VAULT_LAYER_BACKEND=duckdb vault-layer index /path/to/vault
```

Use it for analytics, report/export queries, aggregations, and Parquet/Arrow-oriented workflows. It must not become the default retrieval path by accident. Search still works on `vault-layer.duckdb`, but SQLite owns the primary retrieval UX.

### Turso/libSQL

Hosted Turso/libSQL sync only runs through explicit `sync-turso` or `index --remote-sync` with `TURSO_DATABASE_URL` and `TURSO_AUTH_TOKEN`. Private vault-derived text is never uploaded implicitly.

## Core tables

Planned/implemented core tables:

- `vaults`
- `notes`
- `sections`
- `links`
- `frontmatter`
- `tags`
- `embeddings`
- `index_runs`
- `provenance`
- `sections_fts` in SQLite

## Human relevance

`notes`, `sections`, and `provenance` include `human_relevance_score` so consumers can choose between agent/system context and human-facing views. The score is source-derived from explicit frontmatter when present; otherwise conservative defaults keep system/agent plumbing lower than normal notes.

## Safety

- Read-only vault scan by default.
- No writeback in MVP.
- Every result includes path, heading, chunk id, content hash, modified timestamp, and excerpt.
- Private sample vault data is forbidden in the repo.
- Runtime/generated index artifacts are forbidden in the repo.

## CLI and backend contract

The CLI surface includes `init`, `index`, `search`, `context`, `get-note`, `related`, `embed`, `vector-search`, `serve`, `backend-info`, and `sync-turso`.

`backend-info` reports backend, index write mode, vector mode, local indexing, and remote sync state so agents can make safe routing decisions.

## Scanner records

The scanner produces public-safe records rather than storing private fixture content in the repo:

- `NoteRecord`: path, title, modified timestamp, content hash, frontmatter pairs.
- `SectionRecord`: deterministic chunk id, heading path, level, text, content hash.
- `LinkRecord`: source note id, WikiLink target, raw link text.

Stable IDs are derived from vault id, relative path, heading, ordinal, and content hash so indexes can be rebuilt and stale embeddings detected.

## Viewer adapter

Mega Vault Viewer consumes VaultLayer read models. It should not own separate parsing/indexing logic beyond UI-local cache. See `docs/viewer-adapter.md`.
