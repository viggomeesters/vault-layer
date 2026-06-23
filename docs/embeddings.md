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
