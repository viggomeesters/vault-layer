# Embeddings and Vector Storage

VaultLayer has an embedding provider boundary from the start.

## MVP provider

`deterministic-v0` is an offline deterministic provider for tests and smoke runs. It is **not** semantically useful; it proves storage, query, provenance, and no-external-data-leak behavior.

```bash
vault-layer embed --db <db>
vault-layer vector-search "query" --db <db> --json
```

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
