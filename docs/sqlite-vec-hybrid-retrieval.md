# sqlite-vec hybrid retrieval

Date: 2026-06-25  
Task: `20260625-194000-vault-layer-sqlite-vec-hybrid-ranking`

## What changed

VaultLayer now goes beyond sqlite-vec availability smoke:

- `vault-layer embed` still writes portable JSON embeddings, and now also refreshes a native sqlite-vec `vec0` table when sqlite-vec is available.
- `vault-layer vector-search` prefers native sqlite-vec KNN search and falls back to JSON cosine only if native search is unavailable.
- `vault-layer hybrid-search` runs SQLite FTS candidate retrieval and reranks with vector score, human relevance, and text-quality scoring.

The embedding model remains explicitly `deterministic-v0`. This is a plumbing/runtime milestone, not a claim of production semantic quality.

## Native sqlite-vec storage

`embed` now reports native row materialization:

```json
{
  "model": "deterministic-v0",
  "dimensions": 8,
  "chunks_embedded": 8304,
  "sqlite_vec_rows": 8304,
  "vector_runtime": "native-sqlite-vec+json-fallback"
}
```

Native sqlite-vec artifacts are stored inside the runtime DB, outside repo and vault:

- `vec_embedding_map(rowid, chunk_id, model)`
- `vec_embeddings` virtual table using `vec0(embedding float[8])`

## Query outputs

`vector-search` rows include:

- `score`
- `vector_score`
- `vector_distance`
- `text_quality_score`
- `vector_runtime`
- provenance fields: `chunk_id`, `path`, `heading_path`, `excerpt`, `content_hash`, `modified_unix`, `human_relevance_score`

`hybrid-search` rows include:

- `score`
- `fts_score`
- `vector_score`
- `text_quality_score`
- `ranking_runtime=fts-vector-quality`
- the same provenance fields

## Test-vault evidence

```text
vault=/mnt/c/Users/viggo/github/obsidian-test-vault
notes_indexed=13
chunks_embedded=15
sqlite_vec_rows=15
vector_runtime=native-sqlite-vec+json-fallback
vector-search runtime=native-sqlite-vec
hybrid-search rows=3 with fts_score/vector_score/text_quality_score
```

## 5000-note bounded real-vault evidence

```text
vault=/mnt/c/Users/Viggo/Syncthing/vault
limit=5000
notes=5000
sections=8304
fts_rows=8304
embeddings=8304
vec_rows=8304
index_elapsed=2:25.23 index_maxrss=27740KB
embed_elapsed=0:00.39 embed_maxrss=20872KB
vector_elapsed=0:00.17 vector_maxrss=8484KB
hybrid_elapsed=0:00.02 hybrid_maxrss=6400KB
repo_artifacts=0
log=/home/viggo/.local/share/vault-layer/hybrid-real-5000-20260625-203651.log
```

## Current interpretation

This confirms:

- native sqlite-vec table write/search works in the real runtime DB;
- FTS + vector + quality hybrid output works and preserves citations;
- runtime artifacts stay outside repo/vault.

This does **not** confirm:

- high-quality semantic retrieval;
- a real local embedding model;
- full 78k vault scale after native vec table materialization.

## Remaining product layer

Next work should add a real local embedding provider behind a clear model adapter, then re-run the same test-vault and 5000-note gates. Full-vault scale should still wait for progress/resume instrumentation.
