# Embeddings and Vector Storage

VaultLayer has an embedding provider boundary from the start.

## MVP provider

`deterministic-v0` is an offline deterministic provider for tests and smoke runs. It is **not** semantically useful; it proves storage, query, provenance, and no-external-data-leak behavior.

```bash
vault-layer embed --db <db>
vault-layer vector-search "query" --db <db> --json
```

The `embeddings` table is keyed by `(chunk_id, model)` and records `model` plus `dimensions` per row, so deterministic smoke vectors can coexist with future real local model vectors for the same chunk.

## Real local model provider

`fastembed-mini-lm` is the first real local model adapter. It uses Python `fastembed` + ONNX Runtime through `scripts/fastembed_embed.py`, keeping Rust dependency resolution compatible with the current Cargo 1.75 toolchain.

```bash
python3 -m pip install fastembed==0.7.3
vault-layer embed --db <db> --model fastembed-mini-lm
vault-layer vector-search "query" --db <db> --model fastembed-mini-lm --json
vault-layer hybrid-search "query" --db <db> --model fastembed-mini-lm --json
```

Model identity: `fastembed:sentence-transformers/all-MiniLM-L6-v2`
Dimensions: `384`
Default cache: `~/.local/share/vault-layer/models/fastembed/`

No SaaS URL/token is required. First use may download the ONNX model into the cache; cached runs are local/offline. See [`local-embedding-adapter.md`](local-embedding-adapter.md) for test-vault and bounded 5000-note evidence.

## libSQL/Turso target shape

The public schema currently stores deterministic vectors as JSON for portable SQLite tests. The intended libSQL/Turso shape is:

```sql
ALTER TABLE embeddings ADD COLUMN embedding F32_BLOB(1536);
CREATE INDEX chunk_embedding_idx ON embeddings (libsql_vector_idx(embedding, 'metric=cosine'));
SELECT * FROM vector_top_k('chunk_embedding_idx', vector32(?), 20);
```

If native vector functions are unavailable in the local SQLite runtime, VaultLayer keeps the interface and records this as a backend capability gap rather than faking native vector support.


## Capability modes

Run:

```bash
vault-layer backend-info
```

Expected local mode:

```text
backend=sqlite
index_write_mode=implemented
vector_mode=portable-json-cosine
remote_sync=not-configured
```

Expected configured Turso/libSQL target mode:

```text
backend=turso-libsql
database_url_configured=true
auth_token_configured=true
index_write_mode=implemented-explicit-remote-sync
vector_mode=native-libsql-vector-target
remote_sync=implemented-explicit
```

This avoids the dangerous middle state where private vault text is silently sent
to a remote database just because an environment variable exists.


## DuckDB local mode

DuckDB is the preferred local backend for embedding metadata and future vector
search because it is analytics-first and already has FTS/VSS extension paths.
The current implementation keeps portable JSON vectors for deterministic offline
smoke tests; native DuckDB VSS is the next hardening step.

## Local libSQL mode

`VAULT_LAYER_BACKEND=libsql-local` stores the same portable JSON vectors in a
local embedded libSQL database. It is the open-source local Turso-compatible path
and requires no URL/token. Hosted Turso can later use native vector columns.


## sqlite-vec target

The selected primary retrieval architecture is SQLite + FTS5 with a sqlite-vec native vector target. `vault-layer sqlite-vec-info` smoke-tests native sqlite-vec through a scoped Rust/rusqlite adapter and reports `sqlite_vec_available=true` when the extension registers successfully. `vault-layer embed` can now populate either deterministic smoke vectors or real local `fastembed-mini-lm` vectors, and sqlite-vec table writes/search are refreshed for the selected model.
