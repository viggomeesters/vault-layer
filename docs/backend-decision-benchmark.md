# Backend decision benchmark

Date: 2026-06-25  
Task: `20260625-075856-backend-decision-benchmark`

## Decision

VaultLayer's primary local retrieval backend should be **SQLite + FTS5**, with a sqlite-vec target tracked behind an explicit packaging gate. DuckDB should remain an optional analytics/export sidecar. Turso/libSQL remote remains a cloud-sync/export target, not the local product identity.

## Why

VaultLayer is a rebuildable projection over Markdown/Obsidian vaults:

- `.md` files remain the source of truth.
- Runtime databases live outside the repo and outside the vault.
- The hot path is agent retrieval: text search, context packing, provenance, and eventually vector search.
- Analytics is valuable but secondary.

SQLite + FTS5 is the best primary fit because it is embedded, mature, already available through the system `sqlite3` CLI, supports BM25-style full-text search, and works naturally as a single local `.db` runtime artifact. sqlite-vec is the right vector direction because it is designed to add local vector search to SQLite without a server.

DuckDB remains useful for OLAP, scans, reports, Parquet/Arrow export, and future analytics materialization. DuckDB VSS is promising but currently less conservative as the primary retrieval persistence path for a public cross-platform vault layer.

## Evidence gathered

### Current implementation state

| Backend | Current state | Notes |
|---|---|---|
| DuckDB | implemented | Recently made default; works for local projection and LIKE search. |
| SQLite + FTS5 | implemented | `write_scan_sqlite` creates `sections_fts`; previous full vault run indexed 78,562 notes / 143,353 sections in 14:26 with 458 MB RSS. |
| sqlite-vec | not integrated yet | Crate available as `sqlite-vec = 0.1.10-alpha.4`; needs packaging smoke before becoming a hard required dependency. |
| local libSQL | implemented | Useful compatibility path, but no decisive local retrieval win over SQLite. |
| Turso remote | implemented explicit sync | Useful only when cloud upload/sync is intentional; requires URL/token. |

### Bounded WSL measurement

Command shape: release `vault-layer index /mnt/c/Users/Viggo/Syncthing/vault --limit 1000` with runtime state under `/tmp`.

| Backend | Notes | Sections | FTS rows | Elapsed | Max RSS | DB size |
|---|---:|---:|---:|---:|---:|---:|
| SQLite + FTS5 | 1,000 | 3,006 | 3,006 | 0:22.32 | 8.8 MB | 3.0 MB |
| DuckDB | 1,000 | not queried in this quick run | n/a | 0:25.59 | 41.5 MB | 4.8 MB |

The bounded result does not prove full-vault superiority by itself, but it removes the concern that SQLite+FTS5 is obviously slower/heavier. Combined with the previous full-vault SQLite success (`78,562 notes / 143,353 sections`, `14:26`, `458 MB RSS`), SQLite is a safe primary retrieval baseline.

### Benchmark dimensions for execution tasks

Execution tasks must record:

- index elapsed time;
- max RSS;
- database size;
- notes/sections/embeddings counts;
- text-search latency;
- vector-search latency/path;
- Rust/MSRV and packaging friction;
- rebuildability and provenance correctness.

### Acceptance thresholds

| Path | Keep if | Kill/defer if |
|---|---|---|
| SQLite + FTS5 primary | indexes full vault reliably, search stays fast, provenance intact, no private artifacts tracked | FTS query performance or packaging fails on supported machines |
| sqlite-vec vector path | can be loaded/linked reproducibly on WSL/macOS/Windows with Rust 1.75-compatible build | requires fragile system install, unsupported binaries, or breaks public repo checks |
| DuckDB sidecar | adds analytics/export/report value without owning retrieval UX | forces complex query split or becomes default by accident |
| Turso remote | explicit cloud-sync/export use case exists | any implicit upload of private vault-derived text is required |

## Decision output

1. Set `VAULT_LAYER_BACKEND=sqlite` / SQLite+FTS5 as the recommended primary retrieval default.
2. Keep DuckDB behind an explicit analytics/export backend name.
3. Keep Turso/libSQL remote behind explicit `--remote-sync` and credentials.
4. Add gates that prove the above on fixtures and bounded/full WSL vault smokes.
5. Treat sqlite-vec as the next vector implementation target; if packaging blocks, keep deterministic JSON cosine as fallback and document the blocker.
