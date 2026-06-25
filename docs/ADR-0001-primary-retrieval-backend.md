# ADR 0001: Primary retrieval backend

Date: 2026-06-25  
Status: accepted

## Context

VaultLayer is a local-first query layer over Markdown/Obsidian vaults. Users continue editing `.md` files in their existing vault. VaultLayer builds rebuildable runtime projections outside both the repository and the source vault so agents, CLI tools, MCP tools, and viewers can run fast retrieval/data operations without scanning the Markdown tree every time.

The backend choice was revisited after comparing:

- SQLite + FTS5 + sqlite-vec;
- DuckDB;
- local libSQL / open-source Turso-compatible DB;
- hosted Turso/libSQL remote sync.

Hosted Turso is mainly compelling when cloud sync or a shared remote query endpoint is explicitly desired. It is not the right default for private local vault retrieval.

## Decision

Use **SQLite + FTS5** as the default primary local retrieval backend.

Use **sqlite-vec** as the intended native local vector path, gated by reproducible packaging on supported platforms. Until that gate is green, keep deterministic JSON cosine vector search as the portable fallback.

Use **DuckDB** as an explicit analytics/export sidecar, not as the primary retrieval default.

Use **hosted Turso/libSQL** only for explicit remote sync/export commands with explicit credentials.

## Backend roles

| Backend | Role | Invocation | Artifact |
|---|---|---|---|
| SQLite + FTS5 | Primary local retrieval, BM25/search/context/provenance | default or `VAULT_LAYER_BACKEND=sqlite` | `vault-layer.db` |
| sqlite-vec | Native local vector search target | future gated SQLite extension path | inside `vault-layer.db` |
| DuckDB | Analytics/export/reporting sidecar | `VAULT_LAYER_BACKEND=duckdb` | `vault-layer.duckdb` |
| local libSQL | Local Turso-compatible compatibility adapter | `VAULT_LAYER_BACKEND=libsql-local` | `vault-layer.libsql` |
| hosted Turso/libSQL | Explicit cloud sync/export | `sync-turso` or `index --remote-sync` with URL/token | remote DB |

## Consequences

Positive:

- `.md` remains the source of truth.
- The default local path needs no URL, token, SaaS account, or network.
- FTS5 gives a mature retrieval primitive for BM25-style text search.
- SQLite keeps the runtime artifact small, familiar, and easy to inspect.
- DuckDB stays available for the workloads where it is strongest.
- Turso remote sync cannot accidentally upload private vault-derived text.

Tradeoffs:

- SQLite is not as strong as DuckDB for broad analytical scans and columnar export.
- sqlite-vec integration needs a packaging gate before it can be claimed as native production behavior.
- Some vector search behavior currently remains a deterministic JSON cosine fallback.

## Migration notes

This supersedes the short-lived DuckDB-default state from commit `d7b607e`.

Default behavior now resolves to:

```bash
vault-layer index /path/to/vault
# backend=sqlite
# db_path=.../vault-layer.db
```

DuckDB is still available explicitly:

```bash
VAULT_LAYER_BACKEND=duckdb vault-layer index /path/to/vault
# backend=duckdb
# db_path=.../vault-layer.duckdb
```

Hosted Turso/libSQL remains explicit and credential-gated:

```bash
TURSO_DATABASE_URL=libsql://your-database.turso.io \
TURSO_AUTH_TOKEN=*** \
vault-layer sync-turso /path/to/vault
```

No runtime DBs are source-of-truth. If backend defaults change, rebuild the projection from Markdown instead of migrating private generated DB files.

## Verification

This ADR is backed by:

- `docs/backend-decision-benchmark.md`;
- `scripts/wsl-smoke.sh` bounded WSL smoke evidence;
- `make check` repository guard that rejects tracked `.db`, `.sqlite`, `.sqlite3`, `.libsql`, `.duckdb`, `.turso`, `.parquet`, and `.arrow` files.
