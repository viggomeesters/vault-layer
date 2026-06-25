# VaultLayer CLI/API Contract

Retrieval commands emit JSON from local runtime projections. Every result must be citable by agents.

## Commands

```bash
vault-layer index /path/to/vault --state-dir ~/.local/share/vault-layer
vault-layer backend-info
vault-layer search "query" --db ~/.local/share/vault-layer/<vault-id>/vault-layer.db --json
vault-layer get-note "path/or/id" --db <db> --json
vault-layer related "path/or/id" --db <db> --json
vault-layer context "query" --db <db> --json
vault-layer embed --db <db>
vault-layer vector-search "query" --db <db> --json
vault-layer sqlite-vec-info
```

## Backend contract

`backend-info` reports the active backend and capability mode:

- default: `backend=sqlite`, `index_write_mode=implemented-primary-local-retrieval`, `vector_mode=sqlite-vec-target-json-cosine-fallback`;
- with `VAULT_LAYER_BACKEND=duckdb`: `backend=duckdb`, `index_write_mode=implemented-analytics-sidecar`, `vector_mode=duckdb-analytics-portable-json-cosine`;
- with `VAULT_LAYER_BACKEND=libsql-local`: `backend=libsql-local`, `database_url_configured=false`, `auth_token_configured=false`, `index_write_mode=implemented-local-open-source-libsql`;
- with `TURSO_DATABASE_URL` / `VAULT_LAYER_BACKEND=turso-remote`: `backend=turso-libsql`, `vector_mode=native-libsql-vector-target`, `remote_sync=implemented-explicit`.

That split is intentional. Local vault indexing writes SQLite + FTS5 by default (`vault-layer.db`). DuckDB remains available as an explicit analytics/export sidecar (`vault-layer.duckdb`). Hosted Turso/libSQL remote sync is implemented through the libSQL HTTPS pipeline, but it only runs when explicitly invoked with `sync-turso` or `index --remote-sync`.

## Search result shape

```json
[
  {
    "chunk_id": "chunk_...",
    "path": "Projects/example.md",
    "heading_path": "Decision",
    "excerpt": "bounded text",
    "score": 1.23,
    "content_hash": "...",
    "modified_unix": 1234567890,
    "human_relevance_score": 0.8
  }
]
```

The tuple `(path, heading_path, chunk_id, content_hash, modified_unix)` is the provenance contract.

## SQLite + FTS5 primary retrieval

```bash
vault-layer index /path/to/vault
vault-layer search "query" --db ~/.local/share/vault-layer/<vault_id>/vault-layer.db --json
```

SQLite + FTS5 is the recommended local retrieval backend. `search` and `context` use the `sections_fts` table for BM25-ranked chunk retrieval on `.db` files.

## Vector search

```bash
vault-layer embed --db ~/.local/share/vault-layer/<vault_id>/vault-layer.db
vault-layer vector-search "query" --db ~/.local/share/vault-layer/<vault_id>/vault-layer.db --json
```

The current retrieval vector path stores deterministic JSON embeddings and ranks with cosine similarity in the CLI. Native sqlite-vec is now smoke-tested through the scoped `vault-layer-sqlite-vec` adapter and surfaced by `sqlite-vec-info` / `backend-info`, but production `embed` + `vector-search` still use the JSON cosine fallback until sqlite-vec table writes are wired into the indexed vault DB.

## Human relevance score

Every note/section carries `human_relevance_score` in `[0.0, 1.0]` so UI and agent surfaces can separate human-facing knowledge from system/agent plumbing.

Current defaults:

- explicit frontmatter `human_relevance_score`, `human_relevance`, or `human_score` wins and is clamped to `[0.0, 1.0]`;
- `audience: human` => `0.9`;
- `audience: system` or `system_only: true` => `0.1`;
- paths under `system/` => `0.25`;
- otherwise neutral `0.5`.

## DuckDB analytics sidecar

```bash
VAULT_LAYER_BACKEND=duckdb vault-layer index /path/to/vault
vault-layer search "query" --db ~/.local/share/vault-layer/<vault_id>/vault-layer.duckdb --json
```

DuckDB is explicit, not default. Use it for analytics, aggregations, reporting, and future export workflows where a columnar projection is valuable. The Markdown vault stays read-only and source-of-truth.

## Local libSQL / open-source Turso DB

```bash
VAULT_LAYER_BACKEND=libsql-local vault-layer index /path/to/vault
```

This uses embedded `libsql::Builder::new_local(...)`, writes `vault-layer.libsql` under the external state directory, and requires no `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN`, SaaS account, or network.

## Turso/libSQL remote sync

```bash
TURSO_DATABASE_URL=libsql://your-database.turso.io \
TURSO_AUTH_TOKEN=*** \
vault-layer sync-turso /path/to/vault --limit 100
```

`sync-turso` scans the read-only vault, converts the SQLite-compatible schema and rows into libSQL `/v2/pipeline` execute requests, and sends them in batches over HTTPS. `vault-layer index <vault> --remote-sync` routes to the same implementation when remote sync is explicitly configured.
