# Retrieval benchmark: SQLite + FTS5 + sqlite-vec smoke

Date: 2026-06-25  
Task: `20260625-112922-full-vault-retrieval-benchmark`

## Summary

A test-vault benchmark and a bounded real-vault retrieval benchmark passed on the current default backend:

- backend: SQLite + FTS5 default (`vault-layer.db`)
- native sqlite-vec smoke: available (`v0.1.9`)
- production vector path: deterministic JSON cosine fallback
- test vault: `/mnt/c/Users/viggo/github/obsidian-test-vault`
- bounded real input vault: `/mnt/c/Users/Viggo/Syncthing/vault`
- runtime state: `/tmp/vault-layer-bounded-retrieval-benchmark`
- generated DB: outside repo and outside vault
- repo runtime artifacts tracked: `0`

A full-vault run was attempted first and stopped after ~21 minutes with no completed index output. This is recorded as a blocker for unattended full-vault indexing on WSL `/mnt/c`; bounded evidence is valid, but full-vault indexing needs incremental/progress/resume work before being treated as a reliable unattended gate.


## Test-vault evidence

After correcting the verification order, a test-vault benchmark was run before any additional full-vault work.

Input:

```text
/mnt/c/Users/viggo/github/obsidian-test-vault
markdown_files=19
```

Observed output:

```text
started=2026-06-25T14:39:01+02:00
backend=sqlite
notes_indexed=13
db_path=/tmp/vault-layer-test-vault-benchmark/vault_27fc37ef4acdd717/vault-layer.db
index_elapsed=0:00.49 index_maxrss=6400KB
db_size_bytes=94208
notes|sections|fts|embeddings_before|avg_human_relevance = 13|15|15|0|0.5
sqlite_vec_available=true
sqlite_vec_version=v0.1.9
sqlite_vec_dimensions=3
sqlite_vec_distance=0
embed_elapsed=0:00.01 embed_maxrss=6400KB
embeddings_after=15
vector_elapsed=0:00.00 vector_maxrss=6400KB
repo_artifacts=0
finished=2026-06-25T14:39:02+02:00
```

Search/vector samples retained provenance fields (`chunk_id`, `path`, `heading_path`, `excerpt`, `score`, `content_hash`, `modified_unix`, `human_relevance_score`). Runtime DB was under `/tmp`, outside both repo and vault.

## Bounded real-vault evidence

Command shape:

```bash
./target/release/vault-layer index /mnt/c/Users/Viggo/Syncthing/vault \
  --state-dir /tmp/vault-layer-bounded-retrieval-benchmark \
  --limit 1000
```

Observed output:

```text
started=2026-06-25T12:02:31+02:00
backend=sqlite
notes_indexed=1000
db_path=/tmp/vault-layer-bounded-retrieval-benchmark/vault_b9ffda2be595d584/vault-layer.db
index_elapsed=0:12.16 index_maxrss=9164KB
db_size_bytes=3137536
notes|sections|fts|embeddings_before|avg_human_relevance = 1000|3006|3006|0|0.5
sqlite_vec_available=true
sqlite_vec_version=v0.1.9
sqlite_vec_dimensions=3
sqlite_vec_distance=0
```

Search sample:

```json
[
  {
    "chunk_id": "chunk_8036771a435ebba5",
    "path": "10_notes/1994-03/19940314-0000-week-11.md",
    "heading_path": "Week 11 · 14 maart 1994 – 20 maart 1994",
    "excerpt": "**Context:** 1994 · maart",
    "score": 0.9957072730112172,
    "content_hash": "646a871f9d739437",
    "modified_unix": 1776512004,
    "human_relevance_score": 0.5
  }
]
```

Embedding/vector evidence:

```text
embed_elapsed=0:00.08 embed_maxrss=7868KB
embeddings_after=3006
vector_elapsed=0:00.03 vector_maxrss=9856KB
```

Vector sample retained provenance fields:

```json
[
  {
    "chunk_id": "chunk_10ab5b30b40f9bfb",
    "path": "10_notes/2000-07/20000701-1200-neuromancer-william-gibson.md",
    "heading_path": "Neuromancer - William Gibson",
    "excerpt": "2024_02_11_1036__Oud 2018 books to read lijstje",
    "score": 0.958702,
    "content_hash": "5fac0bc060b86889",
    "modified_unix": 1771616235,
    "human_relevance_score": 0.5
  }
]
```

## Full-vault attempt

Full-vault command started at `2026-06-25T11:40:55+02:00`:

```bash
./target/release/vault-layer index /mnt/c/Users/Viggo/Syncthing/vault \
  --state-dir /home/viggo/.local/share/vault-layer-benchmark-sqlite-vec-full
```

It remained in the index phase for ~21 minutes with no DB materialized and was killed. At inspection:

```text
PID 508850 D 21:10 RSS=319232KB ./target/release/vault-layer index ...
state dir size: 4.0K
log contained only started/vault/state lines
```

This does **not** invalidate the SQLite+FTS5 default; previous full-vault SQLite indexing completed before this task. It does mean the full-vault gate should not be a blind foreground gate until VaultLayer has progress output, incremental indexing, and/or resume support.

## Follow-up recommendation

Create a follow-up implementation task for full-vault production indexing hardening:

1. progress logging during scan and write phases;
2. incremental/resumable indexing;
3. streaming/low-memory embedding writes;
4. full-vault benchmark rerun after progress/resume exists.
