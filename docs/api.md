# VaultLayer CLI/API Contract

Retrieval commands emit JSON from the local shadow database. Every result must be citable by agents.

## Commands

```bash
vault-layer index /path/to/vault --state-dir ~/.local/share/vault-layer
vault-layer backend-info
vault-layer search "query" --db ~/.local/share/vault-layer/<vault-id>/vault-layer.db --json
vault-layer get-note "path/or/id" --db <db> --json
vault-layer related "path/or/id" --db <db> --json
vault-layer context "query" --db <db> --json
```

## Backend contract

`backend-info` reports the active backend and capability mode:

- default: `backend=sqlite`, `index_write_mode=implemented`, `vector_mode=portable-json-cosine`;
- with `TURSO_DATABASE_URL`: `backend=turso-libsql`, `vector_mode=native-libsql-vector-target`, `remote_sync=false`.

That split is intentional. Local vault indexing writes a real SQLite shadow DB
today. Turso/libSQL is a configured target shape for future remote sync; it is
not used for index writes until a separate explicit sync command exists.

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


## Human relevance score

Every note/section carries `human_relevance_score` in `[0.0, 1.0]` so UI and
agent surfaces can separate human-facing knowledge from system/agent plumbing.

Current defaults:

- explicit frontmatter `human_relevance_score`, `human_relevance`, or `human_score` wins and is clamped to `[0.0, 1.0]`;
- `audience: human` => `0.9`;
- `audience: system` or `system_only: true` => `0.1`;
- paths under `system/` => `0.25`;
- otherwise neutral `0.5`.
